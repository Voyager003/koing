//! 자동 한글 입력 감지 모듈

mod auto_detect;
mod patterns;
pub mod validator;

pub use auto_detect::AutoDetector;
pub use validator::{has_excessive_jamo, has_incomplete_jamo, is_valid_hangul_result};
