use super::*;
use crate::mocks::MockFilterWheel;

enum MockFilterWheelType {
    Untouched,
    IsOpenTrue {
        times: usize,
    },
    IsOpenFalse {
        times: usize,
    },
    WithTargetPosition {
        target: u32,
    },
    WithFilters {
        times: usize,
        filters: u32,
    },
    WithFiltersAndTargetPosition {
        times: usize,
        filters: u32,
        target: u32,
    },
}

macro_rules! not_connected {
    ($name:ident$tail:tt) => {
        let mock = MockFilterWheel::new();
        let fw = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
        let res = fw.$name$tail.await;
        assert_eq!(
            res.err().unwrap().to_string(),
            ASCOMError::NOT_CONNECTED.to_string(),
        );
    };
}

#[tokio::test]
async fn not_connected_asyncs() {
    not_connected! {focus_offsets()}
    not_connected! {names()}
    not_connected! {position()}
    not_connected! {set_position(0)}
}

fn new_filter_wheel(
    mut device: MockFilterWheel,
    variant: MockFilterWheelType,
) -> QhyccdFilterWheel {
    let mut number_of_filters = RwLock::new(None);
    let mut target_position = RwLock::new(None);
    match variant {
        MockFilterWheelType::Untouched => {}
        MockFilterWheelType::IsOpenTrue { times } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
        }
        MockFilterWheelType::IsOpenFalse { times } => {
            device.expect_is_open().times(times).returning(|| Ok(false));
        }
        MockFilterWheelType::WithTargetPosition { target } => {
            device.expect_is_open().once().returning(|| Ok(true));
            target_position = RwLock::new(Some(target));
        }
        MockFilterWheelType::WithFilters { times, filters } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            number_of_filters = RwLock::new(Some(filters));
        }
        MockFilterWheelType::WithFiltersAndTargetPosition {
            times,
            filters,
            target,
        } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            number_of_filters = RwLock::new(Some(filters));
            target_position = RwLock::new(Some(target));
        }
    }
    QhyccdFilterWheel {
        unique_id: "test-filter_wheel".to_owned(),
        name: "QHYCCD-test_filter_wheel".to_owned(),
        description: "QHYCCD filter wheel".to_owned(),
        device,
        number_of_filters,
        target_position,
    }
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

#[tokio::test]
async fn focus_offset_success() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFilters {
            times: 1,
            filters: 5,
        },
    );
    //when
    let res = filter_wheel.focus_offsets().await;
    //then
    assert_eq!(res.unwrap(), vec![0_i32; 5]);
}

#[tokio::test]
async fn focus_offset_fail() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.focus_offsets().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn names_success() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFilters {
            times: 1,
            filters: 2,
        },
    );
    //when
    let res = filter_wheel.names().await;
    //then
    assert_eq!(res.unwrap(), vec!["Filter0", "Filter1"]);
}

#[tokio::test]
async fn names_fail() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.names().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn get_position_success_not_moving() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel =
        new_filter_wheel(mock, MockFilterWheelType::WithTargetPosition { target: 5 });
    //when
    let position = filter_wheel.position().await;
    //then
    assert_eq!(position.unwrap(), Some(5));
}

#[tokio::test]
async fn get_position_success_moving() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel =
        new_filter_wheel(mock, MockFilterWheelType::WithTargetPosition { target: 1 });
    //when
    let position = filter_wheel.position().await;
    //then
    assert_eq!(position.unwrap(), None);
}

#[tokio::test]
async fn get_position_fail_target_position_none() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.position().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn get_position_fail_get_fw_position() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position()
        .returning(|| Err(eyre!(qhyccd_rs::QHYError::GetCfwPositionError)));
    let filter_wheel =
        new_filter_wheel(mock, MockFilterWheelType::WithTargetPosition { target: 5 });
    //when
    let res = filter_wheel.position().await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string()
    );
}

#[tokio::test]
async fn set_position_success() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_set_fw_position()
        .once()
        .withf(|position| *position == 5)
        .returning(|_| Ok(()));
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFiltersAndTargetPosition {
            times: 2,
            filters: 6,
            target: 0,
        },
    );
    //when
    let res = filter_wheel.set_position(5).await;
    //then
    assert!(res.is_ok());
    assert_eq!(filter_wheel.position().await.unwrap(), Some(5));
}

#[tokio::test]
async fn set_position_success_number_of_target_position_none() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_set_fw_position()
        .once()
        .withf(|position| *position == 2)
        .returning(|_| Ok(()));
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFilters {
            times: 1,
            filters: 5,
        },
    );
    //when
    let res = filter_wheel.set_position(2).await;
    //then
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_position_success_already_at_target() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFiltersAndTargetPosition {
            times: 2,
            filters: 6,
            target: 5,
        },
    );
    //when
    let res = filter_wheel.set_position(5).await;
    //then
    assert!(res.is_ok());
    assert_eq!(filter_wheel.position().await.unwrap(), Some(5));
}

#[tokio::test]
async fn set_position_fail_invalid_value() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFilters {
            times: 1,
            filters: 5,
        },
    );
    //when
    let res = filter_wheel.set_position(5).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_VALUE.to_string()
    );
}

#[tokio::test]
async fn set_position_fail_number_of_filters_none() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    //when
    let res = filter_wheel.set_position(5).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
}

#[tokio::test]
async fn set_position_fail_set_fw_position() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_set_fw_position()
        .once()
        .withf(|position| *position == 5)
        .returning(|_| Err(eyre!(qhyccd_rs::QHYError::SetCfwPositionError)));
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel = new_filter_wheel(
        mock,
        MockFilterWheelType::WithFiltersAndTargetPosition {
            times: 1,
            filters: 6,
            target: 0,
        },
    );
    //when
    let res = filter_wheel.set_position(5).await;
    //then
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::INVALID_OPERATION.to_string()
    );
}
