//! Exposure control tests

use super::*;

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
#[case(Ok(5_000_u32), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 10_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(50_u8))]
#[case(Ok(10_000_u32), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 10_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(100_u8))]
#[case(Ok(10_000_u32), 0, State::Idle {}, Ok(100_u8))]
#[case(Ok(u32::MIN), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 0_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(0_u8))]
#[case(Ok(u32::MAX), 1, State::Exposing { start: SystemTime::UNIX_EPOCH, expected_duration_us: 0_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(100_u8))]
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
