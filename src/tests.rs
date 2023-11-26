use qhyccd_rs::Control;

use super::*;
use crate::mocks::MockCamera;
use eyre::eyre;

#[tokio::test]
async fn qhyccd_camera() {
    let mut mock = MockCamera::new();
    mock.expect_id()
        .times(2)
        .return_const("test_camera".to_owned());
    mock.expect_clone().returning(MockCamera::new);
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
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
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
            Control::CamBin6x6mode => Err(eyre!(qhyccd_rs::QHYError::IsControlAvailableError {
                feature: Control::CamBin6x6mode
            })),
            Control::CamBin8x8mode => Err(eyre!(qhyccd_rs::QHYError::IsControlAvailableError {
                feature: Control::CamBin8x8mode
            })),
            _ => panic!("Unexpected control"),
        });
    let camera = new_camera(mock);
    assert_eq!(camera.max_bin_x().await.unwrap(), 4);
}

#[tokio::test]
async fn max_bin_x_fail_no_modes() {
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(true));
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
        .returning(|control| {
            Err(eyre!(qhyccd_rs::QHYError::IsControlAvailableError {
                feature: control
            }))
        });
    let camera = new_camera(mock);
    assert!(camera.max_bin_x().await.is_err());
}

#[tokio::test]
async fn max_bin_x_fail_not_connected() {
    let mut mock = MockCamera::new();
    mock.expect_is_open().times(1).returning(|| Ok(false));
    let camera = new_camera(mock);
    assert!(camera.max_bin_x().await.is_err());
}
