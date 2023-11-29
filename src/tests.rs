use qhyccd_rs::Control;

use super::*;
use crate::mocks::MockCamera;
use eyre::eyre;

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

fn new_camera(device: MockCamera) -> QhyccdCamera {
    QhyccdCamera {
        unique_id: "test_camera".to_owned(),
        name: "QHYCCD-test_camera".to_owned(),
        description: "QHYCCD camera".to_owned(),
        device,
        binning: RwLock::new(BinningMode { symmetric_value: 1 }),
        valid_bins: RwLock::new(None),
        roi: RwLock::new(None),
        last_exposure_start_time: RwLock::new(None),
        last_exposure_duration_us: RwLock::new(None),
        last_image: RwLock::new(None),
        exposing: RwLock::new(ExposingState::Idle),
    }
}

#[tokio::test]
async fn max_bin_x_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(2).returning(|| Ok(true));
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
            Control::CamBin1x1mode => Ok(0_u32),
            Control::CamBin2x2mode => Ok(0_u32),
            Control::CamBin3x3mode => Ok(0_u32),
            Control::CamBin4x4mode => Ok(0_u32),
            Control::CamBin6x6mode => Ok(0_u32),
            Control::CamBin8x8mode => Ok(0_u32),
            _ => panic!("Unexpected control"),
        });
    //when
    let camera = new_camera(mock);
    //then
    assert_eq!(camera.max_bin_x().await.unwrap(), 8);
    assert_eq!(camera.max_bin_y().await.unwrap(), 8);
}

#[tokio::test]
async fn max_bin_x_fail_no_modes() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(2).returning(|| Ok(true));
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
        .returning(|control| {
            Err(eyre!(qhyccd_rs::QHYError::IsControlAvailableError {
                feature: control
            }))
        });
    //when
    let camera = new_camera(mock);
    //then
    assert!(camera.max_bin_x().await.is_err());
    assert!(camera.max_bin_y().await.is_err());
}

#[tokio::test]
async fn max_bin_x_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(2).returning(|| Ok(false));
    //when
    let camera = new_camera(mock);
    //then
    assert!(camera.max_bin_x().await.is_err());
    assert!(camera.max_bin_y().await.is_err());
}

#[tokio::test]
async fn camrea_state_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    let camera = new_camera(mock);
    //when
    let res = camera.camera_state().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), CameraState::Idle);
}

#[tokio::test]
async fn camera_state_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.camera_state().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    )
}

#[tokio::test]
async fn connected_fail() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open()
        .times(1)
        .returning(|| Err(eyre!("Could not acquire read lock on camera handle")));
    let camera = new_camera(mock);
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
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    let camera = new_camera(mock);
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_already_disconnected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.set_connected(false).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_true_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
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
            Control::CamBin1x1mode => Ok(0_u32),
            Control::CamBin2x2mode => Ok(0_u32),
            Control::CamBin3x3mode => Ok(0_u32),
            Control::CamBin4x4mode => Ok(0_u32),
            Control::CamBin6x6mode => Ok(0_u32),
            Control::CamBin8x8mode => Ok(0_u32),
            _ => panic!("Unexpected control"),
        });
    let camera = new_camera(mock);
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_false_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_close().times(1).returning(|| Ok(()));
    let camera = new_camera(mock);
    //when
    let res = camera.set_connected(false).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_fail_open() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    mock.expect_open()
        .times(1)
        .returning(|| Err(eyre!("Could not open camera")));
    let camera = new_camera(mock);
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
    mock.expect_is_open().times(1).returning(|| Ok(false));
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
            Control::CamBin1x1mode => Ok(0_u32),
            Control::CamBin2x2mode => Ok(0_u32),
            Control::CamBin3x3mode => Ok(0_u32),
            Control::CamBin4x4mode => Ok(0_u32),
            Control::CamBin6x6mode => Ok(0_u32),
            Control::CamBin8x8mode => Ok(0_u32),
            _ => panic!("Unexpected control"),
        });
    let camera = new_camera(mock);
    //when
    let res = camera.set_connected(true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_fail_close() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_close()
        .times(1)
        .returning(|| Err(eyre!("Could not close camera")));
    let camera = new_camera(mock);
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
    let camera = new_camera(mock);
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
    let camera = new_camera(mock);
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
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_get_model()
        .once()
        .returning(|| Ok("test_model".to_owned()));
    let camera = new_camera(mock);
    //when
    let res = camera.sensor_name().await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "test_model");
}

#[tokio::test]
async fn sensor_name_fail_get_model() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_get_model()
        .once()
        .returning(|| Err(eyre!("Could not get model")));
    let camera = new_camera(mock);
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
async fn sensor_name_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.sensor_name().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn bin_x_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    let camera = new_camera(mock);
    //when
    let res = camera.bin_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn bin_y_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    let camera = new_camera(mock);
    //when
    let res = camera.bin_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_i32);
}

#[tokio::test]
async fn bin_x_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.bin_x().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn bin_y_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.bin_y().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_bin_x_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 1 && *bin_y == 1)
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock);
    //when
    let res = camera.set_bin_x(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_y_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 1 && *bin_y == 1)
        .returning(|_, _| Ok(()));
    let camera = new_camera(mock);
    //when
    let res = camera.set_bin_y(1).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_bin_x_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.set_bin_x(1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_bin_y_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.set_bin_y(1).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_bin_x_fail_set_bin_mode() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Err(eyre!("Could not set bin mode")));
    let camera = new_camera(mock);
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
    mock.expect_is_open().times(1).returning(|| Ok(true));
    mock.expect_set_bin_mode()
        .once()
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Err(eyre!("Could not set bin mode")));
    let camera = new_camera(mock);
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
    let camera = new_camera(mock);
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
    let camera = new_camera(mock);
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
    let camera = new_camera(mock);
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
    mock.expect_is_open().once().returning(|| Ok(true));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamMechanicalShutter)
        .returning(|_| Ok(0_u32));
    let camera = new_camera(mock);
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
    mock.expect_is_open().once().returning(|| Ok(true));
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::CamMechanicalShutter)
        .returning(|_| {
            Err(eyre!(qhyccd_rs::QHYError::IsControlAvailableError {
                feature: qhyccd_rs::Control::CamMechanicalShutter
            }))
        });
    let camera = new_camera(mock);
    //when
    let res = camera.has_shutter().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn has_shutter_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.has_shutter().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn image_array_empty() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(true));
    let camera = new_camera(mock);
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
async fn image_array_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.image_array().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn image_ready_not_ready_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(true));
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(10000_u32));
    let camera = new_camera(mock);
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
    mock.expect_is_open().once().returning(|| Ok(true));
    mock.expect_get_remaining_exposure_us()
        .once()
        .returning(|| Ok(0_u32));
    let camera = new_camera(mock);
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn image_ready_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    )
}

#[tokio::test]
async fn last_exposure_duration_fail_not_set() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(true));
    let camera = new_camera(mock);
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
async fn last_exposure_duration_fail_not_connected() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open().once().returning(|| Ok(false));
    let camera = new_camera(mock);
    //when
    let res = camera.last_exposure_duration().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    )
}
