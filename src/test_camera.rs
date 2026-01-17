#![allow(clippy::too_many_arguments)]
use std::time::Duration;
use std::vec;

use qhyccd_rs::Control;

use super::*;
use crate::mocks::MockCamera;
use eyre::eyre;
use ndarray::{Array3, array};

use rstest::*;

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

#[tokio::test]
async fn not_connected_asyncs() {
    not_connected! {sensor_type()}
    not_connected! {max_bin_x()}
    not_connected! {max_bin_y()}
    not_connected! {sensor_name()}
    not_connected! {camera_state()}
    not_connected! {bin_x()}
    not_connected! {bin_y()}
    not_connected! {set_bin_x(1)}
    not_connected! {set_bin_y(1)}
    not_connected! {has_shutter()}
    not_connected! {image_array()}
    not_connected! {image_ready()}
    not_connected! {last_exposure_start_time()}
    not_connected! {last_exposure_duration()}
    not_connected! {camera_x_size()}
    not_connected! {camera_y_size()}
    not_connected! {start_x()}
    not_connected! {set_start_x(100)}
    not_connected! {start_y()}
    not_connected! {set_start_y(100)}
    not_connected! {num_x()}
    not_connected! {set_num_x(100)}
    not_connected! {num_y()}
    not_connected! {set_num_y(100)}
    not_connected! {readout_mode()}
    not_connected! {set_readout_mode(1)}
    not_connected! {readout_modes()}
    not_connected! {percent_completed()}
    not_connected! {start_exposure(Duration::from_secs_f64(1.0), true)}
    not_connected! {max_adu()}
    //not_connected! {stop_exposure()}
    not_connected! {abort_exposure()}
    not_connected! {pixel_size_x()}
    not_connected! {pixel_size_y()}
    not_connected! {can_get_cooler_power()}
    not_connected! {ccd_temperature()}
    not_connected! {set_ccd_temperature()}
    not_connected! {set_set_ccd_temperature(0.0)}
    not_connected! {cooler_on()}
    not_connected! {set_cooler_on(true)}
    not_connected! {cooler_power()}
    not_connected! {exposure_min()}
    not_connected! {exposure_max()}
    not_connected! {exposure_resolution()}
    not_connected! {gain()}
    not_connected! {set_gain(1)}
    not_connected! {gain_min()}
    not_connected! {gain_max()}
    not_connected! {offset()}
    not_connected! {set_offset(10)}
    not_connected! {offset_min()}
    not_connected! {offset_max()}
    not_connected! {bayer_offset_x()}
    not_connected! {bayer_offset_y()}
    not_connected! {can_fast_readout()}
    not_connected! {fast_readout()}
    not_connected! {set_fast_readout(true)}
}

enum MockCameraType {
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

fn new_camera(mut device: MockCamera, variant: MockCameraType) -> QhyccdCamera {
    let mut valid_bins = RwLock::new(None);
    let mut binning = RwLock::new(0_u8);
    let mut target_temperature = RwLock::new(None);
    let mut ccd_info = RwLock::new(None);
    let mut intended_roi = RwLock::new(None);
    let mut exposing = RwLock::new(State::Idle);
    let mut readout_speed_min_max_step = RwLock::new(None);
    let mut exposure_min_max_step = RwLock::new(None);
    let mut last_exposure_start_time = RwLock::new(None);
    let mut last_exposure_duration_us = RwLock::new(None);
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
            last_exposure_start_time = RwLock::new(start_time);
        }
        MockCameraType::WithLastExposureDuration { duration } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_duration_us = RwLock::new(duration);
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

#[tokio::test]
async fn qhyccd_camera() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_id()
        .times(2)
        .return_const("test_camera".to_owned());
    mock.expect_clone().returning(MockCamera::new);
    //when
    let camera = QhyccdCamera {
        unique_id: mock.id().to_owned(),
        name: format!("QHYCCD-{}", mock.id()),
        description: "QHYCCD camera".to_owned(),
        device: mock.clone(),
        binning: RwLock::new(1_u8),
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
    //then
    assert_eq!(camera.unique_id, "test_camera");
    assert_eq!(camera.name, "QHYCCD-test_camera");
    assert_eq!(camera.description, "QHYCCD camera");
    assert_eq!(*camera.binning.read().await, 1);
    assert!(camera.valid_bins.read().await.is_none());
    assert!(camera.intended_roi.read().await.is_none());
    assert!(camera.last_exposure_start_time.read().await.is_none());
    assert!(camera.last_exposure_duration_us.read().await.is_none());
    assert!(camera.last_image.read().await.is_none());
    assert_eq!(*camera.state.read().await, State::Idle);
    assert_eq!(camera.static_name(), "QHYCCD-test_camera");
    assert_eq!(camera.unique_id(), "test_camera");
    assert_eq!(camera.description().await.unwrap(), "QHYCCD camera");
    assert_eq!(
        camera.driver_info().await.unwrap(),
        "qhyccd-alpaca See: https://crates.io/crates/qhyccd-alpaca"
    );
    assert_eq!(
        camera.driver_version().await.unwrap(),
        env!("CARGO_PKG_VERSION")
    );
}

#[rstest]
#[case(true, Ok(8_u8), Ok(8_u8))]
#[case(
    false,
    Err(ASCOMError::INVALID_OPERATION),
    Err(ASCOMError::INVALID_OPERATION)
)]
#[tokio::test]
async fn max_bin_xy(
    #[case] has_modes: bool,
    #[case] expected_x: ASCOMResult<u8>,
    #[case] expected_y: ASCOMResult<u8>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(12)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(move |control| {
            if has_modes {
                match control {
                    Control::CamBin1x1mode => Some(0_u32),
                    Control::CamBin2x2mode => Some(0_u32),
                    Control::CamBin3x3mode => Some(0_u32),
                    Control::CamBin4x4mode => Some(0_u32),
                    Control::CamBin6x6mode => Some(0_u32),
                    Control::CamBin8x8mode => Some(0_u32),
                    _ => panic!("Unexpected control"),
                }
            } else {
                None
            }
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.max_bin_x().await;
    //then
    if expected_x.is_ok() {
        assert_eq!(res.unwrap(), expected_x.unwrap());
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected_x.unwrap_err().to_string()
        );
    }

    //when
    let res = camera.max_bin_y().await;
    //then
    if expected_y.is_ok() {
        assert_eq!(res.unwrap(), expected_y.unwrap());
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected_y.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(State::Idle, Ok(CameraState::Idle))]
#[case(State::Exposing{ start: SystemTime::UNIX_EPOCH, expected_duration_us: 1_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(CameraState::Exposing))]
#[tokio::test]
async fn camera_state(#[case] state: State, #[case] expected: ASCOMResult<CameraState>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::WithState { times: 1, state });
    //when
    let res = camera.camera_state().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap())
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected.unwrap_err().to_string()
        );
    }
}

#[tokio::test]
async fn connected_fail() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open()
        .times(1)
        .returning(|| Err(eyre!("Could not acquire read lock on camera handle")));
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.connected().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_already_connected() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_already_disconnected() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    assert!(res.is_ok());
}

#[rustfmt::skip]
#[rstest]
#[case(false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, false, true, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, false, true, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, false, true, false, false, Ok(()))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, false, false, Ok(()))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, Ok(()))]
#[tokio::test]
async fn set_connected_true(
    #[case] open: bool,
    #[case] has_single_frame_mode: bool,
    #[case] set_stream_mode: bool,
    #[case] set_readout_mode: bool,
    #[case] init: bool,
    #[case] transfer_bit: bool,
    #[case] ccd_info: bool,
    #[case] effective_area: bool,
    #[case] has_bin_modes: bool,
    #[case] has_speed_control: bool,
    #[case] speed_min_max: bool,
    #[case] exposure_min_max: bool,
    #[case] has_gain_control: bool,
    #[case] gain_min_max: bool,
    #[case] has_offset_control: bool,
    #[case] offset_min_max: bool,
    #[case] expected: ASCOMResult,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open()
        .once()
        .returning(move || if open { Ok(()) } else { Err(eyre!("error")) });
    mock.expect_is_control_available()
        .times(if open { 1 } else { 0 })
        .withf(|control| *control == qhyccd_rs::Control::CamSingleFrameMode)
        .returning(move |_| {
            if has_single_frame_mode {
                Some(0_u32)
            } else {
                None
            }
        });
    mock.expect_set_stream_mode()
        .times(if has_single_frame_mode { 1 } else { 0 })
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(move |_| {
            if set_stream_mode {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_set_readout_mode()
        .times(if set_stream_mode { 1 } else { 0 })
        .withf(|mode| *mode == 0)
        .returning(move |_| {
            if set_readout_mode {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_init()
        .times(if set_readout_mode { 1 } else { 0 })
        .returning(move || if init { Ok(()) } else { Err(eyre!("error")) });
    mock.expect_set_if_available()
        .times(if init { 1 } else { 0 })
        .withf(|control, bits| *control == qhyccd_rs::Control::TransferBit && *bits == 16_f64)
        .returning(move |_, _| {
            if transfer_bit {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_ccd_info()
        .times(if transfer_bit { 1 } else { 0 })
        .returning(move || {
            if ccd_info {
                Ok(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.9,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_effective_area()
        .times(if ccd_info { 1 } else { 0 })
        .returning(move || {
            if effective_area {
                Ok(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if effective_area { 6 } else { 0 })
        .withf(move |control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(move |control| {
            if has_bin_modes {
                match control {
                    Control::CamBin1x1mode => Some(0_u32),
                    Control::CamBin2x2mode => Some(0_u32),
                    Control::CamBin3x3mode => Some(0_u32),
                    Control::CamBin4x4mode => Some(0_u32),
                    Control::CamBin6x6mode => Some(0_u32),
                    Control::CamBin8x8mode => Some(0_u32),
                    _ => panic!("Unexpected control"),
                }
            } else {
                None
            }
        });
    mock.expect_is_control_available()
        .times(if effective_area { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| if has_speed_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_speed_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| {
            if speed_min_max {
                Ok((0_f64, 255_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_parameter_min_max_step()
        .times(if speed_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Exposure)
        .returning(move |_| {
            if exposure_min_max {
                Ok((1_f64, 3_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if exposure_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| if has_gain_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_gain_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| {
            if gain_min_max {
                Ok((0_f64, 51_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if gain_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| if has_offset_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_offset_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| {
            if offset_min_max {
                Ok((0_f64, 1023_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    if expected.is_ok() {
        assert!(res.is_ok())
    } else {
        assert_eq!(
            expected.unwrap_err().to_string(),
            res.unwrap_err().to_string()
        )
    }
}

#[rstest]
#[case(Ok(()), Ok(()))]
#[case(Err(eyre!("error")), Err(ASCOMError::NOT_CONNECTED))]
#[tokio::test]
async fn set_connected_false_success(#[case] close: Result<()>, #[case] expected: ASCOMResult) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_close().once().return_once(move || close);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    if expected.is_ok() {
        assert!(res.is_ok())
    } else {
        assert_eq!(
            expected.unwrap_err().to_string(),
            res.unwrap_err().to_string()
        )
    }
}

// https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
#[rustfmt::skip]
#[rstest]
#[case(Some(0), Some(qhyccd_rs::BayerMode::GBRG as u32), 2, Ok(0_u8), Ok(1_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::GRBG as u32), 2, Ok(1_u8), Ok(0_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::BGGR as u32), 2, Ok(1_u8), Ok(1_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::RGGB as u32), 2, Ok(0_u8), Ok(0_u8))]
#[case(None, Some(qhyccd_rs::BayerMode::RGGB as u32), 0, Err(ASCOMError::NOT_IMPLEMENTED), Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(Some(0), Some(0_u32), 2, Err(ASCOMError::INVALID_VALUE), Err(ASCOMError::INVALID_VALUE))]
#[case(Some(0), None, 2, Err(ASCOMError::INVALID_VALUE), Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn bayer_offset(
    #[case] cam_is_color: Option<u32>,
    #[case] cam_color: Option<u32>,
    #[case] cam_color_times: usize,
    #[case] expected_x: ASCOMResult<u8>,
    #[case] expected_y: ASCOMResult<u8>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(move |control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(move |_| cam_is_color);
    mock.expect_is_control_available()
        .times(cam_color_times)
        .withf(move |control| *control == qhyccd_rs::Control::CamColor)
        .returning(move |_| cam_color);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res_x = camera.bayer_offset_x().await;
    let res_y = camera.bayer_offset_y().await;
    //then
    if expected_x.is_ok() {
        assert_eq!(res_x.unwrap(), expected_x.unwrap());
        assert_eq!(res_y.unwrap(), expected_y.unwrap());
    } else {
        assert_eq!(
            expected_x.unwrap_err().to_string(),
            res_x.unwrap_err().to_string()
        );
        assert_eq!(
            expected_y.unwrap_err().to_string(),
            res_y.unwrap_err().to_string()
        )
    }
}

#[tokio::test]
async fn sensor_name_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_name().await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "test");
}

#[tokio::test]
async fn bin_x_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBins {
            times: 2,
            camera_valid_bins: { vec![1_u8, 2_u8] },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera.bin_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_u8);

    //when
    let res = camera.bin_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_u8);
}

#[rstest]
#[case(true, 1, vec![1, 2], 1, Ok(()), 0, Ok(()))]
#[case(true, 2, vec![1, 2], 1, Ok(()), 1, Ok(()))]
#[case(true, 2, vec![1, 2], 1, Err(eyre!("error")), 1, Err(ASCOMError::VALUE_NOT_SET))]
#[case(true, 0, vec![1, 2], 1, Ok(()), 0, Err(ASCOMError::invalid_value("bin value must be one of the valid bins")))]
#[case(false, 1, vec![1, 2], 1, Ok(()), 0, Ok(()))]
#[case(false, 2, vec![1, 2], 1, Ok(()), 1, Ok(()))]
#[case(false, 2, vec![1, 2], 1, Err(eyre!("error")), 1, Err(ASCOMError::VALUE_NOT_SET))]
#[case(false, 0, vec![1, 2], 1, Ok(()), 0, Err(ASCOMError::invalid_value("bin value must be one of the valid bins")))]
#[tokio::test]
async fn set_bin_x_y(
    #[case] x: bool,
    #[case] bin: u32,
    #[case] camera_valid_bins: Vec<u8>,
    #[case] camera_binning: u8,
    #[case] set_bin_mode: Result<()>,
    #[case] set_bin_mode_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(set_bin_mode_times)
        .withf(move |x: &u32, y: &u32| *x == bin && *y == bin)
        .return_once(move |_, _| set_bin_mode);
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBins {
            times: 1,
            camera_valid_bins,
            camera_binning,
        },
    );
    //when
    let res = if x {
        camera.set_bin_x(bin as u8).await
    } else {
        camera.set_bin_y(bin as u8).await
    };
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(10, 20, 5, 10)]
#[case(5, 11, 2, 5)]
#[tokio::test]
async fn set_bin_x_with_roi(
    #[case] start_x: u32,
    #[case] start_y: u32,
    #[case] expected_start_x: u32,
    #[case] expected_start_y: u32,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBinsAndRoiAndCCDInfo {
            times: 9,
            camera_roi: CCDChipArea {
                start_x,
                start_y,
                width: 1920,
                height: 1080,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
            camera_valid_bins: { vec![1_u8, 2_u8] },
        },
    );
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_ok());
    assert_eq!(camera.camera_x_size().await.unwrap(), 1920_u32);
    assert_eq!(camera.camera_y_size().await.unwrap(), 1080_u32);
    assert_eq!(camera.bin_x().await.unwrap(), 2_u8);
    assert_eq!(camera.bin_y().await.unwrap(), 2_u8);
    assert_eq!(camera.start_x().await.unwrap(), expected_start_x as u32);
    assert_eq!(camera.start_y().await.unwrap(), expected_start_y as u32);
    assert_eq!(camera.num_x().await.unwrap(), 960_u32);
    assert_eq!(camera.num_y().await.unwrap(), 540_u32);
}

#[tokio::test]
async fn set_bin_x_fail_no_valid_bins() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn unimplmented_functions() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    assert_eq!(
        camera.electrons_per_adu().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera.full_well_capacity().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera.stop_exposure().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
}

#[rstest]
#[case(Ok(12_f64), Ok(4096_u32))]
#[case(Err(eyre!("error")), Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn max_adu(#[case] bits: Result<f64>, #[case] expected: ASCOMResult<u32>) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_parameter()
        .once()
        .withf(move |control| *control == qhyccd_rs::Control::OutputDataActualBits)
        .return_once(move |_| bits);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.max_adu().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some((0_f64, 3_600_000_000_f64, 1_f64)), Ok(Duration::from_secs_f64(3_600.0)))]
#[case(None, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn exposure_max(
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<Duration>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep { min_max_step },
    );
    //when
    let res = camera.exposure_max().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some((0_f64, 3_600_000_000_f64, 1_f64)), Ok(Duration::from_secs_f64(0.0)))]
#[case(None, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn exposure_min(
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<Duration>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep { min_max_step },
    );
    //when
    let res = camera.exposure_min().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some((0_f64, 3_600_000_000_f64, 1_f64)), Ok(Duration::from_secs_f64(1e-6)))]
#[case(None, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn exposure_resolution(
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<Duration>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep { min_max_step },
    );
    //when
    let res = camera.exposure_resolution().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some(0), Ok(true))]
#[case(None, Ok(false))]
#[tokio::test]
async fn has_shutter(
    #[case] is_control_available: Option<u32>,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(move |control| *control == qhyccd_rs::Control::CamMechanicalShutter)
        .returning(move |_| is_control_available);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.has_shutter().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[tokio::test]
async fn image_array_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithImage {
            image_array: Array3::<u16>::zeros((10_usize, 10_usize, 3)).into(),
        },
    );
    //when
    let res = camera.image_array().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap().shape(), [10, 10, 3]);
}

#[tokio::test]
async fn image_array_empty() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.image_array().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn image_ready_not_ready_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 1000_f64,
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn image_ready_ready_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithImage {
            image_array: Array3::<u16>::zeros((10_usize, 10_usize, 3)).into(),
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn image_ready_ready_success_no_image_taken_yet() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithState {
            times: 1,
            state: State::Idle,
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[rstest]
#[case(Some(SystemTime::UNIX_EPOCH), Ok(SystemTime::UNIX_EPOCH))]
#[case(None, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn last_exposure_start_time(
    #[case] start_time: Option<SystemTime>,
    #[case] expected: ASCOMResult<SystemTime>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::WithLastExposureStart { start_time });
    //when
    let res = camera.last_exposure_start_time().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some(2_000_000_u32), Ok(Duration::from_secs_f64(2.0)))]
#[case(None, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn last_exposure_duration(
    #[case] duration: Option<u32>,
    #[case] expected: ASCOMResult<Duration>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::WithLastExposureDuration { duration });
    //when
    let res = camera.last_exposure_duration().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(1920_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn camera_xsize(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: if has_roi {
                Some(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.9,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.camera_x_size().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(1080_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn camera_ysize(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: if has_roi {
                Some(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.9,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.camera_y_size().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn start_x(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 10,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.start_x().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_start_x(
    #[case] x: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_start_x(x).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: x as u32,
                start_y: 0,
                width: 100,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn start_y(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 100,
                    width: 10,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.start_y().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_start_y(
    #[case] y: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_start_y(y).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: y,
                width: 100,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(1000_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn num_x(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 1000,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.num_x().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(1000_u32, 1, true, Ok(()))]
#[case(1000_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_num_x(
    #[case] w: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_num_x(w).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: w as u32,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn num_y(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 10,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.num_y().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_num_y(
    #[case] h: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 1001,
                    height: 1000,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_num_y(h).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 1001,
                height: h as u32,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Ok(5_000_u32), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 10_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(50_u8))]
#[case(Ok(10_000_u32), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 10_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(100_u8))]
#[case(Ok(10_000_u32), 0, State::Idle {}, Ok(100_u8))]
#[case(Ok(std::u32::MIN), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 0_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(0_u8))]
#[case(Ok(std::u32::MAX), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 0_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(100_u8))]
#[case(Err(eyre!("error")), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 10_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn percent_completed(
    #[case] remaining_exposure_us: Result<u32>,
    #[case] remaining_exposure_us_times: usize,
    #[case] state: State,
    #[case] expected: ASCOMResult<u8>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .times(remaining_exposure_us_times)
        .return_once(move || remaining_exposure_us);
    let camera = new_camera(mock, MockCameraType::WithState { times: 1, state });
    //when
    let res = camera.percent_completed().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Ok(2), Ok(2_usize))]
#[case(Err(eyre!("error")), Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn readout_mode_success(
    #[case] readout_mode: Result<u32>,
    #[case] expected: ASCOMResult<usize>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_readout_mode()
        .once()
        .return_once(move || readout_mode);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_mode().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(3, Ok(4_u32), Ok((1920_u32, 1080_u32)), 1, Ok(()), 1, true, Ok(()))]
#[case(3, Ok(4_u32), Ok((1920_u32, 1080_u32)), 1, Ok(()), 1, false, Ok(()))]
#[case(5, Ok(4_u32), Ok((1920_u32, 1080_u32)), 0, Ok(()), 0, true, Err(ASCOMError::INVALID_VALUE))]
#[case(3, Err(eyre!("error")), Ok((1920_u32, 1080_u32)), 0, Ok(()), 0, true, Err(ASCOMError::INVALID_VALUE))]
#[case(3, Ok(4_u32), Err(eyre!("error")), 1, Ok(()), 0, true, Err(ASCOMError::INVALID_VALUE))]
#[case(3, Ok(4_u32), Ok((1920_u32, 1080_u32)), 1, Err(eyre!("error")), 1, true, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn set_readout_mode(
    #[case] mode: usize,
    #[case] number_of_readout_modes: Result<u32>,
    #[case] resolution: Result<(u32, u32)>,
    #[case] resolution_times: usize,
    #[case] set_mode: Result<()>,
    #[case] set_mode_times: usize,
    #[case] has_ccd_info: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .return_once(move || number_of_readout_modes);
    mock.expect_get_readout_mode_resolution()
        .times(resolution_times)
        .withf(move |readout_mode| *readout_mode == 3)
        .return_once(move |_| resolution);
    mock.expect_set_readout_mode()
        .times(set_mode_times)
        .withf(move |readout_mode| *readout_mode == 3)
        .return_once(|_| set_mode);
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: if has_ccd_info {
                Some(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.9,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_readout_mode(mode).await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Ok(1_u32), Ok("Standard Mode".to_string()), 1, Ok(vec!["Standard Mode".to_string()]))]
#[case(Err(eyre!("error")), Ok("Standard Mode".to_string()), 0, Err(ASCOMError::INVALID_OPERATION))]
#[case(Ok(1_u32), Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn readout_modes(
    #[case] number_of_readout_modes: Result<u32>,
    #[case] get_readout_mode_name: Result<String>,
    #[case] get_name_times: usize,
    #[case] expected: ASCOMResult<Vec<String>>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .return_once(move || number_of_readout_modes);
    mock.expect_get_readout_mode_name()
        .times(get_name_times)
        .withf(move |index| *index == 0)
        .return_once(move |_| get_readout_mode_name);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_modes().await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(Some(0), Some(1), 1, Ok(SensorType::RGGB))]
#[case(None, Some(1), 0, Ok(SensorType::Monochrome))]
#[case(Some(0), None, 1, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn sensor_type_success_color(
    #[case] cam_is_color: Option<u32>,
    #[case] cam_color: Option<u32>,
    #[case] cam_color_times: usize,
    #[case] expected: ASCOMResult<SensorType>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(move |control| *control == qhyccd_rs::Control::CamIsColor)
        .return_once(move |_| cam_is_color);
    mock.expect_is_control_available()
        .times(cam_color_times)
        .withf(move |control| *control == qhyccd_rs::Control::CamColor)
        .return_once(move |_| cam_color);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_type().await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[tokio::test]
async fn stop_abort() {
    //given
    let camera = new_camera(MockCamera::new(), MockCameraType::Untouched);
    // when / then
    assert!(!camera.can_stop_exposure().await.unwrap());
    assert!(camera.can_abort_exposure().await.unwrap());
}

#[rustfmt::skip]
#[rstest]
#[case(Duration::from_secs_f64(10.0), false, Err(ASCOMError::invalid_operation("dark frames not supported")))]
#[tokio::test]
async fn start_exposure_fail_dark_neg(
    #[case] duration: Duration,
    #[case] is_dark: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.start_exposure(duration, is_dark).await;
    //then
    assert_eq!(
        res.unwrap_err().to_string(),
        expected.unwrap_err().to_string(),
    )
}

#[rstest]
#[case(3, 100, 0, 10, 10, Err(ASCOMError::invalid_value("StartX > NumX")))]
#[case(5, 0, 100, 10, 10, Err(ASCOMError::invalid_value("StartY > NumY")))]
#[tokio::test]
async fn start_exposure_fail_start_num(
    #[case] times: usize,
    #[case] start_x: u32,
    #[case] start_y: u32,
    #[case] num_x: u32,
    #[case] num_y: u32,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: Some(CCDChipArea {
                start_x,
                start_y,
                width: num_x,
                height: num_y,
            }),
        },
    );
    //when
    let res = camera
        .start_exposure(Duration::from_secs_f64(1000.0), true)
        .await;
    //then
    assert_eq!(
        res.unwrap_err().to_string(),
        expected.unwrap_err().to_string(),
    )
}

#[rustfmt::skip]
#[rstest]
#[case(8, 50, 100, 20, 1080, Err(ASCOMError::invalid_value("NumX > CameraXSize")))]
#[case(11, 50, 100, 1920, 80, Err(ASCOMError::invalid_value("NumY > CameraYSize")))]
#[tokio::test]
async fn start_exposure_fail_num_size(
    #[case] times: usize,
    #[case] num_x: u32,
    #[case] num_y: u32,
    #[case] image_width: u32,
    #[case] image_height: u32,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: num_x,
                height: num_y,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width,
                image_height,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera.start_exposure(Duration::from_secs_f64(1000.0), true).await;
    //then
    assert_eq!(
        res.unwrap_err().to_string(),
        expected.unwrap_err().to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_set_roi() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            }
        })
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetRoiError { error_code: 123 })));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times: 11,
            camera_roi: CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera
        .start_exposure(Duration::from_secs_f64(1000.0), true)
        .await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("failed to set ROI").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_is_exposing_no_miri() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            }
        })
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfoAndExposing {
            times: 11,
            camera_roi: CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
            expected_duration: 1000_f64,
        },
    );
    //when
    let res = camera
        .start_exposure(Duration::from_secs_f64(1000.0), true)
        .await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[rustfmt::skip]
#[rstest]
#[case(vec![0, 1, 2, 3, 4, 5], 3, 2, 8, 1, Ok(()), array![[[0_u8],[3_u8]],[[1_u8],[4_u8]],[[2_u8],[5_u8]]].into())] //8bpp
#[case(Vec::new(), 3, 2, 8, 1, Err(ASCOMError::INVALID_OPERATION), Array3::<u16>::zeros((1_usize, 1_usize, 3)).into())] // invalid vector
#[case(vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0], 3, 2, 16, 1, Ok(()), array![[[0_u16],[3_u16]],[[1_u16],[4_u16]],[[2_u16],[5_u16]]].into())] //16bpp
#[case(Vec::new(), 3, 2, 16, 1, Err(ASCOMError::INVALID_OPERATION), Array3::<u16>::zeros((1_usize, 1_usize, 3)).into())] //invalid vector
#[case(vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0], 3, 2, 16, 2, Err(ASCOMError::INVALID_OPERATION), Array3::<u16>::zeros((1_usize, 1_usize, 3)).into())] //unsupported channel
#[case(vec![0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0], 3, 2, 32, 1, Err(ASCOMError::INVALID_OPERATION), Array3::<u16>::zeros((1_usize, 1_usize, 3)).into())] //unsupported bpp*/
#[tokio::test]
#[ignore]
async fn start_exposure_success_no_miri(
    #[case] data: Vec<u8>,
    #[case] width: u32,
    #[case] height: u32,
    #[case] bits_per_pixel: u32,
    #[case] channels: u32,
    #[case] expected: ASCOMResult,
    #[case] expected_image: ImageArray,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_parameter()
        .once()
        .withf(|control, exposure| {
            *control == qhyccd_rs::Control::Exposure && *exposure == 1_000_000_f64
        })
        .returning(|_, _| Ok(()));
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            }
        })
        .returning(|_| Ok(()));
    let mut clone_mock = MockCamera::new();
    clone_mock
        .expect_start_single_frame_exposure()
        .once()
        .returning(|| Ok(()));
    clone_mock
        .expect_get_image_size()
        .once()
        .returning(|| Ok(100_usize));
    clone_mock
        .expect_get_single_frame()
        .once()
        .withf(|size| *size == 100_usize)
        .returning(move |_| {
            Ok(qhyccd_rs::ImageData {
                data: data.clone(),
                width,
                height,
                bits_per_pixel,
                channels,
            })
        });
    clone_mock.expect_clone().once().returning(MockCamera::new);
    mock.expect_clone().once().return_once(move || clone_mock);
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfoUnlimited {
            camera_roi: CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera.start_exposure(Duration::from_secs_f64(1.0), true).await;
    
    // Wait for exposure to complete with 1 second timeout
    let timeout = tokio::time::Duration::from_secs(1);
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout {
        if matches!(camera.camera_state().await, Ok(ascom_alpaca::api::camera::CameraState::Idle)) {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    let image = camera.image_array().await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
        assert_eq!(image.unwrap(), expected_image);
    } else {
        assert!(res.is_ok()); // start_exposure should succeed
        // For error cases, image_array should return error
        assert!(image.is_err());
    }
}

#[rustfmt::skip]
#[rstest]
#[case(true, true, 1, true, 1, true, 1, true, 1, 1, Ok(()))]
#[case(false, true, 0, true, 0, true, 0, true, 0, 0, Err(ASCOMError::invalid_value("failed to set ROI")))]
#[case(true, false, 1, true, 0, true, 0, true, 0, 0, Err(ASCOMError::INVALID_OPERATION))]
#[case(true, true, 1, false, 1, true, 1, true, 1, 1, Ok(()))]
#[case(true, true, 1, true, 1, false, 1, true, 0, 1, Ok(()))]
#[case(true, true, 1, true, 1, true, 1, false, 1, 1, Ok(()))]
#[tokio::test]
#[ignore]
async fn start_exposure_fail_no_miri(
    #[case] set_roi_ok: bool,
    #[case] set_parameter_ok: bool,
    #[case] set_parameter_times: usize,
    #[case] start_single_frame_ok: bool,
    #[case] start_single_frame_times: usize,
    #[case] get_image_size_ok: bool,
    #[case] get_image_size_times: usize,
    #[case] get_singleframe_ok: bool,
    #[case] get_singleframe_times: usize,
    #[case] clone_times: usize,
    #[case] expected: ASCOMResult,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .times(1)
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            }
        })
        .returning(move |_| {
            if set_roi_ok {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_set_parameter()
        .times(set_parameter_times)
        .withf(|control, exposure| {
            *control == qhyccd_rs::Control::Exposure && *exposure == 1_000_000_f64
        })
        .returning(move |_, _| {
            if set_parameter_ok {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    let mut clone_mock = MockCamera::new();
    clone_mock
        .expect_start_single_frame_exposure()
        .times(start_single_frame_times)
        .returning(move || {
            if start_single_frame_ok {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    clone_mock
        .expect_get_image_size()
        .times(get_image_size_times)
        .returning(move || {
            if get_image_size_ok {
                Ok(100)
            } else {
                Err(eyre!("error"))
            }
        });
    clone_mock
        .expect_get_single_frame()
        .times(get_singleframe_times)
        .withf(|size| *size == 100_usize)
        .returning(move |_| {
            if get_singleframe_ok {
                Ok(qhyccd_rs::ImageData {
                    data: vec![0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5],
                    width: 3,
                    height: 2,
                    bits_per_pixel: 16,
                    channels: 1,
                })
            } else {
                Err(eyre!("error"))
            }
        });
    if clone_times > 0 {
        clone_mock.expect_clone().once().returning(MockCamera::new);
    }

    mock.expect_clone()
        .times(clone_times)
        .return_once(move || clone_mock);
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfoUnlimited {
            camera_roi: CCDChipArea {
                start_x: 10,
                start_y: 20,
                width: 1920,
                height: 1080,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera.start_exposure(Duration::from_secs_f64(1.0), true).await;
    
    // Wait for exposure to complete with 1 second timeout
    let timeout = tokio::time::Duration::from_secs(1);
    let start = tokio::time::Instant::now();
    while start.elapsed() < timeout {
        if matches!(camera.camera_state().await, Ok(ascom_alpaca::api::camera::CameraState::Idle)) {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    let image = camera.image_array().await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
        // For async failures, check that image_array returns error
        if !start_single_frame_ok || !get_image_size_ok || !get_singleframe_ok {
            assert!(image.is_err());
        }
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.unwrap_err().to_string(),
        )
    }
}

#[rstest]
#[case(true, 1, 2.5_f64, Ok(2.5_f64))]
#[case(false, 1, 2.5_f64, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn pixel_size_x(
    #[case] has_ccd_info: bool,
    #[case] is_open_times: usize,
    #[case] size: f64,
    #[case] expected: ASCOMResult<f64>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: is_open_times,
            camera_ccd_info: if has_ccd_info {
                Some(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: size,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.pixel_size_x().await;
    //then
    if res.is_ok() {
        assert!((res.unwrap() - expected.unwrap()).abs() < f64::EPSILON);
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, 1, 2.9_f64, Ok(2.9_f64))]
#[case(false, 1, 2.9_f64, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn pixel_size_y(
    #[case] has_ccd_info: bool,
    #[case] is_open_times: usize,
    #[case] size: f64,
    #[case] expected: ASCOMResult<f64>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: is_open_times,
            camera_ccd_info: if has_ccd_info {
                Some(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.5,
                    pixel_height: size,
                    bits_per_pixel: 16,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.pixel_size_y().await;
    //then
    if res.is_ok() {
        assert!((res.unwrap() - expected.unwrap()).abs() < f64::EPSILON);
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}
#[rstest]
#[case(true, Ok(1_f64), 1, Ok(true))]
#[case(true, Ok(0_f64), 1, Ok(false))]
#[case(true, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_VALUE))]
#[case(false, Ok(1_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn cooler_on(
    #[case] is_control_available: bool,
    #[case] get_parameter: Result<f64>,
    #[case] get_parameter_times: usize,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if is_control_available { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(get_parameter_times)
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .return_once(move |_| get_parameter);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_on().await;
    //then
    if res.is_ok() {
        assert!(expected.is_ok())
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, true, true, 0, Ok(()))]
#[case(false, false, true, 0, Ok(()))]
#[case(true, false, true, 1, Ok(()))]
#[case(false, true, true, 1, Ok(()))]
#[case(true, false, false, 1, Err(ASCOMError::INVALID_OPERATION))]
#[case(false, true, false, 1, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn set_cooler_on(
    #[case] is_cooler_on: bool,
    #[case] cooler_on: bool,
    #[case] set_manualpwm_ok: bool,
    #[case] set_manualpwm_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(move |_| if is_cooler_on { Ok(1_f64) } else { Ok(0_f64) });
    mock.expect_set_parameter()
        .times(set_manualpwm_times)
        .withf(move |control, temp| {
            *control == qhyccd_rs::Control::ManualPWM
                && (*temp - if cooler_on { 1_f64 } else { 0_f64 } / 100_f64 * 255_f64).abs()
                    < f64::EPSILON
        })
        .returning(move |_, _| {
            if set_manualpwm_ok {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(cooler_on).await;
    //then
    if res.is_ok() {
        assert!(expected.is_ok())
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, Ok(25_f64), 1, Ok(25_f64/255_f64*100_f64))]
#[case(true, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_VALUE))]
#[case(false, Ok(25_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn cooler_power(
    #[case] has_cooler: bool,
    #[case] get_pwm: Result<f64>,
    #[case] get_pwm_times: usize,
    #[case] expected: ASCOMResult<f64>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if has_cooler { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(get_pwm_times)
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .return_once(move |_| get_pwm);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_power().await;
    //then
    if res.is_ok() {
        assert!((res.unwrap() - expected.unwrap()).abs() < f64::EPSILON);
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, Ok(true))]
#[case(false, Ok(false))]
#[tokio::test]
async fn can_set_ccd_temperature(#[case] has_cooler: bool, #[case] expected: ASCOMResult<bool>) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if has_cooler { Some(0) } else { None });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.can_set_ccd_temperature().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, Ok(25_f64), 1, Ok(25_f64))]
#[case(true, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_VALUE))]
#[case(false, Ok(25_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn ccd_temperature_success_cooler(
    #[case] has_cooler: bool,
    #[case] cur_temp: Result<f64>,
    #[case] cur_temp_times: usize,
    #[case] expected: ASCOMResult<f64>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if has_cooler { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(cur_temp_times)
        .withf(|control| *control == qhyccd_rs::Control::CurTemp)
        .return_once(move |_| cur_temp);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.ccd_temperature().await;
    //then
    if res.is_ok() {
        assert!((res.unwrap() - expected.unwrap()).abs() < f64::EPSILON);
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, 2, None, 2, Ok(25_f64), 1, Ok(25_f64))]
#[case(true, 2, None, 2, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_VALUE))]
#[case(true, 1, Some(-2_f64), 1, Ok(25_f64), 0, Ok(-2_f64))]
#[case(false, 1, Some(-2_f64), 1, Ok(25_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn set_ccd_temperature(
    #[case] has_cooler: bool,
    #[case] cooler_times: usize,
    #[case] target_temperature: Option<f64>,
    #[case] is_open_times: usize,
    #[case] cur_temp: Result<f64>,
    #[case] cur_temp_times: usize,
    #[case] expected: ASCOMResult<f64>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(cooler_times)
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if has_cooler { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(cur_temp_times)
        .withf(|control| *control == qhyccd_rs::Control::CurTemp)
        .return_once(move |_| cur_temp);
    let camera = new_camera(
        mock,
        MockCameraType::WithTargetTemperature {
            times: is_open_times,
            temperature: target_temperature,
        },
    );
    //when
    let res = camera.set_ccd_temperature().await;
    //then
    if res.is_ok() {
        assert!((res.unwrap() - expected.unwrap()).abs() < f64::EPSILON);
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, 1, 1, -2_f64, Ok(()), 1, Ok(()))]
#[case(true, 0, 0, -300_f64, Ok(()), 0, Err(ASCOMError::INVALID_VALUE))]
#[case(true, 0, 0, 81_f64, Ok(()), 0, Err(ASCOMError::INVALID_VALUE))]
#[case(true, 1, 1, -2_f64, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[case(false, 1, 1, -2_f64, Ok(()), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn set_set_ccd_temperature(
    #[case] has_cooler: bool,
    #[case] is_control_avaiable_times: usize,
    #[case] is_open_times: usize,
    #[case] temperature: f64,
    #[case] set_parameter: Result<()>,
    #[case] set_parameter_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(is_control_avaiable_times)
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(move |_| if has_cooler { Some(0) } else { None });
    mock.expect_set_parameter()
        .times(set_parameter_times)
        .withf(move |control, temp| {
            *control == qhyccd_rs::Control::Cooler && (*temp - temperature).abs() < f64::EPSILON
        })
        .return_once(move |_, _| set_parameter);
    let camera = new_camera(
        mock,
        MockCameraType::IsOpenTrue {
            times: is_open_times,
        },
    );
    //when
    let res = camera.set_set_ccd_temperature(temperature).await;
    //then
    if res.is_ok() {
        assert!(expected.is_ok());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, 1, 1, Ok(25_f64), 1, Ok(25_i32))]
#[case(true, 1, 1, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[case(false, 1, 1, Ok(25_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn gain(
    #[case] is_control_available: bool,
    #[case] is_control_available_times: usize,
    #[case] open_times: usize,
    #[case] get_parameter: Result<f64>,
    #[case] get_parameter_times: usize,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(is_control_available_times)
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| if is_control_available { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(get_parameter_times)
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .return_once(move |_| get_parameter);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: open_times });
    //when
    let res = camera.gain().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(50_i32, true, 1, 1, Some((0_f64,  51_f64)), Ok(()), 1, Ok(()))]
#[case(50_i32, true, 1, 1, None, Ok(()), 0, Err(ASCOMError::invalid_operation("camera reports gain control available, but min, max values are not set after initialization")))]
#[case(-50_i32, true, 1, 1, Some((0_f64,  51_f64)), Ok(()), 0, Err(ASCOMError::INVALID_VALUE))]
#[case(50_i32, false, 1, 1, Some((0_f64,  51_f64)), Ok(()), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(50_i32, true, 1, 1, Some((0_f64,  51_f64)), Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn set_gain(
    #[case] gain: i32,
    #[case] is_control_available: bool,
    #[case] is_control_available_times: usize,
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] set_parameter: Result<()>,
    #[case] set_parameter_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(is_control_available_times)
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| if is_control_available { Some(0) } else { None });
    mock.expect_set_parameter()
        .times(set_parameter_times)
        .withf(move |control, g| {
            *control == qhyccd_rs::Control::Gain && (*g - gain as f64).abs() < f64::EPSILON
        })
        .return_once(move |_, _| set_parameter);
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.set_gain(gain).await;
    //then
    if res.is_ok() {
        assert!(expected.is_ok());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(1, None, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(1, Some((0_f64, 51_f64)), Ok(0_i32))]
#[tokio::test]
async fn gain_min(
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.gain_min().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(1, None, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(1, Some((0_f64, 51_f64)), Ok(51_i32))]
#[tokio::test]
async fn gain_max(
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.gain_max().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, 1, 1, Ok(25_f64), 1, Ok(25_i32))]
#[case(true, 1, 1, Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[case(false, 1, 1, Ok(25_f64), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn offset(
    #[case] is_control_available: bool,
    #[case] is_control_available_times: usize,
    #[case] open_times: usize,
    #[case] get_parameter: Result<f64>,
    #[case] get_parameter_times: usize,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(is_control_available_times)
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| if is_control_available { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(get_parameter_times)
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .return_once(move |_| get_parameter);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: open_times });
    //when
    let res = camera.offset().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(250_i32, true, 1, 1, Some((0_f64,  1023_f64)), Ok(()), 1, Ok(()))]
#[case(250_i32, true, 1, 1, None, Ok(()), 0, Err(ASCOMError::invalid_operation("camera reports offset control available, but min, max values are not set after initialization")))]
#[case(-250_i32, true, 1, 1, Some((0_f64,  1023_f64)), Ok(()), 0, Err(ASCOMError::INVALID_VALUE))]
#[case(250_i32, false, 1, 1, Some((0_f64,  1023_f64)), Ok(()), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(250_i32, true, 1, 1, Some((0_f64,  1023_f64)), Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn set_offset(
    #[case] offset: i32,
    #[case] is_control_available: bool,
    #[case] is_control_available_times: usize,
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] set_parameter: Result<()>,
    #[case] set_parameter_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(is_control_available_times)
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| if is_control_available { Some(0) } else { None });
    mock.expect_set_parameter()
        .times(set_parameter_times)
        .withf(move |control, off| {
            *control == qhyccd_rs::Control::Offset && (*off - offset as f64).abs() < f64::EPSILON
        })
        .return_once(move |_, _| set_parameter);
    let camera = new_camera(
        mock,
        MockCameraType::WithOffset {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.set_offset(offset).await;
    //then
    if res.is_ok() {
        assert!(expected.is_ok());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(1, None, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(1, Some((0_f64, 1023_f64)), Ok(0_i32))]
#[tokio::test]
async fn offset_min(
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithOffset {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.offset_min().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(1, None, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(1, Some((0_f64, 1023_f64)), Ok(1023_i32))]
#[tokio::test]
async fn offset_max(
    #[case] open_times: usize,
    #[case] min_max: Option<(f64, f64)>,
    #[case] expected: ASCOMResult<i32>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithOffset {
            times: open_times,
            min_max,
        },
    );
    //when
    let res = camera.offset_max().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(Some(0), Some((0_f64, 2_f64, 1_f64)), Ok(true))]
#[case(Some(0), None, Ok(false))]
#[case(None, Some((0_f64, 1_f64, 0.1_f64)), Ok(false))]
#[tokio::test]
async fn can_fast_readout(
    #[case] is_control_available: Option<u32>,
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .return_once(move |_| is_control_available);
    let camera = new_camera(
        mock,
        MockCameraType::WithReadoutMinMax {
            times: 1,
            min_max_step,
        },
    );
    //when
    let res = camera.can_fast_readout().await;
    //then
    if res.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(Some(0), Ok(2_f64), 1, Some((0_f64, 2_f64, 1_f64)), Ok(true))]
#[case(Some(0), Ok(0_f64), 1, Some((0_f64, 2_f64, 1_f64)), Ok(false))]
#[case(Some(0), Err(eyre!("error")), 1, Some((0_f64, 2_f64, 1_f64)), Err(ASCOMError::INVALID_OPERATION))]
#[case(Some(0), Ok(0_f64), 1, None, Err(ASCOMError::INVALID_OPERATION))]
#[case(None, Ok(0_f64), 0, None, Err(ASCOMError::NOT_IMPLEMENTED))]
#[tokio::test]
async fn fast_readout(
    #[case] is_control_available: Option<u32>,
    #[case] get_parameter: Result<f64>,
    #[case] get_parameter_times: usize,
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .return_once(move |_| is_control_available);
    mock.expect_get_parameter()
        .times(get_parameter_times)
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .return_once(move |_| get_parameter);
    let camera = new_camera(
        mock,
        MockCameraType::WithReadoutMinMax {
            times: 1,
            min_max_step,
        },
    );
    //when
    let res = camera.fast_readout().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}

#[rstest]
#[case(true, Some(0), Some((0_f64, 2_f64, 1_f64)), Ok(()), 1, Ok(()))]
#[case(false, Some(0), Some((0_f64, 2_f64, 1_f64)), Ok(()), 1, Ok(()))]
#[case(true, None, Some((0_f64, 2_f64, 1_f64)), Ok(()), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(true, Some(0), None, Ok(()), 0, Err(ASCOMError::invalid_operation("camera reports readout speed control available, but min, max values are not set after initialization")))]
#[case(true, Some(0), Some((0_f64, 2_f64, 1_f64)), Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn set_fast_readout(
    #[case] readout: bool,
    #[case] is_control_available: Option<u32>,
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] set_parameter: Result<()>,
    #[case] set_parameter_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .return_once(move |_| is_control_available);
    mock.expect_set_parameter()
        .times(set_parameter_times)
        .withf(move |control, speed| {
            *control == qhyccd_rs::Control::Speed
                && (*speed - if readout { 2_f64 } else { 0_f64 }).abs() < f64::EPSILON
        })
        .return_once(move |_, _| set_parameter);
    let camera = new_camera(
        mock,
        MockCameraType::WithReadoutMinMax {
            times: 1,
            min_max_step,
        },
    );
    //when
    let res = camera.set_fast_readout(readout).await;
    //then
    if expected.is_ok() {
        assert!(res.is_ok());
    } else {
        assert_eq!(
            res.err().unwrap().to_string(),
            expected.err().unwrap().to_string()
        )
    }
}
