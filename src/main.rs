#![warn(clippy::integer_division)]
use core::f64;
use qhyccd_rs::CCDChipInfo;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use ascom_alpaca::api::camera::{CameraState, ImageArray, SensorType};
use ascom_alpaca::api::{Camera, CargoServerInfo, Device, FilterWheel};
use ascom_alpaca::{ASCOMError, ASCOMResult, Server};
use async_trait::async_trait;

use eyre::{Result, eyre};
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
        use crate::mocks::MockFilterWheel as QhyFilterWheel;
    } else {
        use qhyccd_rs::{CCDChipArea, Sdk, Camera as QhyCamera, FilterWheel as QhyFilterWheel};
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

#[derive(Debug)]
struct QhyccdCamera {
    unique_id: String,
    name: String,
    description: String,
    device: QhyCamera,
    binning: RwLock<u32>,
    valid_bins: RwLock<Option<Vec<u32>>>,
    target_temperature: RwLock<Option<f64>>,
    ccd_info: RwLock<Option<CCDChipInfo>>,
    intended_roi: RwLock<Option<qhyccd_rs::CCDChipArea>>,
    readout_speed_min_max_step: RwLock<Option<(f64, f64, f64)>>,
    exposure_min_max_step: RwLock<Option<(f64, f64, f64)>>,
    last_exposure_start_time: RwLock<Option<SystemTime>>,
    last_exposure_duration_us: RwLock<Option<u32>>,
    last_image: Arc<RwLock<Option<ImageArray>>>,
    state: Arc<RwLock<State>>,
    gain_min_max: RwLock<Option<(f64, f64)>>,
    offset_min_max: RwLock<Option<(f64, f64)>>,
}

impl QhyccdCamera {
    fn get_valid_binning_modes(&self) -> Vec<u32> {
        let mut valid_binning_modes = Vec::with_capacity(6);
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin1x1mode)
            .is_some()
            .then(|| valid_binning_modes.push(1_u32));
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin2x2mode)
            .is_some()
            .then(|| valid_binning_modes.push(2_u32));
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin3x3mode)
            .is_some()
            .then(|| valid_binning_modes.push(3_u32));
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin4x4mode)
            .is_some()
            .then(|| valid_binning_modes.push(4_u32));
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin6x6mode)
            .is_some()
            .then(|| valid_binning_modes.push(6_u32));
        self.device
            .is_control_available(qhyccd_rs::Control::CamBin8x8mode)
            .is_some()
            .then(|| valid_binning_modes.push(8_u32));
        valid_binning_modes
    }

    fn transform_image_static(image: qhyccd_rs::ImageData) -> Result<ImageArray> {
        match image.channels {
            1_u32 => match image.bits_per_pixel {
                8_u32 => {
                    if image.width as usize * image.height as usize > image.data.len() {
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
                    let array_base = Array3::from_shape_vec(
                        (image.height as usize, image.width as usize, 1_usize),
                        data,
                    )
                    .map_err(|e| {
                        error!(?e, "could not transform image");
                        eyre!(e)
                    })?;
                    let mut swapped = array_base;
                    swapped.swap_axes(0, 1);
                    Ok(swapped.into())
                }
                16_u32 => {
                    if image.width as usize * image.height as usize * 2 > image.data.len() {
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
                    let array_base = Array3::from_shape_vec(
                        (image.height as usize, image.width as usize, 1_usize),
                        data,
                    )
                    .map_err(|e| {
                        error!(?e, "could not transform image");
                        eyre!(e)
                    })?;
                    let mut swapped = array_base;
                    swapped.swap_axes(0, 1);
                    Ok(swapped.into())
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

    async fn connect(&self) -> ASCOMResult {
        self.device.open().map_err(|e| {
            error!(?e, "open failed");
            ASCOMError::NOT_CONNECTED
        })?;
        self.device
            .is_control_available(qhyccd_rs::Control::CamSingleFrameMode)
            .ok_or_else(|| {
                error!("SingleFrameMode is not avaialble");
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
        self.device
            .set_if_available(qhyccd_rs::Control::TransferBit, 16_f64)
            .map_err(|e| {
                error!(?e, "setting transfer bits is not supported");
                ASCOMError::NOT_CONNECTED
            })?;
        trace!(cam_transfer_bit = 16.0);
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
        match self.device.is_control_available(qhyccd_rs::Control::Speed) {
            Some(_) => {
                let mut lock = self.readout_speed_min_max_step.write().await;
                let readout_speed_min_max_step = self
                    .device
                    .get_parameter_min_max_step(qhyccd_rs::Control::Speed)
                    .map_err(|e| {
                        error!(?e, "get_readout_speed_min_max_step failed");
                        ASCOMError::NOT_CONNECTED
                    })?;
                *lock = Some(readout_speed_min_max_step);
            }
            None => debug!("readout_speed control not available"),
        }
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
        if self.connected().await? == connected {
            return Ok(());
        };
        match connected {
            true => self.connect().await,
            false => self.device.close().map_err(|e| {
                error!(?e, "close_camera failed");
                ASCOMError::NOT_CONNECTED
            }),
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("qhyccd-alpaca See: https://crates.io/crates/qhyccd-alpaca".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

macro_rules! ensure_connected {
    ($self:ident) => {
        if !$self.connected().await.is_ok_and(|connected| connected) {
            error!("camera not connected");
            return Err(ASCOMError::NOT_CONNECTED);
        }
    };
}

#[async_trait]
impl Camera for QhyccdCamera {
    async fn bayer_offset_x(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::CamIsColor)
            .ok_or_else(|| {
                error!("CamIsColor not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let bayer_id = self
            .device
            .is_control_available(qhyccd_rs::Control::CamColor)
            .ok_or_else(|| {
                error!("invalid bayer_id from camera");
                ASCOMError::INVALID_VALUE
            })?;
        // https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
        match bayer_id.try_into() {
            Ok(qhyccd_rs::BayerMode::GBRG) => Ok(0),
            Ok(qhyccd_rs::BayerMode::GRBG) => Ok(1),
            Ok(qhyccd_rs::BayerMode::BGGR) => Ok(1),
            Ok(qhyccd_rs::BayerMode::RGGB) => Ok(0),
            Err(e) => {
                error!(?e, "invalid bayer_id from camera");
                Err(ASCOMError::INVALID_VALUE)
            }
        }
    }

    async fn bayer_offset_y(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::CamIsColor)
            .ok_or_else(|| {
                error!("CamIsColor not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let bayer_id = self
            .device
            .is_control_available(qhyccd_rs::Control::CamColor)
            .ok_or_else(|| {
                error!("invalid bayer_id from camera");
                ASCOMError::INVALID_VALUE
            })?;
        // https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
        match bayer_id.try_into() {
            Ok(qhyccd_rs::BayerMode::GBRG) => Ok(1),
            Ok(qhyccd_rs::BayerMode::GRBG) => Ok(0),
            Ok(qhyccd_rs::BayerMode::BGGR) => Ok(1),
            Ok(qhyccd_rs::BayerMode::RGGB) => Ok(0),
            Err(e) => {
                error!(?e, "invalid bayer_id from camera");
                Err(ASCOMError::INVALID_VALUE)
            }
        }
    }

    async fn sensor_name(&self) -> ASCOMResult<String> {
        //ideally we would use getModel, but that returns an error for all the cameras I have, so
        //parsing the model from the ID
        ensure_connected!(self);
        match self.unique_id().split('-').next() {
            Some(model) => Ok(model.to_string()),
            None => {
                error!("camera id should be MODEL-SerialNumber, but split failed");
                Err(ASCOMError::INVALID_OPERATION)
            }
        }
    }

    async fn bin_x(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        Ok(*self.binning.read().await as i32)
    }

    async fn set_bin_x(&self, bin_x: i32) -> ASCOMResult {
        ensure_connected!(self);
        let valid_bins = self.valid_bins.read().await.clone().ok_or_else(|| {
            error!("valid_bins not set");
            ASCOMError::NOT_CONNECTED
        })?;
        valid_bins
            .iter()
            .find(|bin| **bin as i32 == bin_x)
            .ok_or_else(|| {
                error!("trying to set invalid bin value: {}", bin_x);
                ASCOMError::invalid_value("bin value must be one of the valid bins")
            })?;
        let mut lock = self.binning.write().await;
        if *lock as i32 == bin_x {
            return Ok(());
        };
        self.device
            .set_bin_mode(bin_x as u32, bin_x as u32)
            .map_err(|e| {
                error!(?e, "set_bin_mode failed");
                ASCOMError::VALUE_NOT_SET
            })?;
        //adjust start and num values
        let old = *lock;
        *lock = bin_x as u32;
        let mut roi_lock = self.intended_roi.write().await;
        *roi_lock = roi_lock.map(|roi| CCDChipArea {
            start_x: (roi.start_x as f32 * old as f32 / bin_x as f32) as u32,
            start_y: (roi.start_y as f32 * old as f32 / bin_x as f32) as u32,
            width: (roi.width as f32 * old as f32 / bin_x as f32) as u32,
            height: (roi.height as f32 * old as f32 / bin_x as f32) as u32,
        });
        Ok(())
    }

    async fn bin_y(&self) -> ASCOMResult<i32> {
        self.bin_x().await
    }

    async fn set_bin_y(&self, bin_y: i32) -> ASCOMResult {
        self.set_bin_x(bin_y).await
    }

    async fn max_bin_x(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.get_valid_binning_modes()
            .iter()
            .map(|m| *m as i32)
            .max()
            .ok_or_else(|| {
                error!("valid_binning_modes is empty");
                ASCOMError::INVALID_OPERATION
            })
    }

    async fn max_bin_y(&self) -> ASCOMResult<i32> {
        self.max_bin_x().await
    }

    async fn camera_state(&self) -> ASCOMResult<CameraState> {
        ensure_connected!(self);
        match *self.state.read().await {
            State::Idle => Ok(CameraState::Idle),
            State::Exposing { .. } => Ok(CameraState::Exposing),
        }
    }

    async fn electrons_per_adu(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn exposure_max(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        match *self.exposure_min_max_step.read().await {
            Some((_min, max, _step)) => Ok(max / 1_000_000_f64), //values from the camera are in
            //us
            None => {
                error!("should have a max exposure value, but don't");
                Err(ASCOMError::INVALID_VALUE)
            }
        }
    }

    async fn exposure_min(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        match *self.exposure_min_max_step.read().await {
            Some((min, _max, _step)) => Ok(min / 1_000_000_f64), //values from the camera are in
            //us
            None => {
                error!("should have a min exposure value, but don't");
                Err(ASCOMError::INVALID_VALUE)
            }
        }
    }

    async fn exposure_resolution(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        match *self.exposure_min_max_step.read().await {
            Some((_min, _max, step)) => Ok(step / 1_000_000_f64), //values from the camera are in
            //us
            None => {
                error!("should have a step exposure value, but don't");
                Err(ASCOMError::INVALID_VALUE)
            }
        }
    }

    async fn full_well_capacity(&self) -> ASCOMResult<f64> {
        debug!("full_well_capacity not implemented");
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn has_shutter(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::CamMechanicalShutter)
            .map_or_else(
                || {
                    debug!("no mechanical shutter");
                    Ok(false)
                },
                |_| Ok(true),
            )
    }

    async fn image_array(&self) -> ASCOMResult<ImageArray> {
        ensure_connected!(self);
        match (*self.last_image.read().await).clone() {
            Some(image) => Ok(image),
            None => Err(ASCOMError::VALUE_NOT_SET),
        }
    }

    async fn image_ready(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        match *self.state.read().await {
            State::Idle => self
                .last_image
                .read()
                .await
                .clone()
                .map_or_else(|| Ok(false), |_| Ok(true)),
            State::Exposing { .. } => Ok(false),
        }
    }

    async fn last_exposure_start_time(&self) -> ASCOMResult<SystemTime> {
        ensure_connected!(self);
        match *self.last_exposure_start_time.read().await {
            Some(time) => Ok(time),
            None => Err(ASCOMError::VALUE_NOT_SET),
        }
    }

    async fn last_exposure_duration(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        match *self.last_exposure_duration_us.read().await {
            Some(duration) => Ok(duration as f64 / 1_000_000_f64),
            None => Err(ASCOMError::VALUE_NOT_SET),
        }
    }

    async fn max_adu(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device
            .get_parameter(qhyccd_rs::Control::OutputDataActualBits)
            .map_or_else(
                |e| {
                    error!(?e, "could not get OutputDataActualBits");
                    Err(ASCOMError::VALUE_NOT_SET)
                },
                |bits| {
                    debug!(?bits, "ADU");
                    Ok(2_i32.pow(bits as u32))
                },
            )
    }

    async fn camera_xsize(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.ccd_info.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |ccd_info| Ok(ccd_info.image_width as i32),
        )
    }

    async fn camera_ysize(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.ccd_info.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |ccd_info| Ok(ccd_info.image_height as i32),
        )
    }

    async fn start_x(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.intended_roi.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |roi| Ok(roi.start_x as i32),
        )
    }

    async fn set_start_x(&self, start_x: i32) -> ASCOMResult {
        if start_x < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        ensure_connected!(self);
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

    async fn start_y(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.intended_roi.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |roi| Ok(roi.start_y as i32),
        )
    }

    async fn set_start_y(&self, start_y: i32) -> ASCOMResult {
        if start_y < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        ensure_connected!(self);
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

    async fn num_x(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.intended_roi.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |roi| Ok(roi.width as i32),
        )
    }

    async fn set_num_x(&self, num_x: i32) -> ASCOMResult {
        if num_x < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        ensure_connected!(self);
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

    async fn num_y(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.intended_roi.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |roi| Ok(roi.height as i32),
        )
    }

    async fn set_num_y(&self, num_y: i32) -> ASCOMResult {
        if num_y < 0 {
            return Err(ASCOMError::INVALID_VALUE);
        }
        ensure_connected!(self);
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

    async fn percent_completed(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        match *self.state.read().await {
            State::Idle => Ok(100_i32),
            State::Exposing {
                expected_duration_us,
                ..
            } => {
                let Ok(remaining) = self.device.get_remaining_exposure_us() else {
                    error!("get_remaining_exposure_us failed");
                    return Err(ASCOMError::INVALID_OPERATION);
                };

                let res = (100_f64 * remaining as f64 / expected_duration_us as f64) as i32;
                if res > 100_i32 { Ok(100_i32) } else { Ok(res) }
            }
        }
    }

    async fn readout_mode(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device.get_readout_mode().map_or_else(
            |e| {
                error!(?e, "get_readout_mode failed");
                Err(ASCOMError::INVALID_OPERATION)
            },
            |readout_mode| Ok(readout_mode as i32),
        )
    }

    async fn set_readout_mode(&self, readout_mode: i32) -> ASCOMResult {
        let readout_mode = readout_mode as u32;
        ensure_connected!(self);
        let number = self.device.get_number_of_readout_modes().map_err(|e| {
            error!(?e, "get_number_of_readout_modes failed");
            ASCOMError::INVALID_VALUE
        })?;
        if !(0..number).contains(&readout_mode) {
            error!(
                "readout_mode {} is greater than number of readout modes {}",
                readout_mode, number
            );
            return Err(ASCOMError::INVALID_VALUE);
        }
        let (width, height) = self
            .device
            .get_readout_mode_resolution(readout_mode)
            .map_err(|e| {
                error!(?e, "get_readout_mode_resolution failed");
                ASCOMError::INVALID_VALUE
            })?;
        self.device.set_readout_mode(readout_mode).map_err(|e| {
            error!(?e, "set_readout_mode failed");
            ASCOMError::VALUE_NOT_SET
        })?;
        let mut lock = self.ccd_info.write().await;
        *lock = lock.map(|ccd_info| CCDChipInfo {
            image_width: width,
            image_height: height,
            ..ccd_info
        });
        Ok(())
    }

    async fn readout_modes(&self) -> ASCOMResult<Vec<String>> {
        ensure_connected!(self);
        let number = self.device.get_number_of_readout_modes().map_err(|e| {
            error!(?e, "get_number_of_readout_modes failed");
            ASCOMError::INVALID_OPERATION
        })?;
        let mut readout_modes = Vec::with_capacity(number as usize);
        for i in 0..number {
            let readout_mode = self.device.get_readout_mode_name(i).map_err(|e| {
                error!(?e, "get_readout_mode failed");
                ASCOMError::INVALID_OPERATION
            })?;
            readout_modes.push(readout_mode);
        }
        Ok(readout_modes)
    }

    async fn sensor_type(&self) -> ASCOMResult<SensorType> {
        //see here: https://ascom-standards.org/api/#/Camera%20Specific%20Methods/get_camera__device_number__imagearray
        ensure_connected!(self);
        if self
            .device
            .is_control_available(qhyccd_rs::Control::CamIsColor)
            .is_none()
        {
            error!("CamIsColor not available");
            return Ok(SensorType::Monochrome);
        };
        self.device
            .is_control_available(qhyccd_rs::Control::CamColor)
            .map_or_else(
                || {
                    error!("invalid bayer_id from camera");
                    Err(ASCOMError::INVALID_VALUE)
                },
                |_| Ok(SensorType::RGGB),
            )
    }

    #[instrument(level = "trace")]
    async fn start_exposure(&self, duration: f64, light: bool) -> ASCOMResult {
        if duration < 0.0 {
            return Err(ASCOMError::invalid_value("duration must be >= 0"));
        }
        if !light {
            return Err(ASCOMError::invalid_operation("dark frames not supported"));
        }
        ensure_connected!(self);
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
        let Some(roi) = *self.intended_roi.read().await else {
            debug!("no roi defined, but trying to start exposure");
            return Err(ASCOMError::invalid_value("no ROI defined for camera"));
        };
        self.device.set_roi(roi).map_err(|e| {
            debug!(?e, "failed to set ROI");
            ASCOMError::invalid_value("failed to set ROI")
        })?;
        let exposure_us = (duration * 1_000_000_f64) as u32;
        let (stop_tx, mut stop_rx) = oneshot::channel::<StopExposure>();
        let (done_tx, done_rx) = watch::channel(false);

        let mut lock = self.state.write().await;
        *lock = match *lock {
            State::Idle => State::Exposing {
                start: SystemTime::now(),
                expected_duration_us: exposure_us,
                stop_tx: Some(stop_tx),
                done_rx,
            },
            State::Exposing { .. } => {
                error!("camera already exposing");
                return Err(ASCOMError::INVALID_OPERATION);
            }
        };
        drop(lock);

        *self.last_exposure_start_time.write().await = Some(SystemTime::now());
        *self.last_exposure_duration_us.write().await = Some(exposure_us);

        self.device
            .set_parameter(qhyccd_rs::Control::Exposure, exposure_us as f64)
            .map_err(|e| {
                error!(?e, "failed to set exposure time: {:?}", e);
                ASCOMError::INVALID_OPERATION
            })?;

        let device = self.device.clone();
        // Create separate device instance for abort to ensure proper SDK synchronization
        // According to SDK docs, after successful abort we must complete data exchange
        let device_for_abort = self.device.clone();
        let state = self.state.clone();
        let last_image = self.last_image.clone();

        tokio::spawn(async move {
            debug!("DEBUG: New implementation started");
            // Helper function to handle abort and data exchange
            let handle_abort = || async {
                debug!("DEBUG: Handling abort");
                match device_for_abort.abort_exposure_and_readout() {
                    Ok(()) => {
                        debug!("abort succeeded, completing data exchange for sync");
                        if let Ok(buffer_size) = device_for_abort.get_image_size() {
                            if let Ok(image) = device_for_abort.get_single_frame(buffer_size) {
                                match QhyccdCamera::transform_image_static(image) {
                                    Ok(transformed) => {
                                        *last_image.write().await = Some(transformed);
                                        debug!("aborted exposure data stored");
                                    }
                                    Err(e) => error!(?e, "failed to transform aborted image"),
                                }
                            }
                        }
                    }
                    Err(e) => error!(?e, "failed to abort exposure"),
                }
                debug!("exposure aborted");
            };

            // Execute start_single_frame_exposure
            let start_task = task::spawn_blocking({
                let device = device.clone();
                move || {
                    device.start_single_frame_exposure().map_err(|e| {
                        error!(?e, "failed to start exposure: {:?}", e);
                        ASCOMError::INVALID_OPERATION
                    })
                }
            });

            match start_task.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!(?e, "start exposure failed");
                    *state.write().await = State::Idle;
                    return;
                }
                Err(e) => {
                    error!(?e, "start task failed");
                    *state.write().await = State::Idle;
                    return;
                }
            }

            // Check for abort after start_single_frame_exposure
            debug!("DEBUG: Checking for abort after start_single_frame_exposure");
            match stop_rx.try_recv() {
                Ok(_) => {
                    debug!("DEBUG: Abort detected after start_single_frame_exposure!");
                    handle_abort().await;
                    *state.write().await = State::Idle;
                    return;
                }
                Err(e) => {
                    debug!("DEBUG: No abort signal: {:?}", e);
                }
            }

            // Execute get_image_size
            let size_task = task::spawn_blocking({
                let device = device.clone();
                move || {
                    device.get_image_size().map_err(|e| {
                        error!(?e, "get_image_size failed");
                        ASCOMError::INVALID_OPERATION
                    })
                }
            });

            let buffer_size = match size_task.await {
                Ok(Ok(size)) => {
                    debug!(?size);
                    size
                }
                Ok(Err(e)) => {
                    error!(?e, "get image size failed");
                    *state.write().await = State::Idle;
                    return;
                }
                Err(e) => {
                    error!(?e, "size task failed");
                    *state.write().await = State::Idle;
                    return;
                }
            };

            // Check for abort after get_image_size
            if stop_rx.try_recv().is_ok() {
                handle_abort().await;
                *state.write().await = State::Idle;
                return;
            }

            // Execute get_single_frame
            let image_task = task::spawn_blocking({
                move || {
                    device.get_single_frame(buffer_size).map_err(|e| {
                        error!(?e, "get_single_frame failed");
                        ASCOMError::INVALID_OPERATION
                    })
                }
            });

            let image = match image_task.await {
                Ok(Ok(image)) => image,
                Ok(Err(e)) => {
                    error!(?e, "get single frame failed");
                    *state.write().await = State::Idle;
                    return;
                }
                Err(e) => {
                    error!(?e, "image task failed");
                    *state.write().await = State::Idle;
                    return;
                }
            };

            // Transform and store the image
            match QhyccdCamera::transform_image_static(image) {
                Ok(transformed) => {
                    *last_image.write().await = Some(transformed);
                    let _ = done_tx.send(true);
                    debug!("exposure completed successfully");
                }
                Err(e) => error!(?e, "failed to transform image"),
            }

            *state.write().await = State::Idle;
        });

        Ok(())
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
                    Err(ASCOMError::INVALID_OPERATION)
                }
            },
            _ => {
                error!("camera not connected");
                return Err(ASCOMError::NOT_CONNECTED);
            }
        } */
    }

    async fn abort_exposure(&self) -> ASCOMResult {
        ensure_connected!(self);

        let mut state_lock = self.state.write().await;
        match &mut *state_lock {
            State::Exposing { stop_tx, .. } => {
                if let Some(tx) = stop_tx.take() {
                    let _ = tx.send(StopExposure { _want_image: false });
                    Ok(())
                } else {
                    // Channel already used
                    Err(ASCOMError::INVALID_OPERATION)
                }
            }
            State::Idle => {
                // Nothing to abort
                Ok(())
            }
        }
    }

    async fn pixel_size_x(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        self.ccd_info.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |ccd_info| Ok(ccd_info.pixel_width),
        )
    }

    async fn pixel_size_y(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        self.ccd_info.read().await.map_or_else(
            || Err(ASCOMError::VALUE_NOT_SET),
            |ccd_info| Ok(ccd_info.pixel_height),
        )
    }

    async fn can_get_cooler_power(&self) -> ASCOMResult<bool> {
        self.can_set_ccd_temperature().await
    }

    async fn can_set_ccd_temperature(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .map_or_else(
                || {
                    debug!("no cooler");
                    Ok(false)
                },
                |_| Ok(true),
            )
    }

    async fn ccd_temperature(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .ok_or_else(|| {
                debug!("no cooler");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        self.device
            .get_parameter(qhyccd_rs::Control::CurTemp)
            .map_err(|e| {
                error!(?e, "could not get current temperature");
                ASCOMError::INVALID_VALUE
            })
    }

    async fn set_ccd_temperature(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .ok_or_else(|| {
                debug!("no cooler");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        match *self.target_temperature.read().await {
            Some(temperature) => Ok(temperature),
            None => self.ccd_temperature().await,
        }
    }

    async fn set_set_ccd_temperature(&self, set_ccd_temperature: f64) -> ASCOMResult {
        //ASCOM checks
        if !(-273.15..=80_f64).contains(&set_ccd_temperature) {
            return Err(ASCOMError::INVALID_VALUE);
        }
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .ok_or_else(|| {
                debug!("no cooler");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        match self
            .device
            .set_parameter(qhyccd_rs::Control::Cooler, set_ccd_temperature)
        {
            Ok(_) => {
                *self.target_temperature.write().await = Some(set_ccd_temperature);
                Ok(())
            }
            Err(e) => {
                error!(?e, "could not set target temperature");
                Err(ASCOMError::INVALID_OPERATION)
            }
        }
    }

    async fn cooler_on(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .ok_or_else(|| {
                debug!("no cooler");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let cooler_power = self
            .device
            .get_parameter(qhyccd_rs::Control::CurPWM)
            .map_err(|e| {
                error!(?e, "could not get current power");
                ASCOMError::INVALID_VALUE
            })?;
        Ok(cooler_power > 0_f64)
    }

    async fn set_cooler_on(&self, cooler_on: bool) -> ASCOMResult {
        if self.cooler_on().await? == cooler_on {
            return Ok(());
        }
        match cooler_on {
            true => self
                .device
                .set_parameter(qhyccd_rs::Control::ManualPWM, 1_f64 / 100_f64 * 255_f64)
                .map_err(|e| {
                    error!(?e, "error setting cooler power to 1");
                    ASCOMError::INVALID_OPERATION
                }),
            false => self
                .device
                .set_parameter(qhyccd_rs::Control::ManualPWM, 0_f64)
                .map_err(|e| {
                    error!(?e, "error setting cooler power to 0");
                    ASCOMError::INVALID_OPERATION
                }),
        }
    }

    async fn cooler_power(&self) -> ASCOMResult<f64> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Cooler)
            .ok_or_else(|| {
                debug!("no cooler");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        self.device
            .get_parameter(qhyccd_rs::Control::CurPWM)
            .map_or_else(
                |e| {
                    error!(?e, "could not get current temperature");
                    Err(ASCOMError::INVALID_VALUE)
                },
                |cooler_power| Ok(cooler_power / 255_f64 * 100_f64),
            )
    }

    async fn gain(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Gain)
            .ok_or_else(|| {
                debug!("gain control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        self.device
            .get_parameter(qhyccd_rs::Control::Gain)
            .map_or_else(
                |e| {
                    error!(?e, "failed to set gain");
                    Err(ASCOMError::INVALID_OPERATION)
                },
                |gain| Ok(gain as i32),
            )
    }

    async fn set_gain(&self, gain: i32) -> ASCOMResult {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Gain)
            .ok_or_else(|| {
                debug!("gain control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let (min, max) = self
                        .gain_min_max
                        .read()
                        .await
                        .ok_or(ASCOMError::invalid_operation("camera reports gain control available, but min, max values are not set after initialization"))?;
        if !(min as i32..=max as i32).contains(&gain) {
            return Err(ASCOMError::INVALID_VALUE);
        }
        self.device
            .set_parameter(qhyccd_rs::Control::Gain, gain as f64)
            .map_err(|e| {
                error!(?e, "failed to set gain");
                ASCOMError::INVALID_OPERATION
            })
    }

    async fn gain_max(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.gain_min_max
            .read()
            .await
            .map(|(_min, max)| max as i32)
            .ok_or(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn gain_min(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.gain_min_max
            .read()
            .await
            .map(|(min, _max)| min as i32)
            .ok_or(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn offset(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Offset)
            .ok_or_else(|| {
                debug!("offset control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        self.device
            .get_parameter(qhyccd_rs::Control::Offset)
            .map_or_else(
                |e| {
                    error!(?e, "failed to get offset");
                    Err(ASCOMError::INVALID_OPERATION)
                },
                |offset| Ok(offset as i32),
            )
    }

    async fn set_offset(&self, offset: i32) -> ASCOMResult {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Offset)
            .ok_or_else(|| {
                debug!("offset control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let (min, max) = self
                        .offset_min_max
                        .read()
                        .await
                        .ok_or(ASCOMError::invalid_operation("camera reports offset control available, but min, max values are not set after initialization"))?;
        if !(min as i32..=max as i32).contains(&offset) {
            return Err(ASCOMError::INVALID_VALUE);
        }
        self.device
            .set_parameter(qhyccd_rs::Control::Offset, offset as f64)
            .map_err(|e| {
                error!(?e, "failed to set offset");
                ASCOMError::INVALID_OPERATION
            })
    }

    async fn offset_max(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.offset_min_max
            .read()
            .await
            .map(|(_min, max)| max as i32)
            .ok_or(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn offset_min(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        self.offset_min_max
            .read()
            .await
            .map(|(min, _max)| min as i32)
            .ok_or(ASCOMError::NOT_IMPLEMENTED)
    }

    async fn can_fast_readout(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        // Return true if both Speed control is available AND we have valid min/max/step values
        Ok(self
            .device
            .is_control_available(qhyccd_rs::Control::Speed)
            .is_some()
            && self.readout_speed_min_max_step.read().await.is_some())
    }

    async fn fast_readout(&self) -> ASCOMResult<bool> {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Speed)
            .ok_or_else(|| {
                debug!("readout speed control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let speed = self
            .device
            .get_parameter(qhyccd_rs::Control::Speed)
            .map_err(|e| {
                error!(?e, "failed to get speed value");
                ASCOMError::INVALID_OPERATION
            })?;
        let (_min, max, _step) = self
            .readout_speed_min_max_step
            .read()
            .await
            .ok_or_else(|| {
                error!("readout speed available, but min, max not set");
                ASCOMError::INVALID_OPERATION
            })?;
        if (speed - max).abs() < f64::EPSILON {
            return Ok(true);
        };
        Ok(false)
    }

    async fn set_fast_readout(&self, fast_readout: bool) -> ASCOMResult {
        ensure_connected!(self);
        self.device
            .is_control_available(qhyccd_rs::Control::Speed)
            .ok_or_else(|| {
                debug!("readout speed control not available");
                ASCOMError::NOT_IMPLEMENTED
            })?;
        let (min, max, _step) = self
                        .readout_speed_min_max_step
                        .read()
                        .await
                        .ok_or(ASCOMError::invalid_operation("camera reports readout speed control available, but min, max values are not set after initialization"))?;
        let speed = match fast_readout {
            true => max,
            false => min,
        };
        self.device
            .set_parameter(qhyccd_rs::Control::Speed, speed)
            .map_err(|e| {
                error!(?e, "failed to set speed");
                ASCOMError::INVALID_OPERATION
            })
    }
}

#[derive(Debug)]
struct QhyccdFilterWheel {
    unique_id: String,
    name: String,
    description: String,
    number_of_filters: RwLock<Option<u32>>,
    target_position: RwLock<Option<u32>>,
    device: QhyFilterWheel,
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
        if self.connected().await? == connected {
            return Ok(());
        };
        match connected {
            true => {
                self.device.open().map_err(|e| {
                    error!(?e, "open failed");
                    ASCOMError::NOT_CONNECTED
                })?;
                let mut lock = self.number_of_filters.write().await;
                let number_of_filters = self.device.get_number_of_filters().map_err(|e| {
                    error!(?e, "get_number_of_filters failed");
                    ASCOMError::NOT_CONNECTED
                })?;
                *lock = Some(number_of_filters);
                let mut lock = self.target_position.write().await;
                let target_position = self.device.get_fw_position().map_err(|e| {
                    error!(?e, "get_fw_position failed");
                    ASCOMError::NOT_CONNECTED
                })?;
                *lock = Some(target_position);
                Ok(())
            }
            false => self.device.close().map_err(|e| {
                error!(?e, "close_camera failed");
                ASCOMError::NOT_CONNECTED
            }),
        }
    }

    async fn description(&self) -> ASCOMResult<String> {
        Ok(self.description.clone())
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("qhyccd-alpaca See: https://crates.io/crates/qhyccd-alpaca".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl FilterWheel for QhyccdFilterWheel {
    /// An integer array of filter focus offsets.
    async fn focus_offsets(&self) -> ASCOMResult<Vec<i32>> {
        ensure_connected!(self);
        let Some(number_of_filters) = *self.number_of_filters.read().await else {
            error!("number of filters not set, but filter wheel connected");
            return Err(ASCOMError::NOT_CONNECTED);
        };
        Ok(vec![0; number_of_filters as usize])
    }

    /// The names of the filters
    async fn names(&self) -> ASCOMResult<Vec<String>> {
        ensure_connected!(self);
        let Some(number_of_filters) = *self.number_of_filters.read().await else {
            error!("number of filters not set, but filter wheel connected");
            return Err(ASCOMError::NOT_CONNECTED);
        };
        let mut names = Vec::with_capacity(number_of_filters as usize);
        for i in 0..number_of_filters {
            names.push(format!("Filter{}", i));
        }
        Ok(names)
    }
    /// Returns the current filter wheel position
    async fn position(&self) -> ASCOMResult<i32> {
        ensure_connected!(self);
        let Some(target_position) = *self.target_position.read().await else {
            error!("target_position not set, but filter wheel connected");
            return Err(ASCOMError::NOT_CONNECTED);
        };
        let actual = self.device.get_fw_position().map_err(|e| {
            error!(?e, "get_fw_position failed");
            ASCOMError::INVALID_OPERATION
        })?;
        match actual == target_position {
            true => Ok(actual as i32),
            false => {
                trace!(
                    "position - target_position set to {}, but filter wheel is at {}",
                    target_position, actual
                );
                Ok(-1)
            }
        }
    }

    /// Sets the filter wheel position
    async fn set_position(&self, position: i32) -> ASCOMResult {
        ensure_connected!(self);
        let Some(number_of_filters) = *self.number_of_filters.read().await else {
            error!("number of filters not set, but filter wheel connected");
            return Err(ASCOMError::NOT_CONNECTED);
        };
        if !(0..number_of_filters as i32).contains(&position) {
            return Err(ASCOMError::INVALID_VALUE);
        }
        let mut lock = self.target_position.write().await;
        if lock.is_some_and(|target_position| target_position == position as u32) {
            return Ok(());
        }
        self.device.set_fw_position(position as u32).map_or_else(
            |e| {
                error!(?e, "set_fw_position failed");
                Err(ASCOMError::INVALID_OPERATION)
            },
            |_| {
                *lock = Some(position as u32);
                Ok(())
            },
        )
    }
}

use clap::Parser;

/// ASCOM Alpaca server for QHYCCD cameras and filter wheels
#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// valid values: trace, debug, info, warn, error
    #[arg(short, long, default_value = "info")]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    let args = Args::parse();
    let log_level = args
        .log_level
        .unwrap_or_else(|| std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()));
    let port = args.port;

    match log_level.as_str() {
        "trace" => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .finish(),
        )?,
        "debug" => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )?,
        "info" => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .finish(),
        )?,
        "warn" => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::WARN)
                .finish(),
        )?,
        "error" => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::ERROR)
                .finish(),
        )?,
        _ => {
            eprintln!("Invalid log level: {}", log_level);
            std::process::exit(1);
        }
    }

    let mut server = Server::new(CargoServerInfo!());

    server.listen_addr.set_port(port);

    let sdk = Sdk::new().expect("SDK::new failed");
    let sdk_version = sdk.version().expect("get_sdk_version failed");
    trace!(sdk_version = ?sdk_version);
    trace!(cameras = ?sdk.cameras().count());
    trace!(filter_wheels = ?sdk.filter_wheels().count());

    sdk.cameras().for_each(|c| {
        let camera = QhyccdCamera {
            unique_id: c.id().to_owned(),
            name: c.id().to_owned(),
            description: "QHYCCD camera".to_owned(),
            device: c.clone(),
            binning: RwLock::new(1_u32),
            valid_bins: RwLock::new(None),
            target_temperature: RwLock::new(None),
            ccd_info: RwLock::new(None),
            intended_roi: RwLock::new(None),
            readout_speed_min_max_step: RwLock::new(None),
            exposure_min_max_step: RwLock::new(None),
            last_exposure_start_time: RwLock::new(None),
            last_exposure_duration_us: RwLock::new(None),
            last_image: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(State::Idle)),
            gain_min_max: RwLock::new(None),
            offset_min_max: RwLock::new(None),
        };
        tracing::debug!(?camera, "Registering camera");
        server
            .devices
            .register::<dyn ascom_alpaca::api::Camera>(camera);
    });

    sdk.filter_wheels().for_each(|c| {
        let filter_wheel = QhyccdFilterWheel {
            unique_id: format!("CFW={}", c.id()),
            name: format!("CFW={}", c.id()),
            description: "QHYCCD filter wheel".to_owned(),
            number_of_filters: RwLock::new(None),
            target_position: RwLock::new(None),
            device: c.clone(),
        };
        tracing::debug!(?filter_wheel, "Registering filter wheel");
        server
            .devices
            .register::<dyn ascom_alpaca::api::FilterWheel>(filter_wheel);
    });

    server.start().await
}

#[cfg(test)]
mod test_camera;
#[cfg(test)]
mod test_filter_wheel;
