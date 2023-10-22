use std::collections::btree_set::SymmetricDifference;
use std::sync::{Arc, RwLock};

use ascom_alpaca::api::{Camera, CameraState, CargoServerInfo, Device, ImageArray};
use ascom_alpaca::{ASCOMError, ASCOMResult, Server};
use async_trait::async_trait;
use eyre::{eyre, Result};
use libqhyccd_sys::{get_readout_mode, QhyccdHandle};
use tokio::sync::{oneshot, watch};
use tracing::{debug, error, trace};

#[derive(Debug)]
struct StopExposure {
    want_image: bool,
}

#[derive(Debug)]
enum ExposingState {
    Idle,
    Exposing {
        stop_tx: Option<oneshot::Sender<StopExposure>>,
        done_rx: watch::Receiver<bool>,
    },
}

#[derive(Debug)]
struct BinningMode {
    symmetric_value: i32,
}
impl BinningMode {
    fn x(&self) -> i32 {
        self.symmetric_value
    }
    fn y(&self) -> i32 {
        self.symmetric_value
    }
}

#[derive(Debug)]
struct QhyccdCamera {
    handle: libqhyccd_sys::QhyccdHandle,
    binning: BinningMode,
    valid_binning_modes: Vec<BinningMode>,
    roi: Option<libqhyccd_sys::CCDChipArea>,
    valid_readout_modes: Option<Vec<libqhyccd_sys::ReadoutMode>>,
    last_exposure_start_time: Option<std::time::SystemTime>,
    last_exposure_duration: Option<f64>,
    last_image: Option<ImageArray>,
    exposing: ExposingState,
}

#[derive(Debug)]
struct QhyccdAlpaca {
    unique_id: String,
    name: String,
    description: String,
    camera: Arc<RwLock<Option<QhyccdCamera>>>,
}

impl QhyccdAlpaca {
    fn get_readout_modes(
        handle: QhyccdHandle,
    ) -> ASCOMResult<Option<Vec<libqhyccd_sys::ReadoutMode>>> {
        match libqhyccd_sys::get_number_of_readout_modes(handle) {
            Ok(num) => {
                let mut readout_modes = Vec::with_capacity(num as usize);
                for i in 0..num {
                    match libqhyccd_sys::get_readout_mode_name(handle, i) {
                        Ok(readout_mode) => readout_modes.push(libqhyccd_sys::ReadoutMode {
                            id: i,
                            name: readout_mode,
                        }),
                        Err(e) => {
                            error!(?e, "get_readout_mode failed");
                            return Err(ASCOMError::NOT_CONNECTED);
                        }
                    }
                }
                Ok(Some(readout_modes))
            }
            Err(e) => {
                error!(?e, "get_number_of_readout_modes failed");
                Ok(None)
            }
        }
    }

    fn get_valid_binning_modes(handle: QhyccdHandle) -> ASCOMResult<Vec<BinningMode>> {
        let mut valid_binning_modes = Vec::with_capacity(6);
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin1x1mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin2x2mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin3x3mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin4x4mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin6x6mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if libqhyccd_sys::is_feature_supported(handle, libqhyccd_sys::CameraFeature::CamBin8x8mode)
            .is_ok()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        Ok(valid_binning_modes)
    }
}

#[async_trait]
impl Device for QhyccdAlpaca {
    fn static_name(&self) -> &str {
        &self.name
    }

    fn unique_id(&self) -> &str {
        &self.unique_id
    }

    async fn connected(&self) -> ASCOMResult<bool> {
        match self.camera.read() {
            Ok(camera) => Ok(camera.is_some()),
            Err(e) => {
                error!(?e, "camera lock poisoned");
                Err(ASCOMError::UNSPECIFIED)
            }
        }
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        match self.connected().await? == connected {
            true => return Ok(()),
            false => match connected {
                true => {
                    let mut camera_lock = match self.camera.write() {
                        Ok(camera) => camera,
                        Err(e) => {
                            error!(?e, "camera lock poisoned");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };
                    let handle = match libqhyccd_sys::open_camera(self.unique_id.clone()) {
                        Ok(handle) => handle,
                        Err(e) => {
                            error!(?e, "open_camera failed");
                            return Err(ASCOMError::NOT_CONNECTED);
                        }
                    };
                    let readout_modes = QhyccdAlpaca::get_readout_modes(handle)?;
                    let roi = match libqhyccd_sys::get_effective_area(handle) {
                        Ok(area) => Some(area),
                        Err(e) => {
                            error!(?e, "get_effective_area failed");
                            None
                        }
                    };
                    let valid_binning_modes: Vec<BinningMode> =
                        QhyccdAlpaca::get_valid_binning_modes(handle)?;
                    let camera = QhyccdCamera {
                        handle,
                        binning: BinningMode { symmetric_value: 1 },
                        valid_binning_modes,
                        roi,
                        valid_readout_modes: readout_modes,
                        last_exposure_start_time: None,
                        last_exposure_duration: None,
                        last_image: None,
                        exposing: ExposingState::Idle,
                    };
                    *camera_lock = Some(camera);
                    Ok(())
                }
                false => {
                    let mut camera_lock = match self.camera.write() {
                        Ok(camera) => camera,
                        Err(e) => {
                            error!(?e, "camera lock poisoned");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };
                    let camera = camera_lock.take().unwrap();
                    match libqhyccd_sys::close_camera(camera.handle) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            error!(?e, "close_camera failed");
                            Err(ASCOMError::NOT_CONNECTED)
                        }
                    }
                }
            },
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("qhyccd_alpaca driver".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl Camera for QhyccdAlpaca {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        Ok(0)
    }

    async fn sensor_name(&self) -> ASCOMResult<String> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match libqhyccd_sys::get_model(camera.handle) {
            Ok(info) => Ok(info),
            Err(e) => {
                error!(?e, "get_model failed");
                Err(ASCOMError::UNSPECIFIED)
            }
        }
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        Ok(camera.binning.x())
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        if bin_x < 1 {
            return Err(ASCOMError::invalid_value("bin_x must be >= 1"));
        }
        let mut camera_lock = match self.camera.write() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref mut camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match libqhyccd_sys::set_bin_mode(camera.handle, bin_x as u32, bin_x as u32) {
            //only supports symmetric binning
            Ok(_) => {
                camera.binning = BinningMode {
                    symmetric_value: bin_x,
                };
                Ok(())
            }
            Err(e) => {
                error!(?e, "set_bin_mode failed");
                Err(ASCOMError::VALUE_NOT_SET)
            }
        }
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        self.bin_x().await
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        self.set_bin_x(bin_y).await
    }

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match camera.valid_binning_modes.iter().map(|m| m.x()).max() {
            Some(max) => Ok(max),
            None => {
                error!("valid_binning_modes is empty");
                Err(ASCOMError::UNSPECIFIED)
            }
        }
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        self.max_bin_x().await
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match camera.exposing {
            ExposingState::Idle => Ok(CameraState::Idle),
            ExposingState::Exposing { .. } => Ok(CameraState::Exposing),
        }
    }

    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        debug!("electrons_per_adu not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn exposure_max(&self) -> ASCOMResult<f64> {
        debug!("exposure_max not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn exposure_min(&self) -> ASCOMResult<f64> {
        debug!("exposure_min not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn exposure_resolution(&self) -> ASCOMResult<f64> {
        debug!("exposure_resolution not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        debug!("full_well_capacity not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match libqhyccd_sys::is_feature_supported(
            camera.handle,
            libqhyccd_sys::CameraFeature::CamMechanicalShutter,
        ) {
            Ok(_) => Ok(true),
            Err(e) => {
                debug!(?e, "is_feature_supported failed for CamMechanicalShutter");
                Ok(false)
            }
        }
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match camera.last_image.clone() {
            Some(image) => Ok(image),
            None => Err(ASCOMError::VALUE_NOT_SET),
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        let camera_lock = match self.camera.read() {
            Ok(camera) => camera,
            Err(e) => {
                error!(?e, "camera lock poisoned");
                return Err(ASCOMError::UNSPECIFIED);
            }
        };
        let camera = match *camera_lock {
            Some(ref camera) => camera,
            None => return Err(ASCOMError::NOT_CONNECTED),
        };
        match libqhyccd_sys::get_remaining_exposure_us(camera.handle) {
            Ok(remaining) => Ok(remaining == 0),
            Err(e) => {
                error!(?e, "get_remaining_exposure_us failed");
                Err(ASCOMError::UNSPECIFIED)
            }
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    tracing_subscriber::fmt::init();

    let mut server = Server {
        info: CargoServerInfo!(),
        ..Default::default()
    };

    server.listen_addr.set_port(8000);

    let sdk_version = libqhyccd_sys::get_sdk_version().expect("get_sdk_version failed");
    trace!(sdk_version = ?sdk_version);

    libqhyccd_sys::init_sdk().expect("init_sdk failed");

    let number_of_cameras = libqhyccd_sys::scan_qhyccd().expect("scan_qhyccd failed");
    trace!(number_of_cameras = ?number_of_cameras);

    for i in 0..number_of_cameras {
        let unique_id = match libqhyccd_sys::get_camera_id(i) {
            Ok(id) => id,
            Err(e) => {
                error!(?e, "get_camera_id failed");
                continue;
            }
        };

        let camera = QhyccdAlpaca {
            unique_id: unique_id.clone(),
            name: format!("QHYCCD-{}", unique_id),
            description: "QHYCCD camera".to_owned(),
            camera: Arc::new(RwLock::new(None)),
        };
        tracing::debug!(?camera, "Registering webcam");
        server.devices.register(camera);
    }

    server.start().await
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
