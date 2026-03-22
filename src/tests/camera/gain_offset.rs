//! Gain and offset management tests

use super::*;

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
