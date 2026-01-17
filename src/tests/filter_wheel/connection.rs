//! Filter wheel connection tests

use super::*;

#[tokio::test]
async fn not_connected_asyncs() {
    not_connected_fw! {focus_offsets()}
    not_connected_fw! {names()}
    not_connected_fw! {position()}
    not_connected_fw! {set_position(0)}
}

#[tokio::test]
async fn new_success() {
    //given
    let mock = MockFilterWheel::new();
    //when
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::Untouched);
    //then
    assert_eq!(filter_wheel.unique_id(), "test-filter_wheel");
    assert_eq!(filter_wheel.static_name(), "QHYCCD-test_filter_wheel");
    assert_eq!(
        filter_wheel.description().await.unwrap(),
        "QHYCCD filter wheel"
    );
    assert_eq!(
        filter_wheel.driver_info().await.unwrap(),
        "qhyccd-alpaca See: https://crates.io/crates/qhyccd-alpaca"
    );
    assert_eq!(
        filter_wheel.driver_version().await.unwrap(),
        env!("CARGO_PKG_VERSION")
    );
}

#[tokio::test]
async fn connected_success() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.connected().await;
    //then
    assert!(res.unwrap());
}

#[tokio::test]
async fn connected_fail() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_is_open()
        .once()
        .returning(|| Err(eyre!("error")));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::Untouched);
    //when
    let res = filter_wheel.connected().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_success_not_connected() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_get_number_of_filters()
        .once()
        .returning(|| Ok(7));
    mock.expect_get_fw_position().once().returning(|| Ok(0));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
    //when
    let res = filter_wheel.set_connected(true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_success_already_connected() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.set_connected(true).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_fail_open() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_open().once().returning(|| Err(eyre!("error")));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
    //when
    let res = filter_wheel.set_connected(true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_number_of_filters() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_get_number_of_filters()
        .once()
        .returning(|| Err(eyre!("error")));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
    //when
    let res = filter_wheel.set_connected(true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_fail_get_fw_position() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_get_number_of_filters()
        .once()
        .returning(|| Ok(7));
    mock.expect_get_fw_position()
        .once()
        .returning(|| Err(eyre!("error")));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
    //when
    let res = filter_wheel.set_connected(true).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_connected_false_success() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_close().once().returning(|| Ok(()));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.set_connected(false).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_false_fail() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_close().once().returning(|| Err(eyre!("error")));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.set_connected(false).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}
