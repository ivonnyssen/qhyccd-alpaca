//! Image data handling tests

use super::*;

#[tokio::test]
async fn image_array_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithImage {
            image_array: Array3::<u16>::zeros((10_usize, 10_usize, 3)).into(),
        },
    );
    //when
    let res = camera.image_array().await;
    //then
    assert!(res.is_ok());
    assert_eq!(res.unwrap().shape(), [10, 10, 3]);
}

#[tokio::test]
async fn image_array_empty() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(mock, MockCameraType::IsOpenTrue { times: 1 });
    //when
    let res = camera.image_array().await;
    //then
    assert!(res.is_err());
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::VALUE_NOT_SET.to_string()
    );
}

#[tokio::test]
async fn image_ready_not_ready_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithStateExposing {
            expected_duration: 1000_f64,
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}

#[tokio::test]
async fn image_ready_ready_success() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithImage {
            image_array: Array3::<u16>::zeros((10_usize, 10_usize, 3)).into(),
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(res.unwrap());
}

#[tokio::test]
async fn image_ready_ready_success_no_image_taken_yet() {
    //given
    let mock = MockCamera::new();
    let camera = new_camera(
        mock,
        MockCameraType::WithState {
            times: 1,
            state: State::Idle,
        },
    );
    //when
    let res = camera.image_ready().await;
    //then
    assert!(res.is_ok());
    assert!(!res.unwrap());
}
