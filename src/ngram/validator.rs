//! N-gram 기반 종합 검증 모듈
//!
//! 3단계 파이프라인으로 한글 변환 여부를 판정합니다:
//! 1. 낱자모 검사 (기존 detection::validator 활용)
//! 2. N-gram 스코어 검사
//! 3. 최종 판정

use crate::core::converter::convert;
use crate::detection::validator::has_incomplete_jamo;
use std::path::PathBuf;

use super::config::NgramConfig;
use super::model::{NgramAnalysis, NgramModel};
use super::syllable_validator::check_syllable_structure;

/// N-gram 기반 한글 검증기
///
/// 영문 입력이 한글로 변환되어야 하는지 종합적으로 판정합니다.
#[derive(Debug)]
pub struct KoreanValidator {
    /// N-gram 모델 (없으면 스코어 검사 생략)
    model: Option<NgramModel>,
    /// 설정
    config: NgramConfig,
}

impl Default for KoreanValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl KoreanValidator {
    /// 기본 설정으로 검증기 생성 (모델 없음)
    pub fn new() -> Self {
        Self {
            model: None,
            config: NgramConfig::default(),
        }
    }

    /// 모델과 설정을 지정하여 검증기 생성
    pub fn with_model(model: NgramModel, config: NgramConfig) -> Self {
        Self {
            model: Some(model),
            config,
        }
    }

    /// 설정만 지정하여 검증기 생성
    pub fn with_config(config: NgramConfig) -> Self {
        Self {
            model: None,
            config,
        }
    }

    /// 모델 파일에서 로드하여 검증기 생성
    pub fn load(path: &str) -> Result<Self, super::model::NgramError> {
        let model = NgramModel::load(path)?;
        Ok(Self {
            model: Some(model),
            config: NgramConfig::new().with_model_path(path),
        })
    }

    /// 일반 실행/앱 번들 환경에서 기본 모델 경로를 찾아 로드
    pub fn load_default() -> Result<Self, super::model::NgramError> {
        for candidate in default_model_candidates() {
            if candidate.is_file() {
                return Self::load(&candidate.to_string_lossy());
            }
        }

        Err(super::model::NgramError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "기본 N-gram 모델 경로를 찾을 수 없습니다",
        )))
    }

    /// 영문 입력을 한글로 변환해야 하는지 판정
    ///
    /// 3단계 파이프라인:
    /// 1. 영문 → 한글 변환
    /// 2. 낱자모 검사: 낱자모가 포함되면 false (잘못된 변환)
    /// 3. N-gram 스코어 검사: 임계값 이상이면 true
    ///
    /// # Examples
    /// ```
    /// use koing::ngram::KoreanValidator;
    ///
    /// let validator = KoreanValidator::new();
    ///
    /// // 낱자모가 있는 경우 → false
    /// assert!(!validator.should_convert_to_korean("name")); // "ㅜ믇"
    ///
    /// // 정상 한글 → true (모델 없으면 낱자모 검사만)
    /// assert!(validator.should_convert_to_korean("dkssud")); // "안녕"
    /// ```
    pub fn should_convert_to_korean(&self, english_input: &str) -> bool {
        self.analyze(english_input).should_convert
    }

    /// 변환된 한글의 N-gram 스코어 반환
    ///
    /// 모델이 없으면 None
    pub fn score(&self, korean_text: &str) -> Option<f64> {
        self.model
            .as_ref()
            .map(|m| m.score_with_config(korean_text, &self.config))
    }

    /// 영문 입력을 한글로 변환하고 스코어 반환
    ///
    /// # Returns
    /// (변환 결과, 낱자모 포함 여부, N-gram 스코어)
    pub fn analyze(&self, english_input: &str) -> ValidationResult {
        if english_input.is_empty() {
            return ValidationResult::rejected(
                english_input,
                String::new(),
                RejectReason::EmptyInput,
                None,
            );
        }

        let converted = convert(english_input);
        if converted == english_input {
            return ValidationResult::rejected(
                english_input,
                converted,
                RejectReason::Unchanged,
                None,
            );
        }

        let has_jamo = has_incomplete_jamo(&converted);
        if has_jamo {
            return ValidationResult::rejected(
                english_input,
                converted,
                RejectReason::IncompleteJamo,
                None,
            );
        }

        let syllable_valid = check_syllable_structure(&converted);
        if !syllable_valid {
            return ValidationResult::rejected(
                english_input,
                converted,
                RejectReason::UnnaturalSyllables,
                None,
            );
        }

        let analysis = self
            .model
            .as_ref()
            .map(|model| model.analyze_with_config(&converted, &self.config));
        let score = analysis.as_ref().map(|result| result.score);
        let should_convert = score.map(|s| s >= self.config.threshold).unwrap_or(true);
        let reject_reason = if should_convert {
            None
        } else {
            Some(RejectReason::LowScore)
        };

        ValidationResult {
            original: english_input.to_string(),
            converted,
            has_incomplete_jamo: has_jamo,
            has_unnatural_syllables: !syllable_valid,
            ngram_score: score,
            should_convert,
            unknown_unigram_ratio: analysis.as_ref().map(|result| result.unknown_unigram_ratio),
            unknown_bigram_ratio: analysis.as_ref().map(|result| result.unknown_bigram_ratio),
            seen_bigram_count: analysis.as_ref().map(|result| result.seen_bigram_count),
            reject_reason,
        }
    }

    /// 현재 설정의 임계값 반환
    pub fn threshold(&self) -> f64 {
        self.config.threshold
    }

    /// 모델이 로드되어 있는지 확인
    pub fn has_model(&self) -> bool {
        self.model.is_some()
    }
}

/// 검증 결과
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 원본 영문 입력
    pub original: String,
    /// 변환된 한글
    pub converted: String,
    /// 낱자모 포함 여부
    pub has_incomplete_jamo: bool,
    /// 비자연스러운 음절 구조 포함 여부
    pub has_unnatural_syllables: bool,
    /// N-gram 스코어 (모델이 없으면 None)
    pub ngram_score: Option<f64>,
    /// 최종 판정: 한글로 변환해야 하는지
    pub should_convert: bool,
    /// 미등록 유니그램 비율 (모델이 없으면 None)
    pub unknown_unigram_ratio: Option<f64>,
    /// 미등록 바이그램 비율 (모델이 없으면 None)
    pub unknown_bigram_ratio: Option<f64>,
    /// 등록된 바이그램 개수 (모델이 없으면 None)
    pub seen_bigram_count: Option<usize>,
    /// 자동 변환 거부 이유
    pub reject_reason: Option<RejectReason>,
}

impl ValidationResult {
    fn rejected(
        english_input: &str,
        converted: String,
        reject_reason: RejectReason,
        analysis: Option<&NgramAnalysis>,
    ) -> Self {
        Self {
            original: english_input.to_string(),
            converted,
            has_incomplete_jamo: matches!(reject_reason, RejectReason::IncompleteJamo),
            has_unnatural_syllables: matches!(reject_reason, RejectReason::UnnaturalSyllables),
            ngram_score: analysis.map(|result| result.score),
            should_convert: false,
            unknown_unigram_ratio: analysis.map(|result| result.unknown_unigram_ratio),
            unknown_bigram_ratio: analysis.map(|result| result.unknown_bigram_ratio),
            seen_bigram_count: analysis.map(|result| result.seen_bigram_count),
            reject_reason: Some(reject_reason),
        }
    }
}

/// 자동 변환 거부 이유
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    EmptyInput,
    Unchanged,
    IncompleteJamo,
    UnnaturalSyllables,
    LowScore,
}

fn default_model_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("data").join("ngram_model.json"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("../Resources/data/ngram_model.json"));
            candidates.push(exe_dir.join("../../data/ngram_model.json"));
            candidates.push(exe_dir.join("../data/ngram_model.json"));
        }
    }

    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique
            .iter()
            .any(|existing: &PathBuf| existing == &candidate)
        {
            unique.push(candidate);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_without_model() {
        let validator = KoreanValidator::new();

        // 정상 한글 변환 -> true
        assert!(validator.should_convert_to_korean("dkssud")); // 안녕
        assert!(validator.should_convert_to_korean("gksrmf")); // 한글
        assert!(validator.should_convert_to_korean("rkskek")); // 가나다

        // 낱자모 포함 -> false
        assert!(!validator.should_convert_to_korean("name")); // ㅜ믇
        assert!(!validator.should_convert_to_korean("hello")); // ㅗ디ㅣㅐ
        assert!(!validator.should_convert_to_korean("code")); // ㅊㅐㅇㄷ

        // 빈 입력 -> false
        assert!(!validator.should_convert_to_korean(""));

        // 변환 불가 문자 (숫자만) -> false
        assert!(!validator.should_convert_to_korean("12345"));
    }

    #[test]
    fn test_validator_with_model() {
        let json = r#"{
            "metadata": { "corpus_size": 1000 },
            "unigrams": { "안": 100, "녕": 80, "한": 90, "글": 70, "가": 100, "나": 80, "다": 70 },
            "bigrams": { "안|녕": 50, "한|글": 40, "가|나": 30, "나|다": 25 }
        }"#;

        let model = NgramModel::from_json(json).unwrap();
        let config = NgramConfig::new().with_threshold(-15.0);
        let validator = KoreanValidator::with_model(model, config);

        // 학습된 패턴 -> true
        assert!(validator.should_convert_to_korean("dkssud")); // 안녕
        assert!(validator.should_convert_to_korean("gksrmf")); // 한글

        // 낱자모 -> false
        assert!(!validator.should_convert_to_korean("name"));
    }

    #[test]
    fn test_analyze() {
        let validator = KoreanValidator::new();

        let result = validator.analyze("dkssud");
        assert_eq!(result.converted, "안녕");
        assert!(!result.has_incomplete_jamo);
        assert!(result.should_convert);

        let result = validator.analyze("name");
        assert!(result.has_incomplete_jamo);
        assert!(!result.should_convert);
    }

    #[test]
    fn test_score() {
        let json = r#"{
            "metadata": {},
            "unigrams": { "안": 100, "녕": 80 },
            "bigrams": { "안|녕": 50 }
        }"#;

        let model = NgramModel::from_json(json).unwrap();
        let validator = KoreanValidator::with_model(model, NgramConfig::default());

        // 학습된 텍스트는 높은 스코어
        let score = validator.score("안녕").unwrap();
        assert!(score > f64::NEG_INFINITY);

        // 모델 없는 검증기
        let no_model = KoreanValidator::new();
        assert!(no_model.score("안녕").is_none());
    }

    #[test]
    fn test_english_preservation() {
        let validator = KoreanValidator::new();

        // 낱자모가 포함되어 변환되지 않아야 하는 영어 단어들
        // (두벌식 변환 결과에 낱자모가 포함됨)
        let english_words_with_jamo = [
            "name",  // ㅜ믇
            "hello", // ㅗ디ㅣㅐ
            "code",  // ㅊㅐㅇㄷ
            "test",  // ㅅㄷㅅㅅ
        ];

        for word in &english_words_with_jamo {
            assert!(
                !validator.should_convert_to_korean(word),
                "영어 단어 '{}'가 변환되어서는 안됨 (낱자모 포함)",
                word
            );
        }
    }

    #[test]
    fn test_korean_patterns() {
        let validator = KoreanValidator::new();

        // 검증된 한글 입력 패턴들
        let korean_patterns = [
            ("dkssudgktpdy", "안녕하세요"),
            ("rkskek", "가나다"),
            ("gksrmf", "한글"),
            ("dkssud", "안녕"),
        ];

        for (input, expected) in &korean_patterns {
            let result = validator.analyze(input);
            assert_eq!(
                result.converted, *expected,
                "패턴 '{}'가 '{}'로 변환되어야 함",
                input, expected
            );
            assert!(
                result.should_convert,
                "패턴 '{}' ('{}'이 됨)가 한글로 변환되어야 함",
                input, expected
            );
        }
    }

    #[test]
    fn test_english_words_unnatural_syllables() {
        let validator = KoreanValidator::new();

        // "daisy" → "ㅇ먀뇨" — jamo(ㅇ) + 희귀 음절(먀)
        assert!(!validator.should_convert_to_korean("daisy"));

        // "virus" → "퍄견" — 희귀 음절(퍄)
        assert!(!validator.should_convert_to_korean("virus"));
    }

    #[test]
    fn test_analyze_unnatural_syllables() {
        let validator = KoreanValidator::new();

        let result = validator.analyze("daisy");
        assert!(result.has_unnatural_syllables || result.has_incomplete_jamo);
        assert!(!result.should_convert);

        let result = validator.analyze("virus");
        assert!(result.has_unnatural_syllables);
        assert!(!result.should_convert);
    }

    #[test]
    fn test_threshold_effect() {
        let json = r#"{
            "metadata": {},
            "unigrams": { "안": 10 },
            "bigrams": {}
        }"#;

        let model = NgramModel::from_json(json).unwrap();

        // 낮은 임계값 -> 변환 허용
        let low_threshold = NgramConfig::new().with_threshold(-20.0);
        let validator = KoreanValidator::with_model(model.clone(), low_threshold);
        assert!(validator.should_convert_to_korean("dkssud"));

        // 높은 임계값 -> 변환 거부
        let high_threshold = NgramConfig::new().with_threshold(0.0);
        let validator = KoreanValidator::with_model(model, high_threshold);
        assert!(!validator.should_convert_to_korean("dkssud"));
    }

    #[test]
    fn test_load_default_model() {
        let validator = KoreanValidator::load_default().unwrap();
        assert!(validator.has_model());
    }

    #[test]
    fn test_analyze_tracks_unknown_ngram_metrics() {
        let validator = KoreanValidator::load_default().unwrap();
        let result = validator.analyze("slack");

        assert_eq!(result.converted, "님차");
        assert!(result.unknown_bigram_ratio.unwrap() >= 1.0);
        assert_eq!(result.seen_bigram_count, Some(0));
    }
}
