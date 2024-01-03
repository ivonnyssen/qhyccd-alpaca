use super::*;
use crate::mocks::MockFilterWheel;

enum MockFilterWheelType {
    IsOpenTrue {
        times: usize,
    },
    IsOpenFalse {
        times: usize,
    },
    WithTargetPosition {
        target: u32,
    },
    WithFiltersAndTargetPosition {
        times: usize,
        filters: u32,
        target: u32,
    },
}

fn new_filter_wheel(
    mut device: MockFilterWheel,
    variant: MockFilterWheelType,
) -> QhyccdFilterWheel {
    let mut number_of_filters = RwLock::new(None);
    let mut target_position = RwLock::new(None);
    match variant {
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
async fn set_connected_success_not_connected() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_open().once().returning(|| Ok(()));
    mock.expect_get_number_of_filters()
        .once()
        .returning(|| Ok(7));
    mock.expect_get_fw_position().once().returning(|| Ok(0));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenFalse { times: 1 });
    let res = filter_wheel.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn set_connected_success_connected() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    let res = filter_wheel.set_connected(true).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn get_position_success_not_moving() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel =
        new_filter_wheel(mock, MockFilterWheelType::WithTargetPosition { target: 5 });
    let position = filter_wheel.position().await;
    assert_eq!(position.unwrap(), 5);
}

#[tokio::test]
async fn get_position_success_moving() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_get_fw_position().returning(|| Ok(5));
    let filter_wheel =
        new_filter_wheel(mock, MockFilterWheelType::WithTargetPosition { target: 1 });
    let position = filter_wheel.position().await;
    assert_eq!(position.unwrap(), -1);
}

#[tokio::test]
async fn get_position_fail_target_position_none() {
    //given
    let mock = MockFilterWheel::new();
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::IsOpenTrue { times: 1 });
    let res = filter_wheel.position().await;
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
    let res = filter_wheel.position().await;
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::UNSPECIFIED.to_string()
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
    let res = filter_wheel.set_position(5).await;
    assert!(res.is_ok());
    assert_eq!(filter_wheel.position().await.unwrap(), 5);
}
