//! Temperature control tests

use super::*;

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
