//! Camera test utilities and modules
#![allow(clippy::too_many_arguments)]

use std::time::Duration;
use std::vec;

use qhyccd_rs::Control;

use crate::mocks::MockCamera;
use crate::*;
use eyre::eyre;
use ndarray::{Array3, array};

use rstest::*;

// Test modules
pub mod binning;
pub mod connection;
pub mod exposure;
pub mod gain_offset;
pub mod image;
pub mod properties;
pub mod roi;
pub mod sensor;
pub mod temperature;

/// Macro for testing NOT_CONNECTED error responses
#[macro_export]
macro_rules! not_connected {
    ($name:ident$tail:tt) => {
        let mock = MockCamera::new();
        let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
        let res = camera.$name$tail.await;
        assert_eq!(
            res.err().unwrap().to_string(),
            ASCOMError::NOT_CONNECTED.to_string(),
        );
    };
}

/// Mock camera configuration variants for test setup
pub enum MockCameraType {
    IsOpenTrue {
        times: usize,
    },
    IsOpenFalse {
        times: usize,
    },
    WithCCDInfo {
        times: usize,
        camera_ccd_info: Option<CCDChipInfo>,
    },
    WithRoi {
        times: usize,
        camera_roi: Option<CCDChipArea>,
    },
    WithState {
        times: usize,
        state: State,
    },
    Untouched,
    WithStateExposing {
        expected_duration: f64,
    },
    WithImage {
        image_array: ImageArray,
    },
    WithExposureMinMaxStep {
        min_max_step: Option<(f64, f64, f64)>,
    },
    WithLastExposureStart {
        start_time: Option<SystemTime>,
    },
    WithLastExposureDuration {
        duration: Option<u32>,
    },
    WithBinningAndValidBins {
        times: usize,
        camera_valid_bins: Vec<u8>,
        camera_binning: u8,
    },
    WithBinningAndRoiAndCCDInfo {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: u8,
    },
    WithBinningAndValidBinsAndRoiAndCCDInfo {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: u8,
        camera_valid_bins: Vec<u8>,
    },
    WithBinningAndRoiAndCCDInfoUnlimited {
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: u8,
    },
    WithBinningAndRoiAndCCDInfoAndExposing {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: u8,
        expected_duration: f64,
    },
    WithTargetTemperature {
        times: usize,
        temperature: Option<f64>,
    },
    WithGain {
        times: usize,
        min_max: Option<(f64, f64)>,
    },
    WithOffset {
        times: usize,
        min_max: Option<(f64, f64)>,
    },
    WithReadoutMinMax {
        times: usize,
        min_max_step: Option<(f64, f64, f64)>,
    },
}

/// Creates a new QhyccdCamera with the specified mock configuration
pub fn new_camera(mut device: MockCamera, variant: MockCameraType) -> QhyccdCamera {
    let mut valid_bins = RwLock::new(None);
    let mut binning = RwLock::new(0_u8);
    let mut target_temperature = RwLock::new(None);
    let mut ccd_info = RwLock::new(None);
    let mut intended_roi = RwLock::new(None);
    let mut exposing = RwLock::new(State::Idle);
    let mut readout_speed_min_max_step = RwLock::new(None);
    let mut exposure_min_max_step = RwLock::new(None);
    let mut last_exposure_start_time = Arc::new(RwLock::new(None));
    let mut last_exposure_duration_us = Arc::new(RwLock::new(None));
    let mut last_image = RwLock::new(None);
    let mut gain_min_max = RwLock::new(None);
    let mut offset_min_max = RwLock::new(None);
    match variant {
        MockCameraType::IsOpenTrue { times } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
        }
        MockCameraType::IsOpenFalse { times } => {
            device.expect_is_open().times(times).returning(|| Ok(false));
        }
        MockCameraType::WithCCDInfo {
            times,
            camera_ccd_info,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            ccd_info = RwLock::new(camera_ccd_info);
        }
        MockCameraType::WithRoi { times, camera_roi } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            intended_roi = RwLock::new(camera_roi);
        }
        MockCameraType::WithState {
            times,
            state: camera_state,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            exposing = RwLock::new(camera_state);
        }
        MockCameraType::Untouched => {}
        MockCameraType::WithStateExposing { expected_duration } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            exposing = RwLock::new(State::Exposing {
                start: SystemTime::UNIX_EPOCH,
                expected_duration_us: expected_duration as u32,
                stop_tx: None,
                done_rx: watch::channel(false).1,
            });
        }
        MockCameraType::WithImage { image_array: image } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_image = RwLock::new(Some(image));
        }
        MockCameraType::WithExposureMinMaxStep { min_max_step } => {
            device.expect_is_open().once().returning(|| Ok(true));
            exposure_min_max_step = RwLock::new(min_max_step);
        }
        MockCameraType::WithLastExposureStart { start_time } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_start_time = Arc::new(RwLock::new(start_time));
        }
        MockCameraType::WithLastExposureDuration { duration } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_duration_us = Arc::new(RwLock::new(duration));
        }
        MockCameraType::WithBinningAndValidBins {
            times,
            camera_valid_bins,
            camera_binning,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            valid_bins = RwLock::new(Some(camera_valid_bins));
            binning = RwLock::new(camera_binning);
        }
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times,
            camera_roi,
            camera_ccd_info,
            camera_binning,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            ccd_info = RwLock::new(Some(camera_ccd_info));
            intended_roi = RwLock::new(Some(camera_roi));
            binning = RwLock::new(camera_binning);
        }
        MockCameraType::WithBinningAndValidBinsAndRoiAndCCDInfo {
            times,
            camera_roi,
            camera_ccd_info,
            camera_binning,
            camera_valid_bins,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            ccd_info = RwLock::new(Some(camera_ccd_info));
            intended_roi = RwLock::new(Some(camera_roi));
            valid_bins = RwLock::new(Some(camera_valid_bins));
            binning = RwLock::new(camera_binning);
        }
        MockCameraType::WithBinningAndRoiAndCCDInfoUnlimited {
            camera_roi,
            camera_ccd_info,
            camera_binning,
        } => {
            device.expect_is_open().returning(|| Ok(true));
            ccd_info = RwLock::new(Some(camera_ccd_info));
            intended_roi = RwLock::new(Some(camera_roi));
            binning = RwLock::new(camera_binning);
        }
        MockCameraType::WithBinningAndRoiAndCCDInfoAndExposing {
            times,
            camera_roi,
            camera_ccd_info,
            camera_binning,
            expected_duration,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            ccd_info = RwLock::new(Some(camera_ccd_info));
            intended_roi = RwLock::new(Some(camera_roi));
            binning = RwLock::new(camera_binning);
            exposing = RwLock::new(State::Exposing {
                start: SystemTime::UNIX_EPOCH,
                expected_duration_us: expected_duration as u32,
                stop_tx: None,
                done_rx: watch::channel(false).1,
            });
        }
        MockCameraType::WithTargetTemperature { times, temperature } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            target_temperature = RwLock::new(temperature);
        }
        MockCameraType::WithGain { times, min_max } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            gain_min_max = RwLock::new(min_max);
        }
        MockCameraType::WithOffset { times, min_max } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            offset_min_max = RwLock::new(min_max);
        }
        MockCameraType::WithReadoutMinMax {
            times,
            min_max_step,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            readout_speed_min_max_step = RwLock::new(min_max_step);
        }
    }
    QhyccdCamera {
        unique_id: "test-camera".to_owned(),
        name: "QHYCCD-test_camera".to_owned(),
        description: "QHYCCD camera".to_owned(),
        device,
        binning,
        valid_bins,
        target_temperature,
        ccd_info,
        intended_roi,
        readout_speed_min_max_step,
        exposure_min_max_step,
        last_exposure_start_time,
        last_exposure_duration_us,
        last_image: Arc::new(last_image),
        state: Arc::new(exposing),
        gain_min_max,
        offset_min_max,
    }
}
