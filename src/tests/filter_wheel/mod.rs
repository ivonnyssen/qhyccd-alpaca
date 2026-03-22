//! Filter wheel test utilities and modules

use crate::mocks::MockFilterWheel;
use crate::*;
use eyre::eyre;

// Test modules
pub mod configuration;
pub mod connection;
pub mod position;

/// Mock filter wheel configuration variants for test setup
pub enum MockFilterWheelType {
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

/// Macro for testing NOT_CONNECTED error responses
#[macro_export]
macro_rules! not_connected_fw {
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

/// Creates a new QhyccdFilterWheel with the specified mock configuration
pub fn new_filter_wheel(
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
