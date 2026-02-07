pub mod config;
pub mod core;
pub mod detection;
pub mod ngram;
pub mod platform;
pub mod ui;

pub use core::converter::convert;
pub use detection::{has_excessive_jamo, has_incomplete_jamo, is_valid_hangul_result, AutoDetector};
pub use ngram::{korean_to_eng, KoreanValidator, NgramConfig, NgramModel};
