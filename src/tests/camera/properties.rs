//! Camera metadata and property tests

use super::*;

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
