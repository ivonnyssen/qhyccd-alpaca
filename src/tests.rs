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
    not_connected! {stop_exposure()}
    not_connected! {abort_exposure()}
}

enum MockCameraType {
    IsOpenTrue { times: usize },
    IsOpenFalse { times: usize },
    WithRoi { camera_roi: CCDChipArea },
    Untouched,
    Exposing { expected_duration: f64 },
    WithImage { image_array: ImageArray },
    WithLastExposureStart { start_time: SystemTime },
    WithLastExposureDuration { duration_us: f64 },
}

fn new_camera(mut device: MockCamera, variant: MockCameraType) -> QhyccdCamera {
    let mut roi = RwLock::new(None);
    let mut exposing = RwLock::new(ExposingState::Idle);
    let mut last_exposure_start_time = RwLock::new(None);
    let mut last_exposure_duration_us = RwLock::new(None);
    let mut last_image = RwLock::new(None);
    match variant {
        MockCameraType::IsOpenTrue { times } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
        }
        MockCameraType::IsOpenFalse { times } => {
            device.expect_is_open().times(times).returning(|| Ok(false));
        }
        MockCameraType::WithRoi { camera_roi } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            roi = RwLock::new(Some(camera_roi));
        }
        MockCameraType::Untouched => {}
        MockCameraType::Exposing { expected_duration } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            exposing = RwLock::new(ExposingState::Exposing {
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
        MockCameraType::WithLastExposureStart { start_time } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_start_time = RwLock::new(Some(start_time));
        }
        MockCameraType::WithLastExposureDuration { duration_us } => {
            device.expect_is_open().times(1).returning(|| Ok(true));
            last_exposure_duration_us = RwLock::new(Some(duration_us as u32));
        }
    }
    QhyccdCamera {
        unique_id: "test_camera".to_owned(),
        name: "QHYCCD-test_camera".to_owned(),
        description: "QHYCCD camera".to_owned(),
        device,
        binning: RwLock::new(BinningMode { symmetric_value: 1 }),
        valid_bins: RwLock::new(None),
        roi,
        last_exposure_start_time,
        last_exposure_duration_us,
        last_image,
        exposing,
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
        roi: RwLock::new(None),
        last_exposure_start_time: RwLock::new(None),
        last_exposure_duration_us: RwLock::new(None),
        last_image: RwLock::new(None),
        exposing: RwLock::new(ExposingState::Idle),
    };
    //then
    assert_eq!(camera.unique_id, "test_camera");
    assert_eq!(camera.name, "QHYCCD-test_camera");
    assert_eq!(camera.description, "QHYCCD camera");
    assert_eq!(camera.binning.read().await.symmetric_value, 1);
    assert!(camera.valid_bins.read().await.is_none());
    assert!(camera.roi.read().await.is_none());
    assert!(camera.last_exposure_start_time.read().await.is_none());
    assert!(camera.last_exposure_duration_us.read().await.is_none());
    assert!(camera.last_image.read().await.is_none());
    assert_eq!(*camera.exposing.read().await, ExposingState::Idle);
    assert_eq!(camera.static_name(), "QHYCCD-test_camera");
    assert_eq!(camera.unique_id(), "test_camera");
    assert_eq!(camera.description().await.unwrap(), "QHYCCD camera");
    assert_eq!(camera.driver_info().await.unwrap(), "qhyccd_alpaca driver");
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
        MockCameraType::Exposing {
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
async fn set_connected_fail_get_effective_area() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_get_effective_area()
        .once()
        .returning(|| Err(eyre!("could not get effective area")));
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
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_ok());
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

#[tokio::test]
async fn offset_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.bayer_offset_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn offset_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.bayer_offset_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0_i32);
}

#[tokio::test]
async fn sensor_name_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_model()
        .once()
        .returning(|| Ok("test_model".to_owned()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_name().await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "test_model");
}

#[tokio::test]
async fn sensor_name_fail_get_model() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_model()
        .once()
        .returning(|| Err(eyre!("Could not get model")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_name().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string()
    );
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
async fn set_bin_x_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 1 && *bin_y == 1)
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_bin_x(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_y_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 1 && *bin_y == 1)
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
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
async fn set_bin_y_fail_set_bin_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .once()
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Err(eyre!("Could not set bin mode")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_bin_y(2).await;
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
        ASCOMError::invalid_value("bin_x must be >= 1").to_string()
    );
}

#[tokio::test]
async fn set_bin_y_fail_invalid_bin() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::Untouched);
    //when
    let res = camera.set_bin_y(0).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("bin_x must be >= 1").to_string()
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
        camera.exposure_max().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera.exposure_min().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera
            .exposure_resolution()
            .await
            .err()
            .unwrap()
            .to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera.full_well_capacity().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
    );
    assert_eq!(
        camera.max_adu().await.err().unwrap().to_string(),
        ASCOMError::NOT_IMPLEMENTED.to_string()
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
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(10000_u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn image_ready_ready_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(0_u32));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn image_ready_fail_get_remaining_exposure_us() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetExposureRemainingError)));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string()
    );
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
            duration_us: 1000_f64,
        },
    );
    //when
    let res = camera.last_exposure_duration().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1000_f64);
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
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.camera_xsize().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
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
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.camera_ysize().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 100_i32);
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
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 100,
                start_y: 0,
                width: 10,
                height: 10,
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
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 100,
                start_y: 0,
                width: 100,
                height: 100,
            }
        })
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.set_start_x(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.roi.read().await,
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
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 0 });
    //when
    let res = camera.set_start_x(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("start_x must be >= 0").to_string()
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
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_start_x_fail_set_roi() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 100,
                start_y: 0,
                width: 100,
                height: 100,
            }
        })
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetRoiError { error_code: 123 })));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.set_start_x(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        *camera.roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    );
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn start_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 100,
                width: 10,
                height: 10,
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
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 100,
                width: 100,
                height: 100,
            }
        })
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.set_start_y(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.roi.read().await,
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
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 0 });
    //when
    let res = camera.set_start_y(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("start_y must be >= 0").to_string()
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
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_start_y_fail_set_roi() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 100,
                width: 100,
                height: 100,
            }
        })
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetRoiError { error_code: 123 })));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            },
        },
    );
    //when
    let res = camera.set_start_y(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        *camera.roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 100,
            height: 100,
        })
    );
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn num_x_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 10,
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
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 10,
            }
        })
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 10,
            },
        },
    );
    //when
    let res = camera.set_num_x(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.roi.read().await,
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
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 0 });
    //when
    let res = camera.set_num_x(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("num_x must be >= 0").to_string()
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
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn set_num_x_fail_set_roi() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 100,
                height: 100,
            }
        })
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetRoiError { error_code: 123 })));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 100,
            },
        },
    );
    //when
    let res = camera.set_num_x(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        *camera.roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 10,
            height: 100,
        })
    );
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    )
}

#[tokio::test]
async fn num_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 100,
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
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 100,
            }
        })
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 10,
            },
        },
    );
    //when
    let res = camera.set_num_y(100).await;
    //then
    assert!(res.is_ok());
    assert_eq!(
        *camera.roi.read().await,
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
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 0 });
    //when
    let res = camera.set_num_y(-1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::invalid_value("num_y must be >= 0").to_string()
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
        ASCOMError::VALUE_NOT_SET.to_string(),
    )
}

#[tokio::test]
async fn set_num_y_fail_set_roi() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_roi()
        .once()
        .withf(|roi| {
            *roi == CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 100,
            }
        })
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetRoiError { error_code: 123 })));
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            camera_roi: CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 10,
                height: 10,
            },
        },
    );
    //when
    let res = camera.set_num_y(100).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        *camera.roi.read().await,
        Some(CCDChipArea {
            start_x: 0,
            start_y: 0,
            width: 10,
            height: 10,
        })
    );
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string(),
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
        MockCameraType::Exposing {
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
        MockCameraType::Exposing {
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
        MockCameraType::Exposing {
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
        MockCameraType::Exposing {
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
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.sensor_type().await;
    //then
    assert_eq!(res.unwrap(), SensorType::Color);
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
async fn stop_abort() {
    //given
    let camera = new_camera(MockCamera::new(), MockCameraType::Untouched);
    // when / then
    assert!(!camera.can_stop_exposure().await.unwrap());
    assert!(camera.can_abort_exposure().await.unwrap());
}

#[tokio::test]
async fn stop_exposure() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_stop_exposure().once().returning(|| Ok(()));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.stop_exposure().await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn stop_exposure_fail_stop_exposure() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_stop_exposure().once().returning(|| {
        Err(eyre!(qhyccd_rs::QHYError::StopExposureError {
            error_code: 123
        }))
    });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.stop_exposure().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string(),
    )
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
        ASCOMError::NOT_CONNECTED.to_string(),
    )
}
