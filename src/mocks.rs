use eyre::Result;
use qhyccd_rs::{CCDChipArea, CCDChipInfo, Control};

use mockall::*;

mock! {
    pub Sdk {
        pub fn new() -> Result<MockSdk>;
        pub fn cameras(&self) -> impl Iterator<Item = MockCamera>;
        pub fn filter_wheels(&self) -> impl Iterator<Item = MockFilterWheel>;
        pub fn version(&self) -> Result<qhyccd_rs::SDKVersion>;
    }
}

mock! {
    #[derive(Debug)]
    pub Camera {
        pub fn id(&self) -> &str;
        pub fn set_stream_mode(&self, mode: qhyccd_rs::StreamMode) -> Result<()>;
        pub fn set_readout_mode(&self, mode: u32) -> Result<()>;
        pub fn get_model(&self) -> Result<String>;
        pub fn init(&self) -> Result<()>;
        pub fn get_firmware_version(&self) -> Result<String>;
        pub fn get_number_of_readout_modes(&self) -> Result<u32>;
        pub fn get_readout_mode_name(&self, index: u32) -> Result<String>;
        pub fn get_readout_mode_resolution(&self, index: u32) -> Result<(u32, u32)>;
        pub fn get_readout_mode(&self) -> Result<u32>;
        pub fn get_type(&self) -> Result<u32>;
        pub fn set_bin_mode(&self, bin_x: u32, bin_y: u32) -> Result<()>;
        pub fn set_debayer(&self, on: bool) -> Result<()>;
        pub fn set_roi(&self, roi: CCDChipArea) -> Result<()>;
        pub fn begin_live(&self) -> Result<()>;
        pub fn end_live(&self) -> Result<()>;
        pub fn get_image_size(&self) -> Result<usize>;
        pub fn get_live_frame(&self, buffer_size: usize) -> Result<qhyccd_rs::ImageData>;
        pub fn get_single_frame(&self, buffer_size: usize) -> Result<qhyccd_rs::ImageData>;
        pub fn get_overscan_area(&self) -> Result<CCDChipArea>;
        pub fn get_effective_area(&self) -> Result<CCDChipArea>;
        pub fn start_single_frame_exposure(&self) -> Result<()>;
        pub fn get_remaining_exposure_us(&self) -> Result<u32>;
        pub fn stop_exposure(&self) -> Result<()>;
        pub fn abort_exposure_and_readout(&self) -> Result<()>;
        pub fn is_control_available(&self, control: Control) -> Option<u32>;
        pub fn get_ccd_info(&self) -> Result<CCDChipInfo>;
        pub fn set_bit_mode(&self, mode: u32) -> Result<()>;
        pub fn get_parameter(&self, control: Control) -> Result<f64>;
        pub fn get_parameter_min_max_step(&self, control: Control) -> Result<(f64,f64,f64)>;
        pub fn set_parameter(&self, control: Control, value: f64) -> Result<()>;
        pub fn set_if_available(&self, control: Control, value: f64) -> Result<()>;
        pub fn is_cfw_plugged_in(&self) -> Result<bool>;
        pub fn open(&self) -> Result<()>;
        pub fn close(&self) -> Result<()>;
        pub fn is_open(&self) -> Result<bool>;
        pub fn get_number_of_filters(&self) -> Option<u32>;
        pub fn get_fw_position(&self) -> Option<u32>;
        pub fn set_fw_position(&self, position: u32) -> Result<()>;
    }
    impl Clone for Camera {
        fn clone(&self) -> Self;
    }
}

mock! {
    #[derive(Debug)]
    pub FilterWheel {
        pub fn get_number_of_filters(&self) -> Result<u32>;
        pub fn get_fw_position(&self) -> Result<u32>;
        pub fn set_fw_position(&self, position: u32) -> Result<()>;
        pub fn open(&self) -> Result<()>;
        pub fn close(&self) -> Result<()>;
        pub fn is_open(&self) -> Result<bool>;
    }
}
