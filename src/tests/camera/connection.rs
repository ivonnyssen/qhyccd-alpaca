//! Connection state management tests

use super::*;

#[tokio::test]
async fn not_connected_asyncs() {
    not_connected! {sensor_type()}
    not_connected! {max_bin_x()}
    not_connected! {max_bin_y()}
    not_connected! {sensor_name()}
    not_connected! {camera_state()}
    not_connected! {bin_x()}
    not_connected! {bin_y()}
    not_connected! {set_bin_x(1)}
    not_connected! {set_bin_y(1)}
    not_connected! {has_shutter()}
    not_connected! {image_array()}
    not_connected! {image_ready()}
    not_connected! {last_exposure_start_time()}
    not_connected! {last_exposure_duration()}
    not_connected! {camera_x_size()}
    not_connected! {camera_y_size()}
    not_connected! {start_x()}
    not_connected! {set_start_x(100)}
    not_connected! {start_y()}
    not_connected! {set_start_y(100)}
    not_connected! {num_x()}
    not_connected! {set_num_x(100)}
    not_connected! {num_y()}
    not_connected! {set_num_y(100)}
    not_connected! {readout_mode()}
    not_connected! {set_readout_mode(1)}
    not_connected! {readout_modes()}
    not_connected! {percent_completed()}
    not_connected! {start_exposure(Duration::from_secs_f64(1.0), true)}
    not_connected! {max_adu()}
    //not_connected! {stop_exposure()}
    not_connected! {abort_exposure()}
    not_connected! {pixel_size_x()}
    not_connected! {pixel_size_y()}
    not_connected! {can_get_cooler_power()}
    not_connected! {ccd_temperature()}
    not_connected! {set_ccd_temperature()}
    not_connected! {set_set_ccd_temperature(0.0)}
    not_connected! {cooler_on()}
    not_connected! {set_cooler_on(true)}
    not_connected! {cooler_power()}
    not_connected! {exposure_min()}
    not_connected! {exposure_max()}
    not_connected! {exposure_resolution()}
    not_connected! {gain()}
    not_connected! {set_gain(1)}
    not_connected! {gain_min()}
    not_connected! {gain_max()}
    not_connected! {offset()}
    not_connected! {set_offset(10)}
    not_connected! {offset_min()}
    not_connected! {offset_max()}
    not_connected! {bayer_offset_x()}
    not_connected! {bayer_offset_y()}
    not_connected! {can_fast_readout()}
    not_connected! {fast_readout()}
    not_connected! {set_fast_readout(true)}
}

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
        binning: RwLock::new(1_u8),
        valid_bins: RwLock::new(None),
        target_temperature: RwLock::new(None),
        ccd_info: RwLock::new(None),
        intended_roi: RwLock::new(None),
        readout_speed_min_max_step: RwLock::new(None),
        exposure_min_max_step: RwLock::new(None),
        last_exposure_start_time: Arc::new(RwLock::new(None)),
        last_exposure_duration_us: Arc::new(RwLock::new(None)),
        last_image: Arc::new(RwLock::new(None)),
        state: Arc::new(RwLock::new(State::Idle)),
        gain_min_max: RwLock::new(None),
        offset_min_max: RwLock::new(None),
    };
    //then
    assert_eq!(camera.unique_id, "test_camera");
    assert_eq!(camera.name, "QHYCCD-test_camera");
    assert_eq!(camera.description, "QHYCCD camera");
    assert_eq!(*camera.binning.read().await, 1);
    assert!(camera.valid_bins.read().await.is_none());
    assert!(camera.intended_roi.read().await.is_none());
    assert!(camera.last_exposure_start_time.read().await.is_none());
    assert!(camera.last_exposure_duration_us.read().await.is_none());
    assert!(camera.last_image.read().await.is_none());
    assert_eq!(*camera.state.read().await, State::Idle);
    assert_eq!(camera.static_name(), "QHYCCD-test_camera");
    assert_eq!(camera.unique_id(), "test_camera");
    assert_eq!(camera.description().await.unwrap(), "QHYCCD camera");
    assert_eq!(
        camera.driver_info().await.unwrap(),
        "qhyccd-alpaca See: https://crates.io/crates/qhyccd-alpaca"
    );
    assert_eq!(
        camera.driver_version().await.unwrap(),
        env!("CARGO_PKG_VERSION")
    );
}

#[rstest]
#[case(State::Idle, Ok(CameraState::Idle))]
#[case(State::Exposing{ start: SystemTime::UNIX_EPOCH, expected_duration_us: 1_000_u32, stop_tx: None, done_rx: watch::channel(false).1, }, Ok(CameraState::Exposing))]
#[tokio::test]
async fn camera_state(#[case] state: State, #[case] expected: ASCOMResult<CameraState>) {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::WithState { times: 1, state });
    //when
    let res = camera.camera_state().await;
    //then
    if expected.is_ok() {
        assert_eq!(res.unwrap(), expected.unwrap())
    } else {
        assert_eq!(
            res.unwrap_err().to_string(),
            expected.unwrap_err().to_string()
        );
    }
}

#[tokio::test]
async fn connected_fail() {
    //given
    let mut mock = MockCamera::new();
    mock.expect_is_open()
        .times(1)
        .returning(|| Err(eyre!("Could not acquire read lock on camera handle")));
    let camera = new_camera(mock, MockCameraType::Untouched);
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
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_already_disconnected() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    assert!(res.is_ok());
}

#[rustfmt::skip]
#[rstest]
#[case(false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, false, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, false, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, false, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, false, true, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, false, true, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, false, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, false, false, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, false, true, false, false, Ok(()))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, false, false, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, false, false, Ok(()))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, false, Err(ASCOMError::NOT_CONNECTED))]
#[case(true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, Ok(()))]
#[tokio::test]
async fn set_connected_true(
    #[case] open: bool,
    #[case] has_single_frame_mode: bool,
    #[case] set_stream_mode: bool,
    #[case] set_readout_mode: bool,
    #[case] init: bool,
    #[case] transfer_bit: bool,
    #[case] ccd_info: bool,
    #[case] effective_area: bool,
    #[case] has_bin_modes: bool,
    #[case] has_speed_control: bool,
    #[case] speed_min_max: bool,
    #[case] exposure_min_max: bool,
    #[case] has_gain_control: bool,
    #[case] gain_min_max: bool,
    #[case] has_offset_control: bool,
    #[case] offset_min_max: bool,
    #[case] expected: ASCOMResult,
) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_open()
        .once()
        .returning(move || if open { Ok(()) } else { Err(eyre!("error")) });
    mock.expect_is_control_available()
        .times(if open { 1 } else { 0 })
        .withf(|control| *control == qhyccd_rs::Control::CamSingleFrameMode)
        .returning(move |_| {
            if has_single_frame_mode {
                Some(0_u32)
            } else {
                None
            }
        });
    mock.expect_set_stream_mode()
        .times(if has_single_frame_mode { 1 } else { 0 })
        .withf(|mode| *mode == qhyccd_rs::StreamMode::SingleFrameMode)
        .returning(move |_| {
            if set_stream_mode {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_set_readout_mode()
        .times(if set_stream_mode { 1 } else { 0 })
        .withf(|mode| *mode == 0)
        .returning(move |_| {
            if set_readout_mode {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_init()
        .times(if set_readout_mode { 1 } else { 0 })
        .returning(move || if init { Ok(()) } else { Err(eyre!("error")) });
    mock.expect_set_if_available()
        .times(if init { 1 } else { 0 })
        .withf(|control, bits| *control == qhyccd_rs::Control::TransferBit && *bits == 16_f64)
        .returning(move |_, _| {
            if transfer_bit {
                Ok(())
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_ccd_info()
        .times(if transfer_bit { 1 } else { 0 })
        .returning(move || {
            if ccd_info {
                Ok(CCDChipInfo {
                    chip_width: 7.0,
                    chip_height: 5.0,
                    image_width: 1920,
                    image_height: 1080,
                    pixel_width: 2.9,
                    pixel_height: 2.9,
                    bits_per_pixel: 16,
                })
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_effective_area()
        .times(if ccd_info { 1 } else { 0 })
        .returning(move || {
            if effective_area {
                Ok(CCDChipArea {
                    start_x: 0,
                    start_y: 0,
                    width: 100,
                    height: 100,
                })
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if effective_area { 6 } else { 0 })
        .withf(move |control| {
            control == &Control::CamBin1x1mode
                || control == &Control::CamBin2x2mode
                || control == &Control::CamBin3x3mode
                || control == &Control::CamBin4x4mode
                || control == &Control::CamBin6x6mode
                || control == &Control::CamBin8x8mode
        })
        .returning(move |control| {
            if has_bin_modes {
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
    mock.expect_is_control_available()
        .times(if effective_area { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| if has_speed_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_speed_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Speed)
        .returning(move |_| {
            if speed_min_max {
                Ok((0_f64, 255_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_get_parameter_min_max_step()
        .times(if speed_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Exposure)
        .returning(move |_| {
            if exposure_min_max {
                Ok((1_f64, 3_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if exposure_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| if has_gain_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_gain_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Gain)
        .returning(move |_| {
            if gain_min_max {
                Ok((0_f64, 51_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    mock.expect_is_control_available()
        .times(if gain_min_max { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| if has_offset_control { Some(0) } else { None });
    mock.expect_get_parameter_min_max_step()
        .times(if has_offset_control { 1 } else { 0 })
        .withf(move |control| *control == qhyccd_rs::Control::Offset)
        .returning(move |_| {
            if offset_min_max {
                Ok((0_f64, 1023_f64, 1_f64))
            } else {
                Err(eyre!("error"))
            }
        });
    let camera = new_camera(mock, MockCameraType::IsOpenFalse { times: 1 });
    //when
    let res = camera.set_connected(true).await;
    if expected.is_ok() {
        assert!(res.is_ok())
    } else {
        assert_eq!(
            expected.unwrap_err().to_string(),
            res.unwrap_err().to_string()
        )
    }
}

#[rstest]
#[case(Ok(()), Ok(()))]
#[case(Err(eyre!("error")), Err(ASCOMError::NOT_CONNECTED))]
#[tokio::test]
async fn set_connected_false_success(#[case] close: Result<()>, #[case] expected: ASCOMResult) {
    //given
    let mut mock = MockCamera::new();
    mock.expect_close().once().return_once(move || close);
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.set_connected(false).await;
    if expected.is_ok() {
        assert!(res.is_ok())
    } else {
        assert_eq!(
            expected.unwrap_err().to_string(),
            res.unwrap_err().to_string()
        )
    }
}
