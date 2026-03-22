//! Filter wheel position management tests

use super::*;

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
