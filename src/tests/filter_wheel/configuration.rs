//! Filter wheel configuration tests

use super::*;

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
