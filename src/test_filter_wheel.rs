use super::*;
use crate::mocks::MockFilterWheel;

enum MockFilterWheelType {
    Untouched,
    IsOpenTrue { times: usize },
    WithTargetPosition { target: u32 },
}

fn new_filter_wheel(
    mut device: MockFilterWheel,
    variant: MockFilterWheelType,
) -> QhyccdFilterWheel {
    let number_of_filters = RwLock::new(None);
    let mut target_position = RwLock::new(None);
    match variant {
        MockFilterWheelType::Untouched => {}
        MockFilterWheelType::IsOpenTrue { times } => {
            device.expect_is_open().times(times).returning(|| Ok(true));
            device
                .expect_is_cfw_plugged_in()
                .once()
                .returning(|| Ok(true));
        }
        MockFilterWheelType::WithTargetPosition { target } => {
            device.expect_is_open().once().returning(|| Ok(true));
            device
                .expect_is_cfw_plugged_in()
                .once()
                .returning(|| Ok(true));
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
async fn get_position_fail_is_cwf_plugged_in() {
    //given
    let mut mock = MockFilterWheel::new();
    mock.expect_is_open().once().returning(|| Ok(true));
    mock.expect_is_cfw_plugged_in()
        .once()
        .returning(|| Ok(false));
    let filter_wheel = new_filter_wheel(mock, MockFilterWheelType::Untouched);
    let res = filter_wheel.position().await;
    assert_eq!(
        res.err().unwrap().to_string(),
        ASCOMError::NOT_CONNECTED.to_string()
    );
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
