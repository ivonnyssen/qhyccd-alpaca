//! Sensor properties and readout mode tests

use super::*;

#[rstest]
#[case(Some(0), Some(qhyccd_rs::BayerMode::GBRG as u32), 2, Ok(0_u8), Ok(1_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::GRBG as u32), 2, Ok(1_u8), Ok(0_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::BGGR as u32), 2, Ok(1_u8), Ok(1_u8))]
#[case(Some(0), Some(qhyccd_rs::BayerMode::RGGB as u32), 2, Ok(0_u8), Ok(0_u8))]
#[case(None, Some(qhyccd_rs::BayerMode::RGGB as u32), 0, Err(ASCOMError::NOT_IMPLEMENTED), Err(ASCOMError::NOT_IMPLEMENTED))]
#[case(
    Some(0),
    Some(0_u32),
    2,
    Err(ASCOMError::INVALID_VALUE),
    Err(ASCOMError::INVALID_VALUE)
)]
#[case(
    Some(0),
    None,
    2,
    Err(ASCOMError::INVALID_VALUE),
    Err(ASCOMError::INVALID_VALUE)
)]
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
