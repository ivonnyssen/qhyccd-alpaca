//! Readout mode and fast readout tests

use super::*;

#[rstest]
#[case(Ok(0_u32), Ok(0_usize))]
#[case(Ok(2_u32), Ok(2_usize))]
#[case(Err(eyre!("error")), Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn readout_mode(
    #[case] get_readout_mode: Result<u32>,
    #[case] expected: ASCOMResult<usize>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_readout_mode()
        .once()
        .return_once(move || get_readout_mode);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_mode().await;
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

#[tokio::test]
async fn set_readout_mode_success() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(3));
    mock.expect_get_readout_mode_resolution()
        .once()
        .returning(|_| Ok((1920, 1080)));
    mock.expect_set_readout_mode()
        .once()
        .returning(|_| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: Some(CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            }),
        },
    );
    //when
    let res = camera.set_readout_mode(0).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_readout_mode_fail_get_number() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Err(eyre!("error")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_readout_mode(0).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn set_readout_mode_fail_out_of_range() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(2));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_readout_mode(3).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn set_readout_mode_fail_get_resolution() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(3));
    mock.expect_get_readout_mode_resolution()
        .once()
        .returning(|_| Err(eyre!("error")));
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_readout_mode(0).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn set_readout_mode_fail_set() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_get_number_of_readout_modes()
        .once()
        .returning(|| Ok(3));
    mock.expect_get_readout_mode_resolution()
        .once()
        .returning(|_| Ok((1920, 1080)));
    mock.expect_set_readout_mode()
        .once()
        .returning(|_| Err(eyre!("error")));
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: Some(CCDChipInfo {
                chip_width: 7.0,
                chip_height: 5.0,
                image_width: 1920,
                image_height: 1080,
                pixel_width: 2.9,
                pixel_height: 2.9,
                bits_per_pixel: 16,
            }),
        },
    );
    //when
    let res = camera.set_readout_mode(0).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[rstest]
#[case(Ok(2_u32), vec![Ok("Mode0".to_owned()), Ok("Mode1".to_owned())], Ok(vec!["Mode0".to_owned(), "Mode1".to_owned()]))]
#[case(Err(eyre!("error")), vec![], Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn readout_modes(
    #[case] get_number: Result<u32>,
    #[case] mode_names: Vec<Result<String>>,
    #[case] expected: ASCOMResult<Vec<String>>,
) {
    //given
    let mut mock = MockCamera::new();
    let name_count = mode_names.len();
    mock.expect_get_number_of_readout_modes()
        .once()
        .return_once(move || get_number);
    let names_iter = std::sync::Mutex::new(mode_names.into_iter());
    mock.expect_get_readout_mode_name()
        .times(name_count)
        .returning(move |_| names_iter.lock().unwrap().next().unwrap());
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.readout_modes().await;
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
#[case(Some(0), true, Ok(true))]
#[case(Some(0), false, Ok(false))]
#[case(None, false, Ok(false))]
#[tokio::test]
async fn can_fast_readout(
    #[case] speed_available: Option<u32>,
    #[case] has_min_max: bool,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| speed_available);
    let camera = new_camera(
        mock,
        MockCameraType::WithReadoutMinMax {
            times: 1,
            min_max_step: if has_min_max {
                Some((0.0, 2.0, 1.0))
            } else {
                None
            },
        },
    );
    //when
    let res = camera.can_fast_readout().await;
    //then
    assert_eq!(res.unwrap(), expected.unwrap());
}

#[rstest]
#[case(true, Ok(2.0_f64), 1, Some((0.0, 2.0, 1.0)), Ok(true))]
#[case(true, Ok(0.0_f64), 1, Some((0.0, 2.0, 1.0)), Ok(false))]
#[case(true, Err(eyre!("error")), 1, Some((0.0, 2.0, 1.0)), Err(ASCOMError::INVALID_OPERATION))]
#[case(false, Ok(2.0_f64), 0, Some((0.0, 2.0, 1.0)), Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(true, Ok(2.0_f64), 1, None, Err(ASCOMError::INVALID_OPERATION))]
#[tokio::test]
async fn fast_readout(
    #[case] speed_available: bool,
    #[case] get_speed: Result<f64>,
    #[case] get_speed_times: usize,
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] expected: ASCOMResult<bool>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| if speed_available { Some(0) } else { None });
    mock.expect_get_parameter()
        .times(get_speed_times)
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .return_once(move |_| get_speed);
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
#[case(true, true, Some((0.0, 2.0, 1.0)), Ok(()), 1, Ok(()))]
#[case(true, false, Some((0.0, 2.0, 1.0)), Ok(()), 1, Ok(()))]
#[case(false, true, Some((0.0, 2.0, 1.0)), Ok(()), 0, Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(true, true, Some((0.0, 2.0, 1.0)), Err(eyre!("error")), 1, Err(ASCOMError::INVALID_OPERATION))]
#[case(true, true, None, Ok(()), 0, Err(ASCOMError::invalid_operation("camera reports readout speed control available, but min, max values are not set after initialization")))]
#[tokio::test]
async fn set_fast_readout(
    #[case] speed_available: bool,
    #[case] fast: bool,
    #[case] min_max_step: Option<(f64, f64, f64)>,
    #[case] set_result: Result<()>,
    #[case] set_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_control_available()
        .once()
        .withf(|control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| if speed_available { Some(0) } else { None });
    mock.expect_set_parameter()
        .times(set_times)
        .withf(move |control, value| {
            *control == qhyccd_rs::Control::Speed
                && (*value - if fast { 2.0 } else { 0.0 }).abs() < f64::EPSILON
        })
        .return_once(move |_, _| set_result);
    let camera = new_camera(
        mock,
        MockCameraType::WithReadoutMinMax {
            times: 1,
            min_max_step,
        },
    );
    //when
    let res = camera.set_fast_readout(fast).await;
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
