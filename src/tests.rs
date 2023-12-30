use qhyccd_rs::Control;

use super::*;
use crate::mocks::MockCamera;
use eyre::eyre;
use ndarray::Array3;

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
    not_connected! {camera_xsize()}
    not_connected! {camera_ysize()}
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
    not_connected! {start_exposure(1.0, true)}
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
}

enum MockCameraType {
    IsOpenTrue {
        times: usize,
    },
    IsOpenFalse {
        times: usize,
    },
    WithRoiAndCCDInfo {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
    },
    Untouched,
    WithStateExposing {
        expected_duration: f64,
    },
    WithStateIdle {},
    WithImage {
        image_array: ImageArray,
    },
    WithExposureMinMaxStep {
        min: f64,
        max: f64,
        step: f64,
    },
    WithLastExposureStart {
        start_time: SystemTime,
    },
    WithLastExposureDuration {
        duration_us: f64,
    },
    WithBinning {
        camera_binning: BinningMode,
    },
    WithBinningAndRoiAndCCDInfo {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: BinningMode,
    },
    WithBinningAndRoiAndCCDInfoAndExposing {
        times: usize,
        camera_roi: CCDChipArea,
        camera_ccd_info: CCDChipInfo,
        camera_binning: BinningMode,
        expected_duration: f64,
    },
    WithTargetTemperature {
        times: usize,
        temperature: f64,
    },
    WithGain {
        times: usize,
        gain_min: f64,
        gain_max: f64,
    },
}

fn new_camera(mut device: MockCamera, variant: MockCameraType) -> QhyccdCamera {
    let mut binning = RwLock::new(BinningMode { symmetric_value: 1 });
    let mut target_temperature = RwLock::new(None);
    let mut ccd_info = RwLock::new(None);
    let mut intended_roi = RwLock::new(None);
    let mut exposing = RwLock::new(State::Idle);
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
        MockCameraType::WithRoiAndCCDInfo {
            times,
            camera_roi,
            camera_ccd_info,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            ccd_info = RwLock::new(Some(camera_ccd_info));
            intended_roi = RwLock::new(Some(camera_roi));
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
        MockCameraType::WithStateIdle {} => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            exposing = RwLock::new(State::Idle);
        }
        MockCameraType::WithImage { image_array: image } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_image = RwLock::new(Some(image));
        }
        MockCameraType::WithExposureMinMaxStep { min, max, step } => {
            device.expect_is_open().once().returning(|| Ok(true));
            exposure_min_max_step = RwLock::new(Some((min, max, step)));
        }
        MockCameraType::WithLastExposureStart { start_time } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_start_time = RwLock::new(Some(start_time));
        }
        MockCameraType::WithLastExposureDuration { duration_us } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_duration_us = RwLock::new(Some(duration_us as u32));
        }
        MockCameraType::WithBinning { camera_binning } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
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
            target_temperature = RwLock::new(Some(temperature));
        }
        MockCameraType::WithGain {
            times,
            gain_min,
            gain_max,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            gain_min_max = RwLock::new(Some((gain_min, gain_max)));
        }
    }
    QhyccdCamera {
        unique_id: "test-camera".to_owned(),
        name: "QHYCCD-test_camera".to_owned(),
        description: "QHYCCD camera".to_owned(),
        device,
        binning,
        valid_bins: RwLock::new(None),
        target_temperature,
        ccd_info,
        intended_roi,
        exposure_min_max_step,
        last_exposure_start_time,
        last_exposure_duration_us,
        last_image,
        state: exposing,
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
    //then
    assert_eq!(camera.unique_id, "test_camera");
    assert_eq!(camera.name, "QHYCCD-test_camera");
    assert_eq!(camera.description, "QHYCCD camera");
    assert_eq!(camera.binning.read().await.symmetric_value, 1);
    assert!(camera.valid_bins.read().await.is_none());
    assert!(camera.intended_roi.read().await.is_none());
    assert!(camera.last_exposure_start_time.read().await.is_none());
    assert!(camera.last_exposure_duration_us.read().await.is_none());
    assert!(camera.last_image.read().await.is_none());
    assert_eq!(*camera.state.read().await, State::Idle);
    assert_eq!(camera.static_name(), "QHYCCD-test_camera");
    assert_eq!(camera.unique_id(), "test_camera");
    assert_eq!(camera.description().await.unwrap(), "QHYCCD camera");
    assert_eq!(camera.driver_info().await.unwrap(), "qhyccd_alpaca-rs");
    assert_eq!(
        camera.driver_version().await.unwrap(),
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::test]
async fn max_bin_x_success() {
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
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    //when
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //then
    assert_eq!(camera.max_bin_x().await.unwrap(), 8);
    assert_eq!(camera.max_bin_y().await.unwrap(), 8);
}

#[tokio::test]
async fn max_bin_x_fail_no_modes() {
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
        .returning(|_| None);
    //when
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //then
    assert!(camera.max_bin_x().await.is_err());
    assert!(camera.max_bin_y().await.is_err());
}

#[tokio::test]
async fn camera_state_successw_idle() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.camera_state().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), CameraState::Idle);
}

#[tokio::test]
async fn camera_state_success_exposing() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 10000_f64,
        },
    );
    //when
    let res = camera.camera_state().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), CameraState::Exposing);
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

#[tokio::test]
async fn set_connected_true_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().times(1).returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area().times(1).returning(|| {
        Ok(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    });
    mock.expect_is_control_available()
        .times(6)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Exposure)
        .returning(|_| Ok((1_f64, 3_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Ok((0_f64, 51_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(|_| Some(0));
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(|_| Ok((0_f64, 1023_f64, 1_f64)));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_true_success_no_gain_no_offset() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().times(1).returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area().times(1).returning(|| {
        Ok(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    });
    mock.expect_is_control_available()
        .times(6)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Exposure)
        .returning(|_| Ok((1_f64, 3_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| None);
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_false_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_close().times(1).returning(|| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_fail_open() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open()
        .times(1)
        .returning(|| Err(eyre!("Could not open camera")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_set_stream_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Err(eyre!("Could not set stream mode")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_set_readout_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Err(eyre!("Could not set readout mode")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_init() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init()
        .once()
        .returning(|| Err(eyre!("Could not init camera")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_ccd_info() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info()
        .once()
        .returning(|| Err(eyre!("Could not get ccd info")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_effective_area() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area()
        .once()
        .returning(|| Err(eyre!("could not get effective area")));
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_parameter_min_max_step_exposure() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().times(1).returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area().times(1).returning(|| {
        Ok(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    });
    mock.expect_is_control_available()
        .times(6)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Exposure)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetMinMaxStepError {
                control: qhyccd_rs::Control::Exposure
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_parameter_min_max_step_gain() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().times(1).returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area().times(1).returning(|| {
        Ok(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    });
    mock.expect_is_control_available()
        .times(6)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Exposure)
        .returning(|_| Ok((0_f64, 3_600_000_000_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetMinMaxStepError {
                control: qhyccd_rs::Control::Gain
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_parameter_min_max_step_offset() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().times(1).returning(|| Ok(()));
    mock.expect_set_stream_mode()
        .once()
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(|_| Ok(()));
    mock.expect_set_readout_mode()
        .once()
        .withf(|mode| *mode == 0)
        .returning(|_| Ok(()));
    mock.expect_init().once().returning(|| Ok(()));
    mock.expect_get_ccd_info().once().returning(|| {
        Ok(CCDChipInfo {
            chip_width: 7.0,
            chip_height: 5.0,
            image_width: 1920,
            image_height: 1080,
            pixel_width: 2.9,
            pixel_height: 2.9,
            bits_per_pixel: 16,
        })
    });
    mock.expect_get_effective_area().times(1).returning(|| {
        Ok(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    });
    mock.expect_is_control_available()
        .times(6)
        .withf(|control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(|control| match control {
            Control::CamBin1x1mode => Some(0_u32),
            Control::CamBin2x2mode => Some(0_u32),
            Control::CamBin3x3mode => Some(0_u32),
            Control::CamBin4x4mode => Some(0_u32),
            Control::CamBin6x6mode => Some(0_u32),
            Control::CamBin8x8mode => Some(0_u32),
            _ => panic!("Unexpected control"),
        });
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Exposure)
        .returning(|_| Ok((0_f64, 3_600_000_000_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Ok((0_f64, 51_f64, 1_f64)));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(|_| Some(0));
    mock.expect_get_parameter_min_max_step()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Offset)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetMinMaxStepError {
                control: qhyccd_rs::Control::Offset
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_close() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_close()
        .times(1)
        .returning(|| Err(eyre!("Could not close camera")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}
// https://www.cloudynights.com/topic/883660-software-relating-to-bayer-patterns/
#[tokio::test]
async fn bayer_offset_success_gbrg() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(qhyccd_rs::BayerMode::GBRG as u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn bayer_offset_success_grbg() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(qhyccd_rs::BayerMode::GRBG as u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn bayer_offset_success_bggr() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(qhyccd_rs::BayerMode::BGGR as u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn bayer_offset_success_rggb() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(qhyccd_rs::BayerMode::RGGB as u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn bayer_offset_success_monochrome() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
}

#[tokio::test]
async fn bayer_offset_fail_invalid_bayer_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(0));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn bayer_offset_fail_none() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );

    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
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
async fn bin_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.bin_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn bin_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.bin_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn set_bin_x_success_same_bin() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinning {
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.set_bin_x(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_x_success_different_bin_no_roi_yet() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 1 && *bin_y == 1)
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinning {
            camera_binning: BinningMode { symmetric_value: 2 },
        },
    );
    //when
    let res = camera.set_bin_x(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_x_success_different_bin_with_roi_even() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times: 9,
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
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_ok());
    assert_eq!(camera.camera_xsize().await.unwrap(), 1920_i32);
    assert_eq!(camera.camera_ysize().await.unwrap(), 1080_i32);
    assert_eq!(camera.bin_x().await.unwrap(), 2_i32);
    assert_eq!(camera.bin_y().await.unwrap(), 2_i32);
    assert_eq!(camera.start_x().await.unwrap(), 5_i32);
    assert_eq!(camera.start_y().await.unwrap(), 10_i32);
    assert_eq!(camera.num_x().await.unwrap(), 960_i32);
    assert_eq!(camera.num_y().await.unwrap(), 540_i32);
}

#[tokio::test]
async fn set_bin_x_success_different_bin_with_roi_odd() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times: 9,
            camera_roi: CCDChipArea {
                start_x: 5,
                start_y: 11,
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
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_ok());
    assert_eq!(camera.camera_xsize().await.unwrap(), 1920_i32);
    assert_eq!(camera.camera_ysize().await.unwrap(), 1080_i32);
    assert_eq!(camera.bin_x().await.unwrap(), 2_i32);
    assert_eq!(camera.bin_y().await.unwrap(), 2_i32);
    assert_eq!(camera.start_x().await.unwrap(), 2_i32);
    assert_eq!(camera.start_y().await.unwrap(), 5_i32);
    assert_eq!(camera.num_x().await.unwrap(), 960_i32);
    assert_eq!(camera.num_y().await.unwrap(), 540_i32);
}

#[tokio::test]
async fn set_bin_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinning {
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.set_bin_y(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_x_fail_set_bin_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Err(eyre!("Could not set bin mode")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn set_bin_x_fail_invalid_bin() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.set_bin_x(0).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("bin value must be >= 1").to_string()
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

#[tokio::test]
async fn max_adu_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::OutputDataActualBits)
        .returning(|_| Ok(12_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.max_adu().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (4096_i32)); //2 to the power of 12
}

#[tokio::test]
async fn max_adu_fail_error_get_parameter() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::OutputDataActualBits)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetParameterError {
                control: qhyccd_rs::Control::OutputDataActualBits
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.max_adu().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn exposure_max_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep {
            min: 0_f64,
            max: 3_600_000_000_f64,
            step: 1_f64,
        },
    );
    //when
    let res = camera.exposure_max().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (3_600_f64));
}

#[tokio::test]
async fn exposure_max_fail_max_min_step() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.exposure_max().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn exposure_min_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep {
            min: 0_f64,
            max: 3_600_000_000_f64,
            step: 1_f64,
        },
    );
    //when
    let res = camera.exposure_min().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), (0_f64));
}

#[tokio::test]
async fn exposure_min_fail_max_min_step() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.exposure_min().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn exposure_resolution_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithExposureMinMaxStep {
            min: 0_f64,
            max: 3_600_000_000_f64,
            step: 1_f64,
        },
    );
    //when
    let res = camera.exposure_resolution().await;
    //then
    assert!(res.is_ok());
    assert!((res.unwrap() - 1_f64 / 1_000_000_f64).abs() < f64::EPSILON);
}

#[tokio::test]
async fn exposure_resolution_fail_max_min_step() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.exposure_resolution().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn has_shutter_true_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamMechanicalShutter)
        .returning(|_| Some(0_u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.has_shutter().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn has_shutter_false_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamMechanicalShutter)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.has_shutter().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
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
    let camera = new_camera(mock, MockCameraType::WithStateIdle {});
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn last_exposure_start_time_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithLastExposureStart {
            start_time: SystemTime::UNIX_EPOCH,
        },
    );
    //when
    let res = camera.last_exposure_start_time().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), SystemTime::UNIX_EPOCH);
}

#[tokio::test]
async fn last_exposure_start_time_fail_not_set() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.last_exposure_start_time().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn last_exposure_duration_fail_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithLastExposureDuration {
            duration_us: 2_000_000_f64,
        },
    );
    //when
    let res = camera.last_exposure_duration().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 2_f64);
}

#[tokio::test]
async fn last_exposure_duration_fail_not_set() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.last_exposure_duration().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn camera_xsize_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
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
        },
    );
    //when
    let res = camera.camera_xsize().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1920_i32);
}

#[tokio::test]
async fn camera_xsize_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.camera_xsize().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn camera_ysize_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
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
        },
    );
    //when
    let res = camera.camera_ysize().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1080_i32);
}

#[tokio::test]
async fn camera_ysize_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.camera_ysize().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn start_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 100,
                start_y: 0,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.start_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn camera_start_x_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.start_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_start_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
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
        },
    );
    //when
    let res = camera.set_start_x(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.intended_roi.read().await,
        Some(CCDChipArea {
            start_x: 100,
            start_y: 0,
            width: 100,
            height: 100,
        })
    );
}

#[tokio::test]
async fn set_start_x_fail_value_too_small() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched {});
    //when
    let res = camera.set_start_x(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_start_x_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_start_x(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn start_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 100,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.start_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn start_y_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.start_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_start_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
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
        },
    );
    //when
    let res = camera.set_start_y(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.intended_roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 100,
            width: 100,
            height: 100,
        })
    );
}

#[tokio::test]
async fn set_start_y_fail_value_too_small() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched {});
    //when
    let res = camera.set_start_y(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_start_y_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_start_y(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn num_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 10,
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
        },
    );
    //when
    let res = camera.num_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn num_x_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.num_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_num_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.set_num_x(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.intended_roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 10,
        })
    );
}

#[tokio::test]
async fn set_num_x_fail_value_too_small() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched {});
    //when
    let res = camera.set_num_x(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_num_x_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_num_x(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn num_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 100,
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
        },
    );
    //when
    let res = camera.num_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn num_y_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.num_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_num_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.set_num_y(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.intended_roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 10,
            height: 100,
        })
    );
}

#[tokio::test]
async fn set_num_y_fail_value_too_small() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched {});
    //when
    let res = camera.set_num_y(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_num_y_fail_no_roi() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_num_y(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string(),
    )
}

#[tokio::test]
async fn percent_completed_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(5000_u32));
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 10000_f64,
        },
    );
    //when
    let res = camera.percent_completed().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 50_i32);
}
#[tokio::test]
async fn percent_completed_done_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.percent_completed().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn percent_completed_ensure_division() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(std::u32::MIN));
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 0_f64,
        },
    );
    //when
    let res = camera.percent_completed().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn percent_completed_over_9000() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(std::u32::MAX));
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 0_f64,
        },
    );
    //when
    let res = camera.percent_completed().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
}

#[tokio::test]
async fn percent_completed_fail_get_remaining_exposure_us() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetExposureRemainingError)));
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 10000_f64,
        },
    );
    //when
    let res = camera.percent_completed().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn readout_mode_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_readout_mode().once().returning(|| Ok(2));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_mode().await;
    //then
    assert_eq!(res.unwrap(), 2_i32);
}

#[tokio::test]
async fn readout_mode_fail_get_readout_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_readout_mode()
        .once()
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetReadoutModeError {})));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_mode().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn set_readout_mode_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_readout_mode()
        .once()
        .withf(|readout_mode| *readout_mode == 3)
        .returning(|_| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_readout_mode(3_i32).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_readout_mode_fail_set_readout_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_readout_mode()
        .once()
        .withf(|readout_mode| *readout_mode == 3)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::SetReadoutModeError {
                error_code: 123
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_readout_mode(3_i32).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string(),
    )
}

#[tokio::test]
async fn readout_modes_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(1_u32));
    mock.expect_get_readout_mode_name()
        .once()
        .withf(|index| *index == 0)
        .returning(|_| Ok("Standard Mode".to_string()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_modes().await;
    //then
    assert_eq!(res.unwrap(), vec!["Standard Mode"]);
}

#[tokio::test]
async fn readout_modes_fail_get_number_of_readout_modes() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetNumberOfReadoutModesError)));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_modes().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn readout_modes_fail_get_readout_mode_name() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(1_u32));
    mock.expect_get_readout_mode_name()
        .once()
        .withf(|index| *index == 0)
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::GetReadoutModeNameError)));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_modes().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn sensor_type_success_color() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| Some(1));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_type().await;
    //then
    assert_eq!(res.unwrap(), SensorType::RGGB);
}

#[tokio::test]
async fn sensor_type_success_monochrome() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_type().await;
    //then
    assert_eq!(res.unwrap(), SensorType::Monochrome);
}

#[tokio::test]
async fn sensor_type_fail_cam_color_error() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamIsColor)
        .returning(|_| Some(0));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamColor)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_type().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string(),
    )
}

#[tokio::test]
async fn stop_abort() {
    //given
    let camera = new_camera(MockCamera::new(), MockCameraType::Untouched);
    // when / then
    assert!(!camera.can_stop_exposure().await.unwrap());
    assert!(camera.can_abort_exposure().await.unwrap());
}

#[tokio::test]
async fn abort_exposure() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_abort_exposure_and_readout()
        .once()
        .returning(|| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.abort_exposure().await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn abort_exposure_fail_abort_exposure_and_readout() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_abort_exposure_and_readout()
        .once()
        .returning(|| {
            Err(eyre!(qhyccd_rs::QHYError::AbortExposureAndReadoutError {
                error_code: 123
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.abort_exposure().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_negative_exposure() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.start_exposure(-1000_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("duration must be >= 0").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_dark_exposure() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.start_exposure(1000_f64, false).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_operation("dark frames not supported").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_start_x_greater_than_num_x() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 3,
            camera_roi: CCDChipArea {
                start_x: 100,
                start_y: 0,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("StartX > NumX").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_start_y_greater_than_num_y() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 5,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 100,
                width: 10,
                height: 10,
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
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("StartY > NumY").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_num_x_greater_than_camera_x_size() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times: 8,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 50,
                height: 100,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 20,
                image_height: 1080,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("NumX > CameraXSize").to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_num_y_greater_than_camera_y_size() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndRoiAndCCDInfo {
            times: 11,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 50,
                height: 100,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 80,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("NumY > CameraYSize").to_string(),
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
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
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
            camera_binning: BinningMode { symmetric_value: 1 },
            expected_duration: 1000_f64,
        },
    );
    //when
    let res = camera.start_exposure(1000_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_success_1_channel_8_bpp_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: vec![0, 1, 2, 3, 4, 5],
                width: 3,
                height: 2,
                bits_per_pixel: 8,
                channels: 1,
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn start_exposure_fail_1_channel_8_bpp_invalid_vector_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: Vec::new(),
                width: 3,
                height: 2,
                bits_per_pixel: 8,
                channels: 1,
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_success_1_channel_16_bpp_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: vec![0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5],
                width: 3,
                height: 2,
                bits_per_pixel: 16,
                channels: 1,
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn start_exposure_fail_1_channel_16_bpp_invalid_vector_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: Vec::new(),
                width: 3,
                height: 2,
                bits_per_pixel: 16,
                channels: 1,
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_unsupported_channel_16_bpp_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: vec![0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5],
                width: 3,
                height: 2,
                bits_per_pixel: 16,
                channels: 2, //currently only 1 channel is supported
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_1_channel_unsupported_bpp_no_miri() {
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
        .returning(|_| {
            Ok(qhyccd_rs::ImageData {
                data: vec![0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5],
                width: 3,
                height: 2,
                bits_per_pixel: 32,
                channels: 1,
            })
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_set_parameter_no_miri() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_parameter()
        .once()
        .withf(|control, exposure| {
            *control == qhyccd_rs::Control::Exposure && *exposure == 1_000_000_f64
        })
        .returning(|_, _| {
            Err(eyre!(qhyccd_rs::QHYError::SetParameterError {
                error_code: 123
            }))
        });
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
        MockCameraType::WithBinningAndRoiAndCCDInfo {
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
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_start_single_frame_exposure_no_miri() {
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
        .returning(|| {
            Err(eyre!(qhyccd_rs::QHYError::StartSingleFrameExposureError {
                error_code: 123
            }))
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_get_image_size_no_miri() {
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
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetImageSizeError)));
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn start_exposure_fail_get_single_frame_no_miri() {
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
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetSingleFrameError {
                error_code: 123
            }))
        });
    mock.expect_clone().once().return_once(move || clone_mock);
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
                chip_width: 7_f64,
                chip_height: 5_f64,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9_f64,
                pixel_height: 2.9_f64,
                bits_per_pixel: 16,
            },
            camera_binning: BinningMode { symmetric_value: 1 },
        },
    );
    //when
    let res = camera.start_exposure(1_f64, true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string(),
    )
}

#[tokio::test]
async fn pixel_size_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.5,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
        },
    );
    //when
    let res = camera.pixel_size_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 2.5_f64);
}

#[tokio::test]
async fn pixel_size_x_fail_no_ccd_info() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.pixel_size_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn pixel_size_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoiAndCCDInfo {
            times: 1,
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
            camera_ccd_info: CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.5,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            },
        },
    );
    //when
    let res = camera.pixel_size_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 2.9_f64);
}

#[tokio::test]
async fn pixel_size_y_fail_no_ccd_info() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.pixel_size_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn cooler_on_no_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_on().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn set_cooler_on_success_on_already_on() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(16_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_cooler_on_success_off_to_on() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(0_f64));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::ManualPWM
                && (*temp - 1_f64 / 100_f64 * 255_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_cooler_on_fail_off_to_on_set_parameter() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(0_f64));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::ManualPWM
                && (*temp - 1_f64 / 100_f64 * 255_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| {
            Err(eyre!(qhyccd_rs::QHYError::SetParameterError {
                error_code: 123
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string()
    )
}

#[tokio::test]
async fn set_cooler_on_fail_off_to_on_cooler_on_error() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetParameterError {
                control: qhyccd_rs::Control::CurPWM,
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(true).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_cooler_on_success_on_to_off() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(10_f64));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::ManualPWM && (*temp - 0_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(false).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_cooler_on_success_off_already_off() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(0_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(false).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_cooler_on_fail_on_to_off_set_parameter() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(10_f64));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::ManualPWM && (*temp - 0_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| {
            Err(eyre!(qhyccd_rs::QHYError::SetParameterError {
                error_code: 123
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(false).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string()
    )
}

#[tokio::test]
async fn set_cooler_on_fail_on_to_off_cooler_on_error() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetParameterError {
                control: qhyccd_rs::Control::CurPWM,
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_cooler_on(false).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn cooler_power_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| Ok(25_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_power().await;
    //then
    assert!((res.unwrap() - 25_f64 / 255_f64 * 100_f64).abs() < f64::EPSILON);
}

#[tokio::test]
async fn cooler_power_fail_get_parameter() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurPWM)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetParameterError {
                control: qhyccd_rs::Control::CurPWM,
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_power().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn cooler_power_fail_no_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.cooler_power().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn can_set_ccd_temperature_success_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.can_set_ccd_temperature().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn can_set_ccd_temperature_success_non_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.can_set_ccd_temperature().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn ccd_temperature_success_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurTemp)
        .returning(|_| Ok(25_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.ccd_temperature().await;
    //then
    assert!(res.is_ok());
    assert!((res.unwrap() - 25_f64).abs() < f64::EPSILON);
}

#[tokio::test]
async fn ccd_temperature_fail_get_parameter() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurTemp)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::GetParameterError {
                control: qhyccd_rs::Control::CurTemp,
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.ccd_temperature().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_ccd_temperature_success_cooler_temperature_already_set() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    let camera = new_camera(
        mock,
        MockCameraType::WithTargetTemperature {
            times: 1,
            temperature: -2_f64,
        },
    );
    //when
    let res = camera.set_ccd_temperature().await;
    //then
    assert!((res.unwrap() - -2_f64).abs() < f64::EPSILON);
}

#[tokio::test]
async fn set_ccd_temperature_success_cooler_temperature_not_yet_set() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CurTemp)
        .returning(|_| Ok(25_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.set_ccd_temperature().await;
    //then
    assert!((res.unwrap() - 25_f64).abs() < f64::EPSILON);
}

#[tokio::test]
async fn set_ccd_temperature_fail_no_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_ccd_temperature().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn set_set_ccd_temperature_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::Cooler && (*temp - -2_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_set_ccd_temperature(-2_f64).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_set_ccd_temperature_fail_no_cooler() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_set_ccd_temperature(-2_f64).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn set_set_ccd_temperature_fail_set_parameter_error() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Cooler)
        .returning(|_| Some(0));
    mock.expect_set_parameter()
        .once()
        .withf(|control, temp| {
            *control == qhyccd_rs::Control::Cooler && (*temp - -2_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| {
            Err(eyre!(qhyccd_rs::QHYError::SetParameterError {
                error_code: 123
            }))
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_set_ccd_temperature(-2_f64).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_set_ccd_temperature_fail_ascom_check() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.set_set_ccd_temperature(-273.25_f64).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );

    //when
    let res = camera.set_set_ccd_temperature(100_f64).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn gain_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    mock.expect_get_parameter()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Ok(25_f64));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.gain().await;
    //then
    assert_eq!(res.unwrap(), 25_i32);
}

#[tokio::test]
async fn gain_fail_not_implemented() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.gain().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn set_gain_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    mock.expect_set_parameter()
        .once()
        .withf(|control, gain| {
            *control == qhyccd_rs::Control::Gain && (*gain - 25_f64).abs() < f64::EPSILON
        })
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: 1,
            gain_min: 0_f64,
            gain_max: 51_f64,
        },
    );
    //when
    let res = camera.set_gain(25_i32).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_gain_fail_no_min_max() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_gain(25_i32).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::unspecified("gamera reports gain control available, but min, max values are not set after initialization").to_string()
    )
}

#[tokio::test]
async fn set_gain_fail_invalid_gain_value() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .times(2)
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| Some(0));
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: 2,
            gain_min: 0_f64,
            gain_max: 51_f64,
        },
    );
    //when
    let res = camera.set_gain(-1_i32).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );

    //when
    let res = camera.set_gain(60_i32).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    )
}

#[tokio::test]
async fn set_gain_fail_no_gain_control() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Gain)
        .returning(|_| None);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_gain(25_i32).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn gain_min_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: 1,
            gain_min: 0_f64,
            gain_max: 51_f64,
        },
    );
    //when
    let res = camera.gain_min().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn gain_min_fail_no_gain_control() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.gain_min().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}

#[tokio::test]
async fn gain_max_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithGain {
            times: 1,
            gain_min: 0_f64,
            gain_max: 51_f64,
        },
    );
    //when
    let res = camera.gain_max().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 51_i32);
}

#[tokio::test]
async fn gain_max_fail_no_gain_control() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.gain_max().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    )
}
