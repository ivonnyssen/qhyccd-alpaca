//! Region of Interest and geometry tests

use super::*;

#[rstest]
#[case(true, Ok(1920_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn camera_xsize(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: if has_roi {
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
    let res = camera.camera_x_size().await;
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
#[case(true, Ok(1080_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn camera_ysize(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithCCDInfo {
            times: 1,
            camera_ccd_info: if has_roi {
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
    let res = camera.camera_y_size().await;
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
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn start_x(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 10,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.start_x().await;
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
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_start_x(
    #[case] x: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_start_x(x).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: x,
                start_y: 0,
                width: 100,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn start_y(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 100,
                    width: 10,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.start_y().await;
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
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_start_y(
    #[case] y: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_start_y(y).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: y,
                width: 100,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(1000_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn num_x(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 1000,
                    height: 10,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.num_x().await;
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
#[case(1000_u32, 1, true, Ok(()))]
#[case(1000_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_num_x(
    #[case] w: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_num_x(w).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: w,
                height: 100,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}

#[rstest]
#[case(true, Ok(100_u32))]
#[case(false, Err(ASCOMError::VALUE_NOT_SET))]
#[tokio::test]
async fn num_y(#[case] has_roi: bool, #[case] expected: ASCOMResult<u32>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times: 1,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 100,
                    start_y: 0,
                    width: 10,
                    height: 100,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.num_y().await;
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
#[case(100_u32, 1, true, Ok(()))]
#[case(100_u32, 1, false, Err(ASCOMError::INVALID_VALUE))]
#[tokio::test]
async fn set_num_y(
    #[case] h: u32,
    #[case] times: usize,
    #[case] has_roi: bool,
    #[case] expected: ASCOMResult<()>,
) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithRoi {
            times,
            camera_roi: if has_roi {
                Some(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 1001,
                    height: 1000,
                })
            } else {
                None
            },
        },
    );
    //when
    let res = camera.set_num_y(h).await;
    //then
    if expected.is_ok() {
        assert_eq!(
            *camera.intended_roi.read().await,
            Some(CCDChipArea {
                start_x: 0,
                start_y: 0,
                width: 1001,
                height: h,
            })
        );
    } else {
        assert_eq!(
            expected.clone().unwrap_err().to_string(),
            res.unwrap_err().to_string()
        );
    }
}
