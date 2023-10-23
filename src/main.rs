use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use ascom_alpaca::api::{Camera, CameraState, CargoServerInfo, Device, ImageArray, SensorType};
use ascom_alpaca::{ASCOMError, ASCOMResult, Server};
use async_trait::async_trait;
use eyre::Result;
use libqhyccd_sys::{CCDChipArea, QhyccdHandle};
use tokio::sync::{oneshot, watch};
use tokio::task;
use tracing::{debug, error, trace};

#[derive(Debug)]
struct StopExposure {
    want_image: bool,
}

#[derive(Debug)]
enum ExposingState {
    Idle,
    Exposing {
        start: SystemTime,
        expected_duration_us: f64,
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
    last_exposure_start_time: Option<SystemTime>,
    last_exposure_duration_us: Option<f64>,
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
        match &*self.camera.read().await {
            Some(_) => Ok(true),
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        match self.connected().await? == connected {
            true => return Ok(()),
            false => match connected {
                true => {
                    let camera_lock = &mut *self.camera.write().await;
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
                        last_exposure_duration_us: None,
                        last_image: None,
                        exposing: ExposingState::Idle,
                    };
                    *camera_lock = Some(camera);
                    Ok(())
                }
                false => {
                    let mut camera_lock = self.camera.write().await;
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
        match &*self.camera.read().await {
            Some(camera) => match libqhyccd_sys::get_model(camera.handle) {
                Ok(info) => Ok(info),
                Err(e) => {
                    error!(?e, "get_model failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(camera) => Ok(camera.binning.x()),
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        if bin_x < 1 {
            return Err(ASCOMError::invalid_value("bin_x must be >= 1"));
        }
        match &mut *self.camera.write().await {
            Some(camera) => {
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
            None => Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        self.bin_x().await
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        self.set_bin_x(bin_y).await
    }

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.valid_binning_modes.iter().map(|m| m.x()).max() {
                Some(max) => Ok(max),
                None => {
                    error!("valid_binning_modes is empty");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        self.max_bin_x().await
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.exposing {
                ExposingState::Idle => Ok(CameraState::Idle),
                ExposingState::Exposing { .. } => Ok(CameraState::Exposing),
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
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
        match &*self.camera.read().await {
            Some(ref camera) => {
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
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.last_image.clone() {
                Some(image) => Ok(image),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        match &*self.camera.read().await {
            Some(ref camera) => match libqhyccd_sys::get_remaining_exposure_us(camera.handle) {
                Ok(remaining) => Ok(remaining == 0),
                Err(e) => {
                    error!(?e, "get_remaining_exposure_us failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn last_exposure_start_time(&self) -> ASCOMResult<SystemTime> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.last_exposure_start_time {
                Some(time) => Ok(time),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.last_exposure_duration_us {
                Some(duration) => Ok(duration),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn max_adu(&self) -> ASCOMResult<i32> {
        debug!("max_adu not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.width as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn camera_ysize(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.height as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn start_x(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.start_x as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn set_start_x(&self, start_x: i32) -> ASCOMResult {
        if start_x < 0 {
            return Err(ASCOMError::invalid_value("start_x must be >= 0"));
        }
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                let mut roi = match camera.roi {
                    Some(roi) => roi,
                    None => return Err(ASCOMError::VALUE_NOT_SET),
                };

                roi = CCDChipArea {
                    start_x: start_x as u32,
                    ..roi
                };

                match libqhyccd_sys::set_roi(camera.handle, roi) {
                    Ok(_) => {
                        camera.roi = Some(roi);
                        Ok(())
                    }
                    Err(e) => {
                        error!(?e, "set_roi failed");
                        Err(ASCOMError::VALUE_NOT_SET)
                    }
                }
            }
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn start_y(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.start_y as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_start_y(&self, start_y: i32) -> ASCOMResult {
        if start_y < 0 {
            return Err(ASCOMError::invalid_value("start_y must be >= 0"));
        }
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                let mut roi = match camera.roi {
                    Some(roi) => roi,
                    None => return Err(ASCOMError::VALUE_NOT_SET),
                };

                roi = CCDChipArea {
                    start_y: start_y as u32,
                    ..roi
                };

                match libqhyccd_sys::set_roi(camera.handle, roi) {
                    Ok(_) => {
                        camera.roi = Some(roi);
                        Ok(())
                    }
                    Err(e) => {
                        error!(?e, "set_roi failed");
                        Err(ASCOMError::VALUE_NOT_SET)
                    }
                }
            }
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn num_x(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.width as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_num_x(&self, num_x: i32) -> ASCOMResult {
        if num_x < 0 {
            return Err(ASCOMError::invalid_value("num_x must be >= 0"));
        }
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                let mut roi = match camera.roi {
                    Some(roi) => roi,
                    None => return Err(ASCOMError::VALUE_NOT_SET),
                };

                roi = CCDChipArea {
                    width: num_x as u32,
                    ..roi
                };

                match libqhyccd_sys::set_roi(camera.handle, roi) {
                    Ok(_) => {
                        camera.roi = Some(roi);
                        Ok(())
                    }
                    Err(e) => {
                        error!(?e, "set_roi failed");
                        Err(ASCOMError::VALUE_NOT_SET)
                    }
                }
            }
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn num_y(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => camera
                .roi
                .map(|roi| roi.height as i32)
                .ok_or(ASCOMError::VALUE_NOT_SET),
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_num_y(&self, num_y: i32) -> ASCOMResult {
        if num_y < 0 {
            return Err(ASCOMError::invalid_value("num_y must be >= 0"));
        }
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                let mut roi = match camera.roi {
                    Some(roi) => roi,
                    None => return Err(ASCOMError::VALUE_NOT_SET),
                };

                roi = CCDChipArea {
                    height: num_y as u32,
                    ..roi
                };

                match libqhyccd_sys::set_roi(camera.handle, roi) {
                    Ok(_) => {
                        camera.roi = Some(roi);
                        Ok(())
                    }
                    Err(e) => {
                        error!(?e, "set_roi failed");
                        Err(ASCOMError::VALUE_NOT_SET)
                    }
                }
            }
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn percent_completed(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.exposing {
                ExposingState::Idle => Ok(100),
                ExposingState::Exposing {
                    expected_duration_us,
                    ..
                } => match libqhyccd_sys::get_remaining_exposure_us(camera.handle) {
                    Ok(remaining) => Ok(remaining as i32 / expected_duration_us as i32),
                    Err(e) => {
                        error!(?e, "get_remaining_exposure_us failed");
                        Err(ASCOMError::UNSPECIFIED)
                    }
                },
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn readout_mode(&self) -> ASCOMResult<i32> {
        match &*self.camera.read().await {
            Some(ref camera) => match libqhyccd_sys::get_readout_mode(camera.handle) {
                Ok(readout_mode) => Ok(readout_mode as i32),
                Err(e) => {
                    error!(?e, "get_readout_mode failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn set_readout_mode(&self, readout_mode: i32) -> ASCOMResult {
        let readout_mode = readout_mode as u32;
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                match libqhyccd_sys::set_readout_mode(camera.handle, readout_mode) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        error!(?e, "set_readout_mode failed");
                        Err(ASCOMError::VALUE_NOT_SET)
                    }
                }
            }
            None => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn readout_modes(&self) -> ASCOMResult<Vec<String>> {
        match &*self.camera.read().await {
            Some(ref camera) => match camera.valid_readout_modes {
                Some(ref readout_modes) => {
                    Ok(readout_modes.iter().map(|m| m.name.clone()).collect())
                }
                None => Err(ASCOMError::NOT_CONNECTED),
            },
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn sensor_type(&self) -> ASCOMResult<SensorType> {
        match &*self.camera.read().await {
            Some(ref camera) => match libqhyccd_sys::is_feature_supported(
                camera.handle,
                libqhyccd_sys::CameraFeature::CamIsColor,
            ) {
                Ok(_) => Ok(SensorType::Color),
                Err(_) => Ok(SensorType::Monochrome),
            },
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn start_exposure(&self, duration: f64, light: bool) -> ASCOMResult {
        if duration < 0.0 {
            return Err(ASCOMError::invalid_value("duration must be >= 0"));
        }
        if !light {
            return Err(ASCOMError::invalid_operation("dark frames not supported"));
        }
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                let exposure_us = duration * 1_000_000.0;

                let (stop_tx, stop_rx) = oneshot::channel::<StopExposure>();
                let (done_tx, done_rx) = watch::channel(false);

                camera.last_exposure_start_time = Some(SystemTime::now());
                camera.last_exposure_duration_us = Some(exposure_us);

                camera.exposing = ExposingState::Exposing {
                    expected_duration_us: exposure_us,
                    start: SystemTime::now(),
                    stop_tx: Some(stop_tx),
                    done_rx,
                };

                match libqhyccd_sys::set_parameter(
                    camera.handle,
                    libqhyccd_sys::CameraFeature::ControlExposure,
                    exposure_us,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        error!(?e, "failed to set exposure time: {:?}", e);
                        return Err(ASCOMError::UNSPECIFIED);
                    }
                }

                let handle = camera.handle;
                let image = task::spawn_blocking(move || {
                    match libqhyccd_sys::start_single_frame_exposure(handle) {
                        Ok(_) => {}
                        Err(e) => {
                            error!(?e, "failed to stop exposure: {:?}", e);
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    }
                    let buffer_size = match libqhyccd_sys::get_image_size(handle) {
                        Ok(size) => size,
                        Err(e) => {
                            error!(?e, "get_image_size failed");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };

                    let image = match libqhyccd_sys::get_single_frame(handle, buffer_size) {
                        Ok(image) => image,
                        Err(e) => {
                            error!(?e, "get_single_frame failed");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };
                    Ok(image)
                });
                tokio::spawn(async move {
                    let _ = done_tx.send(true);
                });
                Ok(())
            }
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        //this is nto true for every camera, but better to say yes
        Ok(true)
    }

    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        //this is nto true for every camera, but better to say yes
        Ok(true)
    }

    async fn stop_exposure(&self) -> ASCOMResult {
        match &mut *self.camera.write().await {
            Some(ref mut camera) => match libqhyccd_sys::stop_exposure(camera.handle) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!(?e, "stop_exposure failed");
                    Err(ASCOMError::NOT_CONNECTED)
                }
            },
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        match &mut *self.camera.write().await {
            Some(ref mut camera) => {
                match libqhyccd_sys::abort_exposure_and_readout(camera.handle) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        error!(?e, "stop_exposure failed");
                        Err(ASCOMError::NOT_CONNECTED)
                    }
                }
            }
            None => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
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
