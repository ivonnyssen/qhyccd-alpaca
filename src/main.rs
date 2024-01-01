#![warn(clippy::integer_division)]
use qhyccd_rs::CCDChipInfo;
use std::time::SystemTime;
use tokio::sync::RwLock;

use ascom_alpaca::api::{
    Camera, CameraState, CargoServerInfo, Device, FilterWheel, ImageArray, SensorType,
};
use ascom_alpaca::{ASCOMError, ASCOMResult, Server};
use async_trait::async_trait;

use eyre::eyre;
use eyre::Result;
use ndarray::Array3;

#[macro_use]
extern crate educe;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(test)] {
        mod mocks;
        use crate::mocks::MockSdk as Sdk;
        use crate::mocks::MockCamera as QhyCamera;
        use qhyccd_rs::CCDChipArea;
    } else {
        use qhyccd_rs::{CCDChipArea, Sdk, Camera as QhyCamera};
    }
}

use tokio::sync::{oneshot, watch};
use tokio::task;
use tracing::{debug, error, instrument, trace};

#[derive(Debug)]
struct StopExposure {
    _want_image: bool,
}

#[derive(Educe)]
#[educe(Debug, PartialEq)]
enum State {
    Idle,
    Exposing {
        start: SystemTime,
        expected_duration_us: u32,
        #[educe(PartialEq(ignore))]
        stop_tx: Option<oneshot::Sender<StopExposure>>,
        #[educe(PartialEq(ignore))]
        done_rx: watch::Receiver<bool>,
    },
}

#[derive(Debug, Clone, Copy)]
struct BinningMode {
    symmetric_value: i32,
}
impl BinningMode {
    fn value(&self) -> i32 {
        self.symmetric_value
    }
}

#[derive(Debug)]
struct QhyccdCamera {
    unique_id: String,
    name: String,
    description: String,
    device: QhyCamera,
    binning: RwLock<BinningMode>,
    valid_bins: RwLock<Option<Vec<BinningMode>>>,
    target_temperature: RwLock<Option<f64>>,
    ccd_info: RwLock<Option<CCDChipInfo>>,
    intended_roi: RwLock<Option<qhyccd_rs::CCDChipArea>>,
    exposure_min_max_step: RwLock<Option<(f64, f64, f64)>>,
    last_exposure_start_time: RwLock<Option<SystemTime>>,
    last_exposure_duration_us: RwLock<Option<u32>>,
    last_image: RwLock<Option<ImageArray>>,
    state: RwLock<State>,
    gain_min_max: RwLock<Option<(f64, f64)>>,
    offset_min_max: RwLock<Option<(f64, f64)>>,
}

impl QhyccdCamera {
    fn get_valid_binning_modes(&self) -> Vec<BinningMode> {
        let mut valid_binning_modes = Vec::with_capacity(6);
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin1x1mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 1 });
        }
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin2x2mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 2 });
        }
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin3x3mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 3 });
        }
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin4x4mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 4 });
        }
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin6x6mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 6 });
        }
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamBin8x8mode)
            .is_some()
        {
            valid_binning_modes.push(BinningMode { symmetric_value: 8 });
        }
        valid_binning_modes
    }

    fn transform_image(image: qhyccd_rs::ImageData) -> Result<ImageArray> {
        match image.channels {
            1_u32 => match image.bits_per_pixel {
                8_u32 => {
                    if (image.width as usize * image.height as usize) > image.data.len() {
                        error!(
                            "image data length ({}) does not match width ({}) * height ({})",
                            image.data.len(),
                            image.width,
                            image.height
                        );
                        return Err(eyre!(
                            "image data length ({}) does not match width ({}) * height ({})",
                            image.data.len(),
                            image.width,
                            image.height
                        ));
                    }
                    let data: Vec<u8> =
                        image.data[0_usize..image.width as usize * image.height as usize].to_vec();
                    match Array3::from_shape_vec(
                        (image.width as usize, image.height as usize, 1),
                        data,
                    ) {
                        Ok(array_base) => {
                            let mut swapped = array_base;
                            swapped.swap_axes(0, 1);
                            Ok(swapped.into())
                        }
                        Err(e) => {
                            error!(?e, "could not transform image");
                            Err(eyre!(e))
                        }
                    }
                }
                16_u32 => {
                    if (image.width as usize * image.height as usize * 2_usize) > image.data.len() {
                        error!(
                            "image data length ({}) does not match width ({}) * height ({}) * 2",
                            image.data.len(),
                            image.width,
                            image.height
                        );
                        return Err(eyre!(
                            "image data length ({}) does not match width ({}) * height ({}) * 2",
                            image.data.len(),
                            image.width,
                            image.height
                        ));
                    }
                    let data = image.data
                        [0_usize..image.width as usize * image.height as usize * 2_usize]
                        .to_vec()
                        .chunks_exact(2)
                        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
                        .collect();
                    match Array3::from_shape_vec(
                        (image.height as usize, image.width as usize, 1),
                        data,
                    ) {
                        Ok(array_base) => {
                            let mut swapped = array_base;
                            swapped.swap_axes(0, 1);
                            Ok(swapped.into())
                        }
                        Err(e) => {
                            error!(?e, "could not transform image");
                            Err(eyre!(e))
                        }
                    }
                }
                other => {
                    error!("unsupported bits_per_pixel {:?}", other);
                    Err(eyre!("unsupported bits_per_pixel {:?}", other))
                }
            },
            other => {
                error!("unsupported number of channels {:?}", other);
                Err(eyre!("unsupported number of channels {:?}", other))
            }
        }
    }
}

#[async_trait]
impl Device for QhyccdCamera {
    fn static_name(&self) -> &str {
        &self.name
    }

    fn unique_id(&self) -> &str {
        &self.unique_id
    }

    async fn connected(&self) -> ASCOMResult<bool> {
        self.device.is_open().map_err(|e| {
            error!(?e, "is_open failed");
            ASCOMError::NOT_CONNECTED
        })
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        match self.connected().await? == connected {
            true => return Ok(()),
            false => match connected {
                true => {
                    self.device.open().map_err(|e| {
                        error!(?e, "open failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    self.device
                        .set_stream_mode(qhyccd_rs::StreamMode::SingleFrameMode)
                        .map_err(|e| {
                            error!(?e, "setting StreamMode to SingleFrameMode failed");
                            ASCOMError::NOT_CONNECTED
                        })?;
                    self.device.set_readout_mode(0).map_err(|e| {
                        error!(?e, "setting readout mode to 0 failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    self.device.init().map_err(|e| {
                        error!(?e, "camera init failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    let mut lock = self.ccd_info.write().await;
                    let info = self.device.get_ccd_info().map_err(|e| {
                        error!(?e, "get_ccd_info failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    *lock = Some(info);
                    let mut lock = self.intended_roi.write().await;
                    let area = self.device.get_effective_area().map_err(|e| {
                        error!(?e, "get_effective_area failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    *lock = Some(area);
                    *self.valid_bins.write().await = Some(self.get_valid_binning_modes());
                    let mut lock = self.exposure_min_max_step.write().await;
                    let exposure_min_max = self
                        .device
                        .get_parameter_min_max_step(qhyccd_rs::Control::Exposure)
                        .map_err(|e| {
                            error!(?e, "get_exposure_min_max_step failed");
                            ASCOMError::NOT_CONNECTED
                        })?;
                    *lock = Some(exposure_min_max);
                    match self.device.is_control_available(qhyccd_rs::Control::Gain) {
                        Some(_) => {
                            let mut lock = self.gain_min_max.write().await;
                            *lock = match self
                                .device
                                .get_parameter_min_max_step(qhyccd_rs::Control::Gain)
                            {
                                Ok((min, max, _step)) => Some((min, max)),
                                Err(e) => {
                                    error!(?e, "get_gain_min_max failed");
                                    return Err(ASCOMError::NOT_CONNECTED);
                                }
                            };
                        }
                        None => {
                            debug!("gain control not available");
                        }
                    }
                    match self.device.is_control_available(qhyccd_rs::Control::Offset) {
                        Some(_) => {
                            let mut lock = self.offset_min_max.write().await;
                            *lock = match self
                                .device
                                .get_parameter_min_max_step(qhyccd_rs::Control::Offset)
                            {
                                Ok((min, max, _step)) => Some((min, max)),
                                Err(e) => {
                                    error!(?e, "get_offset_min_max failed");
                                    return Err(ASCOMError::NOT_CONNECTED);
                                }
                            };
                        }
                        None => {
                            debug!("offset control not available");
                        }
                    }
                    Ok(())
                }
                false => self.device.close().map_err(|e| {
                    error!(?e, "close_camera failed");
                    ASCOMError::NOT_CONNECTED
                }),
            },
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        //TODO: add link to crates.io once published
        Ok("qhyccd_alpaca-rs".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl Camera for QhyccdCamera {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self
                .device
                .is_control_available(qhyccd_rs::Control::CamIsColor)
            {
                Some(_) => match self
                    .device
                    .is_control_available(qhyccd_rs::Control::CamColor)
                {
                    // https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
                    Some(bayer_id) => match bayer_id.try_into() {
                        Ok(qhyccd_rs::BayerMode::GBRG) => Ok(0),
                        Ok(qhyccd_rs::BayerMode::GRBG) => Ok(1),
                        Ok(qhyccd_rs::BayerMode::BGGR) => Ok(1),
                        Ok(qhyccd_rs::BayerMode::RGGB) => Ok(0),
                        Err(e) => {
                            error!(?e, "invalid bayer_id from camera");
                            Err(ASCOMError::INVALID_VALUE)
                        }
                    },
                    None => {
                        error!("invalid bayer_id from camera");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => Err(ASCOMError::NOT_IMPLEMENTED),
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self
                .device
                .is_control_available(qhyccd_rs::Control::CamIsColor)
            {
                Some(_) => match self
                    .device
                    .is_control_available(qhyccd_rs::Control::CamColor)
                {
                    // https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
                    Some(bayer_id) => match bayer_id.try_into() {
                        Ok(qhyccd_rs::BayerMode::GBRG) => Ok(1),
                        Ok(qhyccd_rs::BayerMode::GRBG) => Ok(0),
                        Ok(qhyccd_rs::BayerMode::BGGR) => Ok(1),
                        Ok(qhyccd_rs::BayerMode::RGGB) => Ok(0),
                        Err(e) => {
                            error!(?e, "invalid bayer_id from camera");
                            Err(ASCOMError::INVALID_VALUE)
                        }
                    },
                    None => {
                        error!("invalid bayer_id from camera");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => Err(ASCOMError::NOT_IMPLEMENTED),
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn sensor_name(&self) -> ASCOMResult<String> {
        //ideally we would use getModel, but that returns an error for all the cameras I have, so
        //parsing the model from the ID
        match self.connected().await {
            Ok(true) => match self.unique_id().split('-').next() {
                Some(model) => Ok(model.to_string()),
                None => {
                    error!("camera id should be MODEL-SerialNumber, but split failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => Ok(self.binning.read().await.value()),
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        if bin_x < 1 {
            return Err(ASCOMError::invalid_value("bin value must be >= 1"));
        }
        match self.connected().await {
            Ok(true) => {
                let mut lock = self.binning.write().await;
                match lock.symmetric_value == bin_x {
                    true => Ok(()),
                    false => {
                        match self.device.set_bin_mode(bin_x as u32, bin_x as u32) {
                            Ok(_) => {
                                //adjust start and num values
                                let old = lock.symmetric_value;
                                *lock = BinningMode {
                                    symmetric_value: bin_x,
                                };
                                let mut roi_lock = self.intended_roi.write().await;
                                *roi_lock = roi_lock.map(|roi| CCDChipArea {
                                    start_x: (roi.start_x as f32 * old as f32 / bin_x as f32)
                                        as u32,
                                    start_y: (roi.start_y as f32 * old as f32 / bin_x as f32)
                                        as u32,
                                    width: (roi.width as f32 * old as f32 / bin_x as f32) as u32,
                                    height: (roi.height as f32 * old as f32 / bin_x as f32) as u32,
                                });
                                Ok(())
                            }
                            Err(e) => {
                                error!(?e, "set_bin_mode failed");
                                Err(ASCOMError::VALUE_NOT_SET)
                            }
                        }
                    }
                }
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
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
        match self.connected().await {
            Ok(true) => match self
                .get_valid_binning_modes()
                .iter()
                .map(|m| m.value())
                .max()
            {
                Some(max) => Ok(max),
                None => {
                    error!("valid_binning_modes is empty");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        self.max_bin_x().await
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        match self.connected().await {
            Ok(true) => match *self.state.read().await {
                State::Idle => Ok(CameraState::Idle),
                State::Exposing { .. } => Ok(CameraState::Exposing),
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn exposure_max(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.exposure_min_max_step.read().await {
                Some((_min, max, _step)) => Ok(max / 1_000_000_f64), //values from the camera are in
                //us
                None => {
                    error!("should have a max exposure value, but don't");
                    Err(ASCOMError::INVALID_VALUE)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn exposure_min(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.exposure_min_max_step.read().await {
                Some((min, _max, _step)) => Ok(min / 1_000_000_f64), //values from the camera are in
                //us
                None => {
                    error!("should have a min exposure value, but don't");
                    Err(ASCOMError::INVALID_VALUE)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn exposure_resolution(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.exposure_min_max_step.read().await {
                Some((_min, _max, step)) => Ok(step / 1_000_000_f64), //values from the camera are in
                //us
                None => {
                    error!("should have a step exposure value, but don't");
                    Err(ASCOMError::INVALID_VALUE)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        debug!("full_well_capacity not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        match self.connected().await {
            Ok(true) => match self
                .device
                .is_control_available(qhyccd_rs::Control::CamMechanicalShutter)
            {
                Some(_) => Ok(true),
                None => {
                    debug!("no mechanical shutter");
                    Ok(false)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        match self.connected().await {
            Ok(true) => match (*self.last_image.read().await).clone() {
                Some(image) => Ok(image),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        match self.connected().await {
            Ok(true) => match *self.state.read().await {
                State::Idle => match *self.last_image.read().await {
                    Some(_) => Ok(true),
                    None => Ok(false),
                },
                State::Exposing { .. } => Ok(false),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn last_exposure_start_time(&self) -> ASCOMResult<SystemTime> {
        match self.connected().await {
            Ok(true) => match *self.last_exposure_start_time.read().await {
                Some(time) => Ok(time),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.last_exposure_duration_us.read().await {
                Some(duration) => Ok(duration as f64 / 1_000_000_f64),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn max_adu(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self
                .device
                .get_parameter(qhyccd_rs::Control::OutputDataActualBits)
            {
                Ok(bits) => {
                    debug!(?bits, "ADU");
                    Ok(2_i32.pow(bits as u32))
                }
                Err(e) => {
                    error!(?e, "could not get OutputDataActualBits");
                    Err(ASCOMError::VALUE_NOT_SET)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.ccd_info.read().await {
                Some(ccd_info) => Ok(ccd_info.image_width as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn camera_ysize(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.ccd_info.read().await {
                Some(ccd_info) => Ok(ccd_info.image_height as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn start_x(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.intended_roi.read().await {
                Some(roi) => Ok(roi.start_x as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_start_x(&self, start_x: i32) -> ASCOMResult {
        if start_x < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        match self.connected().await {
            Ok(true) => {
                let mut lock = self.intended_roi.write().await;
                *lock = match *lock {
                    Some(intended_roi) => Some(CCDChipArea {
                        start_x: start_x as u32,
                        ..intended_roi
                    }),
                    None => {
                        error!("no roi defined, but trying to set start_x");
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                };
                Ok(())
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn start_y(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.intended_roi.read().await {
                Some(roi) => Ok(roi.start_y as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_start_y(&self, start_y: i32) -> ASCOMResult {
        if start_y < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        match self.connected().await {
            Ok(true) => {
                let mut lock = self.intended_roi.write().await;
                *lock = match *lock {
                    Some(intended_roi) => Some(CCDChipArea {
                        start_y: start_y as u32,
                        ..intended_roi
                    }),
                    None => {
                        error!("no roi defined, but trying to set start_y");
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                };
                Ok(())
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn num_x(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.intended_roi.read().await {
                Some(roi) => Ok(roi.width as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_num_x(&self, num_x: i32) -> ASCOMResult {
        if num_x < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        match self.connected().await {
            Ok(true) => {
                let mut lock = self.intended_roi.write().await;
                *lock = match *lock {
                    Some(intended_roi) => Some(CCDChipArea {
                        width: num_x as u32,
                        ..intended_roi
                    }),
                    None => {
                        error!("no roi defined, but trying to set num_x");
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                };
                Ok(())
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn num_y(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.intended_roi.read().await {
                Some(roi) => Ok(roi.height as i32),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_num_y(&self, num_y: i32) -> ASCOMResult {
        if num_y < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        match self.connected().await {
            Ok(true) => {
                let mut lock = self.intended_roi.write().await;
                *lock = match *lock {
                    Some(intended_roi) => Some(CCDChipArea {
                        height: num_y as u32,
                        ..intended_roi
                    }),
                    None => {
                        error!("no roi defined, but trying to set num_y");
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                };
                Ok(())
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn percent_completed(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match *self.state.read().await {
                State::Idle => Ok(100),
                State::Exposing {
                    expected_duration_us,
                    ..
                } => match self.device.get_remaining_exposure_us() {
                    Ok(remaining) => {
                        let res = (100_f64 * remaining as f64 / expected_duration_us as f64) as i32;
                        if res > 100_i32 {
                            Ok(100_i32)
                        } else {
                            Ok(res)
                        }
                    }
                    Err(e) => {
                        error!(?e, "get_remaining_exposure_us failed");
                        Err(ASCOMError::UNSPECIFIED)
                    }
                },
            },
            _ => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn readout_mode(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self.device.get_readout_mode() {
                Ok(readout_mode) => Ok(readout_mode as i32),
                Err(e) => {
                    error!(?e, "get_readout_mode failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn set_readout_mode(&self, readout_mode: i32) -> ASCOMResult {
        let readout_mode = readout_mode as u32;
        match self.connected().await {
            Ok(true) => match self.device.set_readout_mode(readout_mode) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!(?e, "set_readout_mode failed");
                    Err(ASCOMError::VALUE_NOT_SET)
                }
            },
            _ => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn readout_modes(&self) -> ASCOMResult<Vec<String>> {
        match self.connected().await {
            Ok(true) => match self.device.get_number_of_readout_modes() {
                Ok(num) => {
                    let mut readout_modes = Vec::with_capacity(num as usize);
                    for i in 0..num {
                        match self.device.get_readout_mode_name(i) {
                            Ok(readout_mode) => readout_modes.push(readout_mode),
                            Err(e) => {
                                error!(?e, "get_readout_mode failed");
                                return Err(ASCOMError::UNSPECIFIED);
                            }
                        }
                    }
                    Ok(readout_modes)
                }
                Err(e) => {
                    error!(?e, "get_number_of_readout_modes failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => return Err(ASCOMError::NOT_CONNECTED),
        }
    }

    async fn sensor_type(&self) -> ASCOMResult<SensorType> {
        //see here: https://ascom-standards.org/api/#/Camera%20Specific%20Methods/get_camera__device_number__imagearray
        match self.connected().await {
            Ok(true) => match self
                .device
                .is_control_available(qhyccd_rs::Control::CamIsColor)
            {
                Some(_) => match self
                    .device
                    .is_control_available(qhyccd_rs::Control::CamColor)
                {
                    Some(_bayer_id) => Ok(SensorType::RGGB),
                    None => {
                        error!("invalid bayer_id from camera");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => Ok(SensorType::Monochrome),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    #[instrument(level = "trace")]
    async fn start_exposure(&self, duration: f64, light: bool) -> ASCOMResult {
        if duration < 0.0 {
            return Err(ASCOMError::invalid_value("duration must be >= 0"));
        }
        if !light {
            return Err(ASCOMError::invalid_operation("dark frames not supported"));
        }
        match self.connected().await {
            Ok(true) => {
                if self.start_x().await? > self.num_x().await? {
                    return Err(ASCOMError::invalid_value("StartX > NumX"));
                }
                if self.start_y().await? > self.num_y().await? {
                    return Err(ASCOMError::invalid_value("StartY > NumY"));
                }
                if self.num_x().await?
                    > (self.camera_xsize().await? as f32 / self.bin_x().await? as f32) as i32
                {
                    return Err(ASCOMError::invalid_value("NumX > CameraXSize"));
                }
                if self.num_y().await?
                    > (self.camera_ysize().await? as f32 / self.bin_y().await? as f32) as i32
                {
                    return Err(ASCOMError::invalid_value("NumY > CameraYSize"));
                }
                match *self.intended_roi.read().await {
                    Some(roi) => match self.device.set_roi(roi) {
                        Ok(_) => {}
                        Err(e) => {
                            debug!(?e, "failed to set ROI");
                            return Err(ASCOMError::invalid_value("failed to set ROI"));
                        }
                    },
                    None => {
                        debug!("no roi defined, but trying to start exposure");
                        return Err(ASCOMError::invalid_value("no ROI defined for camera"));
                    }
                };
                let exposure_us = (duration * 1_000_000_f64) as u32;
                let (stop_tx, stop_rx) = oneshot::channel::<StopExposure>();
                let (done_tx, done_rx) = watch::channel(false);

                let mut lock = self.state.write().await;
                if *lock != State::Idle {
                    error!("camera already exposing");
                    return Err(ASCOMError::INVALID_OPERATION);
                } else {
                    *lock = State::Exposing {
                        start: SystemTime::now(),
                        expected_duration_us: exposure_us,
                        stop_tx: Some(stop_tx),
                        done_rx,
                    }
                };

                *self.last_exposure_start_time.write().await = Some(SystemTime::now());
                *self.last_exposure_duration_us.write().await = Some(exposure_us);

                match self
                    .device
                    .set_parameter(qhyccd_rs::Control::Exposure, exposure_us as f64)
                {
                    Ok(_) => {}
                    Err(e) => {
                        error!(?e, "failed to set exposure time: {:?}", e);
                        return Err(ASCOMError::UNSPECIFIED);
                    }
                }

                let device = self.device.clone();
                let image = task::spawn_blocking(move || {
                    match device.start_single_frame_exposure() {
                        Ok(_) => {}
                        Err(e) => {
                            error!(?e, "failed to stop exposure: {:?}", e);
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    }
                    let buffer_size = match device.get_image_size() {
                        Ok(size) => size,
                        Err(e) => {
                            error!(?e, "get_image_size failed");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };
                    debug!(?buffer_size);

                    let image = match device.get_single_frame(buffer_size) {
                        Ok(image) => image,
                        Err(e) => {
                            error!(?e, "get_single_frame failed");
                            return Err(ASCOMError::UNSPECIFIED);
                        }
                    };
                    Ok(image)
                });
                let stop = stop_rx;
                tokio::select! {
                    image = image => {
                        match image {
                            Ok(image_result) => {
                                match image_result {
                                    Ok(image) => { let  mut lock = self.last_image.write().await;
                                        match QhyccdCamera::transform_image(image) {
                                            Ok(image) => *lock = Some(image),
                                            Err(e) => {
                                                error!(?e, "failed to transform image");
                                                return Err(ASCOMError::INVALID_OPERATION)
                                            }
                                        }
                                        let _ = done_tx.send(true);
                                    },
                                    Err(e) => {
                                        error!(?e, "failed to get image");
                                        return Err(ASCOMError::UNSPECIFIED);
                                    }
                                }
                            }
                            Err(e) => {
                                error!(?e, "failed to get image");
                                return Err(ASCOMError::UNSPECIFIED);
                            }
                        }
                    },
                    _ = stop => {
                        match self.device.abort_exposure_and_readout() {
                            Ok(_) => {},
                            Err(e) => {
                                error!(?e, "failed to stop exposure: {:?}", e);
                                return Err(ASCOMError::UNSPECIFIED);
                            }
                        }
                    }
                }
                *lock = State::Idle;
                Ok(())
            }
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn can_stop_exposure(&self) -> ASCOMResult<bool> {
        //this is not true for every camera, but better to say no here
        Ok(false)
    }

    async fn can_abort_exposure(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    async fn stop_exposure(&self) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
        /*
        match self.connected().await {
            Ok(true) => match self.device.stop_exposure() {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!(?e, "stop_exposure failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        } */
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        match self.connected().await {
            Ok(true) => match self.device.abort_exposure_and_readout() {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!(?e, "stop_exposure failed");
                    Err(ASCOMError::UNSPECIFIED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn pixel_size_x(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.ccd_info.read().await {
                Some(ccd_info) => Ok(ccd_info.pixel_width),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn pixel_size_y(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match *self.ccd_info.read().await {
                Some(ccd_info) => Ok(ccd_info.pixel_height),
                None => Err(ASCOMError::VALUE_NOT_SET),
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn can_get_cooler_power(&self) -> ASCOMResult<bool> {
        self.can_set_ccd_temperature().await
    }

    async fn can_set_ccd_temperature(&self) -> ASCOMResult<bool> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => Ok(true),
                None => {
                    debug!("no cooler");
                    Ok(false)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn ccd_temperature(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => match self.device.get_parameter(qhyccd_rs::Control::CurTemp) {
                    Ok(temperature) => Ok(temperature),
                    Err(e) => {
                        error!(?e, "could not get current temperature");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => {
                    debug!("no cooler");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_ccd_temperature(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => match *self.target_temperature.read().await {
                    Some(temperature) => Ok(temperature),
                    None => self.ccd_temperature().await,
                },
                None => {
                    debug!("no cooler");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_set_ccd_temperature(&self, set_ccd_temperature: f64) -> ASCOMResult {
        //ASCOM checks
        if !(-273.15..=80_f64).contains(&set_ccd_temperature) {
            return Err(ASCOMError::INVALID_VALUE);
        }
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => match self
                    .device
                    .set_parameter(qhyccd_rs::Control::Cooler, set_ccd_temperature)
                {
                    Ok(_) => {
                        *self.target_temperature.write().await = Some(set_ccd_temperature);
                        Ok(())
                    }
                    Err(e) => {
                        error!(?e, "could not set target temperature");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => {
                    debug!("no cooler");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn cooler_on(&self) -> ASCOMResult<bool> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => match self.device.get_parameter(qhyccd_rs::Control::CurPWM) {
                    Ok(cooler_power) => match cooler_power > 0_f64 {
                        true => Ok(true),
                        false => Ok(false),
                    },
                    Err(e) => {
                        error!(?e, "could not get current power");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => {
                    debug!("cannot control temp, probably no cooler");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn set_cooler_on(&self, cooler_on: bool) -> ASCOMResult {
        match cooler_on {
            true => {
                match self.cooler_on().await {
                    Ok(true) => {
                        Ok(()) //nothing to do here
                    }
                    Ok(false) => {
                        match self
                            .device
                            .set_parameter(qhyccd_rs::Control::ManualPWM, 1_f64 / 100_f64 * 255_f64)
                        {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                error!(?e, "error setting cooler power to 1");
                                Err(ASCOMError::INVALID_OPERATION)
                            }
                        }
                    }
                    Err(e) => {
                        error!(?e, "could not turn cooler on");
                        Err(e)
                    }
                }
            }
            false => {
                match self.cooler_on().await {
                    Ok(true) => {
                        match self
                            .device
                            .set_parameter(qhyccd_rs::Control::ManualPWM, 0_f64)
                        {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                error!(?e, "error setting cooler power to 0");
                                Err(ASCOMError::INVALID_OPERATION)
                            }
                        }
                    }
                    Ok(false) => {
                        Ok(()) //nothing to do here
                    }
                    Err(e) => {
                        error!(?e, "could not turn cooler off");
                        Err(e)
                    }
                }
            }
        }
    }

    async fn cooler_power(&self) -> ASCOMResult<f64> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Cooler) {
                Some(_) => match self.device.get_parameter(qhyccd_rs::Control::CurPWM) {
                    Ok(cooler_power) => Ok(cooler_power / 255_f64 * 100_f64),
                    Err(e) => {
                        error!(?e, "could not get current temperature");
                        Err(ASCOMError::INVALID_VALUE)
                    }
                },
                None => {
                    debug!("no cooler");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        }
    }

    async fn gain(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Gain) {
                Some(_) => match self.device.get_parameter(qhyccd_rs::Control::Gain) {
                    Ok(gain) => Ok(gain as i32),
                    Err(e) => {
                        error!(?e, "failed to set gain");
                        Err(ASCOMError::UNSPECIFIED)
                    }
                },
                None => {
                    debug!("gain control not available");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn set_gain(&self, gain: i32) -> ASCOMResult {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Gain) {
                Some(_) => {
                    let (min, max) = self
                        .gain_min_max
                        .read()
                        .await
                        .ok_or(ASCOMError::unspecified("camera reports gain control available, but min, max values are not set after initialization"))?;
                    if !(min as i32..=max as i32).contains(&gain) {
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                    match self
                        .device
                        .set_parameter(qhyccd_rs::Control::Gain, gain as f64)
                    {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            error!(?e, "failed to set gain");
                            Err(ASCOMError::UNSPECIFIED)
                        }
                    }
                }
                None => {
                    debug!("gain control not available");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn gain_max(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => self
                .gain_min_max
                .read()
                .await
                .map(|(_min, max)| max as i32)
                .ok_or(ASCOMError::NOT_IMPLEMENTED),
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn gain_min(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => self
                .gain_min_max
                .read()
                .await
                .map(|(min, _max)| min as i32)
                .ok_or(ASCOMError::NOT_IMPLEMENTED),
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn offset(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Offset) {
                Some(_) => match self.device.get_parameter(qhyccd_rs::Control::Offset) {
                    Ok(offset) => Ok(offset as i32),
                    Err(e) => {
                        error!(?e, "failed to set offset");
                        Err(ASCOMError::UNSPECIFIED)
                    }
                },
                None => {
                    debug!("offset control not available");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn set_offset(&self, offset: i32) -> ASCOMResult {
        match self.connected().await {
            Ok(true) => match self.device.is_control_available(qhyccd_rs::Control::Offset) {
                Some(_) => {
                    let (min, max) = self
                        .offset_min_max
                        .read()
                        .await
                        .ok_or(ASCOMError::unspecified("camera reports offset control available, but min, max values are not set after initialization"))?;
                    if !(min as i32..=max as i32).contains(&offset) {
                        return Err(ASCOMError::INVALID_VALUE);
                    }
                    match self
                        .device
                        .set_parameter(qhyccd_rs::Control::Offset, offset as f64)
                    {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            error!(?e, "failed to set offset");
                            Err(ASCOMError::UNSPECIFIED)
                        }
                    }
                }
                None => {
                    debug!("offset control not available");
                    Err(ASCOMError::NOT_IMPLEMENTED)
                }
            },
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn offset_max(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => self
                .offset_min_max
                .read()
                .await
                .map(|(_min, max)| max as i32)
                .ok_or(ASCOMError::NOT_IMPLEMENTED),
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn offset_min(&self) -> ASCOMResult<i32> {
        match self.connected().await {
            Ok(true) => self
                .offset_min_max
                .read()
                .await
                .map(|(min, _max)| min as i32)
                .ok_or(ASCOMError::NOT_IMPLEMENTED),
            _ => {
                error!("camera not connected");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }
}

#[derive(Debug)]
struct QhyccdFilterWheel {
    unique_id: String,
    name: String,
    description: String,
    device: QhyCamera,
    num_filters: RwLock<Option<u32>>,
    position: RwLock<Option<u32>>,
}

#[async_trait]
impl Device for QhyccdFilterWheel {
    fn static_name(&self) -> &str {
        &self.name
    }

    fn unique_id(&self) -> &str {
        &self.unique_id
    }

    async fn connected(&self) -> ASCOMResult<bool> {
        self.device.is_open().map_err(|e| {
            error!(?e, "is_open failed");
            ASCOMError::NOT_CONNECTED
        })
    }

    async fn set_connected(&self, connected: bool) -> ASCOMResult {
        match self.connected().await? == connected {
            true => return Ok(()),
            false => match connected {
                true => {
                    self.device.open().map_err(|e| {
                        error!(?e, "open failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    self.device.init().map_err(|e| {
                        error!(?e, "camera init failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                    match self.device.is_cfw_plugged_in() {
                        Ok(true) => {
                            let mut lock = self.num_filters.write().await;
                            *lock = match self.device.get_number_of_filters() {
                                Some(num) => Some(num),
                                None => {
                                    error!("get_number_of_filters failed, even though is_cfw_plugged_in returned true");
                                    return Err(ASCOMError::NOT_CONNECTED);
                                }
                            };
                            let mut lock = self.position.write().await;
                            *lock = match self.device.get_fw_position() {
                                Some(position) => Some(position),
                                None => {
                                    error!("get_fw_position failed, even though is_cfw_plugged_in returned true");
                                    return Err(ASCOMError::NOT_CONNECTED);
                                }
                            }
                        }
                        Ok(false) => {
                            debug!("filter wheel not plugged in");
                        }
                        Err(e) => {
                            error!(?e, "is_cfw_plugged_in failed");
                            return Err(ASCOMError::NOT_CONNECTED);
                        }
                    }
                    Ok(())
                }
                false => self.device.close().map_err(|e| {
                    error!(?e, "close_camera failed");
                    ASCOMError::NOT_CONNECTED
                }),
            },
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        //TODO: add link to crates.io once published
        Ok("qhyccd_alpaca-rs".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl FilterWheel for QhyccdFilterWheel {
    async fn position(&self) -> ASCOMResult<i32> {
        match self.device.get_fw_position() {
            Some(position) => Ok(position as i32),
            None => {
                error!("failed to retrieve position for filter wheel");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }

    async fn set_position(&self, position: i32) -> ASCOMResult {
        let position = (position + 48_i32) as u32;
        match self.device.set_fw_position(position) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!(?e, "failed to set filter wheel position");
                Err(ASCOMError::NOT_CONNECTED)
            }
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let mut server = Server {
        info: CargoServerInfo!(),
        ..Default::default()
    };

    server.listen_addr.set_port(8000);

    let sdk = Sdk::new().expect("SDK::new failed");
    let sdk_version = sdk.version().expect("get_sdk_version failed");
    trace!(sdk_version = ?sdk_version);
    trace!(cameras = ?sdk.cameras().count());

    sdk.cameras().for_each(|c| {
        let camera = QhyccdCamera {
            unique_id: c.id().to_owned(),
            name: c.id().to_owned(),
            description: "QHYCCD camera".to_owned(),
            device: c.clone(),
            binning: RwLock::new(BinningMode { symmetric_value: 1 }),
            valid_bins: RwLock::new(None),
            target_temperature: RwLock::new(None),
            ccd_info: RwLock::new(None),
            intended_roi: RwLock::new(None),
            exposure_min_max_step: RwLock::new(None),
            last_exposure_start_time: RwLock::new(None),
            last_exposure_duration_us: RwLock::new(None),
            last_image: RwLock::new(None),
            state: RwLock::new(State::Idle),
            gain_min_max: RwLock::new(None),
            offset_min_max: RwLock::new(None),
        };
        tracing::debug!(?camera, "Registering camera");
        server
            .devices
            .register::<dyn ascom_alpaca::api::Camera>(camera);
        match c.open() {
            Ok(_) => match c.is_cfw_plugged_in() {
                Ok(true) => {
                    let filter_wheel = QhyccdFilterWheel {
                        unique_id: c.id().to_owned(),
                        name: c.id().to_owned(),
                        description: "QHYCCD filter wheel".to_owned(),
                        device: c.clone(),
                        num_filters: RwLock::new(None),
                        position: RwLock::new(None),
                    };
                    server
                        .devices
                        .register::<dyn ascom_alpaca::api::FilterWheel>(filter_wheel);
                }
                Ok(false) => {
                    debug!(?c, "filter wheel not plugged in");
                }
                Err(e) => {
                    error!(?e, "failed to get filter wheel plugged in state");
                }
            },
            Err(e) => {
                error!(?e, "failed to open camera");
            }
        };
    });

    server.start().await
}

#[cfg(test)]
mod tests;
