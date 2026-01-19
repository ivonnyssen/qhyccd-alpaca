//! Binning operations tests

use super::*;

#[rstest]
#[case(true, Ok(8_u8), Ok(8_u8))]
#[case(
    false,
    Err(ASCOMError::INVALID_OPERATION),
    Err(ASCOMError::INVALID_OPERATION)
)]
#[tokio::test]
async fn max_bin_xy(
    #[case] has_modes: bool,
    #[case] expected_x: ASCOMResult<u8>,
    #[case] expected_y: ASCOMResult<u8>,
) {
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
        .returning(move |control| {
            if has_modes {
                match control {
                    Control::CamBin1x1mode => Some(0_u32),
                    Control::CamBin2x2mode => Some(0_u32),
                    Control::CamBin3x3mode => Some(0_u32),
                    Control::CamBin4x4mode => Some(0_u32),
                    Control::CamBin6x6mode => Some(0_u32),
                    Control::CamBin8x8mode => Some(0_u32),
                    _ => panic!("Unexpected control"),
                }
            } else {
                None
            }
        });
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 2 });
    //when
    let res = camera.max_bin_x().await;
    //then
    if expected_x.is_ok() {
        assert_eq!(res.unwrap(), expected_x.unwrap());
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected_x.unwrap_err().to_string()
        );
    }

    //when
    let res = camera.max_bin_y().await;
    //then
    if expected_y.is_ok() {
        assert_eq!(res.unwrap(), expected_y.unwrap());
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected_y.unwrap_err().to_string()
        );
    }
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
async fn bin_x_y_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBins {
            times: 2,
            camera_valid_bins: { vec![1_u8, 2_u8] },
            camera_binning: 1_u8,
        },
    );
    //when
    let res = camera.bin_x().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_u8);

    //when
    let res = camera.bin_y().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 1_u8);
}

#[rstest]
#[case(true, 1, vec![1, 2], 1, Ok(()), 0, Ok(()))]
#[case(true, 2, vec![1, 2], 1, Ok(()), 1, Ok(()))]
#[case(true, 2, vec![1, 2], 1, Err(eyre!("error")), 1, Err(ASCOMError::VALUE_NOT_SET))]
#[case(true, 0, vec![1, 2], 1, Ok(()), 0, Err(ASCOMError::invalid_value("bin value must be one of the valid bins")))]
#[case(false, 1, vec![1, 2], 1, Ok(()), 0, Ok(()))]
#[case(false, 2, vec![1, 2], 1, Ok(()), 1, Ok(()))]
#[case(false, 2, vec![1, 2], 1, Err(eyre!("error")), 1, Err(ASCOMError::VALUE_NOT_SET))]
#[case(false, 0, vec![1, 2], 1, Ok(()), 0, Err(ASCOMError::invalid_value("bin value must be one of the valid bins")))]
#[tokio::test]
async fn set_bin_x_y(
    #[case] x: bool,
    #[case] bin: u32,
    #[case] camera_valid_bins: Vec<u8>,
    #[case] camera_binning: u8,
    #[case] set_bin_mode: Result<()>,
    #[case] set_bin_mode_times: usize,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(set_bin_mode_times)
        .withf(move |x: &u32, y: &u32| *x == bin && *y == bin)
        .return_once(move |_, _| set_bin_mode);
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBins {
            times: 1,
            camera_valid_bins,
            camera_binning,
        },
    );
    //when
    let res = if x {
        camera.set_bin_x(bin as u8).await
    } else {
        camera.set_bin_y(bin as u8).await
    };
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
#[case(10, 20, 5, 10)]
#[case(5, 11, 2, 5)]
#[tokio::test]
async fn set_bin_x_with_roi(
    #[case] start_x: u32,
    #[case] start_y: u32,
    #[case] expected_start_x: u32,
    #[case] expected_start_y: u32,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_set_bin_mode()
        .times(1)
        .withf(|bin_x: &u32, bin_y: &u32| *bin_x == 2 && *bin_y == 2)
        .returning(|_, _| Ok(()));
    let camera = new_camera(
        mock,
        MockCameraType::WithBinningAndValidBinsAndRoiAndCCDInfo {
            times: 9,
            camera_roi: CCDChipArea {
                start_x,
                start_y,
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
            camera_valid_bins: { vec![1_u8, 2_u8] },
        },
    );
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_ok());
    assert_eq!(camera.camera_x_size().await.unwrap(), 1920_u32);
    assert_eq!(camera.camera_y_size().await.unwrap(), 1080_u32);
    assert_eq!(camera.bin_x().await.unwrap(), 2_u8);
    assert_eq!(camera.bin_y().await.unwrap(), 2_u8);
    assert_eq!(camera.start_x().await.unwrap(), expected_start_x);
    assert_eq!(camera.start_y().await.unwrap(), expected_start_y);
    assert_eq!(camera.num_x().await.unwrap(), 960_u32);
    assert_eq!(camera.num_y().await.unwrap(), 540_u32);
}

#[tokio::test]
async fn set_bin_x_fail_no_valid_bins() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_bin_x(2).await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}
