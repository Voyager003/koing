//! N-gram 기반 한글 검증 시스템
//!
//! 한영 자동변환의 정확도를 높이기 위한 N-gram 기반 검증 시스템입니다.
//!
//! # 개요
//!
//! 이 모듈은 영문 입력이 실제로 한글로 변환되어야 하는지를 판단합니다.
//! 3단계 파이프라인으로 동작합니다:
//!
//! 1. **영문 → 한글 변환**: 기존 `core::converter` 사용
//! 2. **낱자모 검사**: 변환 결과에 불완전한 자모가 있으면 거부
//! 3. **N-gram 스코어**: 학습된 모델로 자연스러운 한글인지 확인
//!
//! # 사용 예시
//!
//! ```no_run
//! use koing::ngram::{KoreanValidator, NgramModel, NgramConfig};
//!
//! // 모델 없이 낱자모 검사만 사용
//! let validator = KoreanValidator::new();
//! assert!(validator.should_convert_to_korean("dkssud"));  // "안녕" -> true
//! assert!(!validator.should_convert_to_korean("name"));   // "ㅜ믇" -> false
//!
//! // 모델과 함께 사용
//! let model = NgramModel::load("data/ngram_model.json").unwrap();
//! let config = NgramConfig::new().with_threshold(-10.0);
//! let validator = KoreanValidator::with_model(model, config);
//! assert!(validator.should_convert_to_korean("gksrmf"));  // "한글"
//! ```
//!
//! # 역변환
//!
//! 한글을 영문으로 역변환하는 기능도 제공합니다:
//!
//! ```
//! use koing::ngram::korean_to_eng;
//! assert_eq!(korean_to_eng("안녕"), "dkssud");
//! assert_eq!(korean_to_eng("한글"), "gksrmf");
//! ```

mod config;
mod keymap;
mod model;
mod validator;

// 공개 인터페이스
pub use config::NgramConfig;
pub use keymap::korean_to_eng;
pub use model::{NgramError, NgramModel};
pub use validator::{KoreanValidator, ValidationResult};
