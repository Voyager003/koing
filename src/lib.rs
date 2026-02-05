pub mod core;
pub mod detection;
pub mod platform;
pub mod ui;

pub use core::converter::convert;
pub use detection::{has_excessive_jamo, has_incomplete_jamo, is_valid_hangul_result, AutoDetector};
