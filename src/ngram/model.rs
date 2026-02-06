//! N-gram 모델 로드 및 스코어링
//!
//! JSON 형식의 N-gram 모델 파일을 로드하고
//! 바이그램 로그 확률을 계산합니다.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use super::config::NgramConfig;

/// N-gram 모델 로드/파싱 에러
#[derive(Debug)]
pub enum NgramError {
    /// 파일 읽기 실패
    IoError(std::io::Error),
    /// JSON 파싱 실패
    ParseError(String),
    /// 모델 형식 오류
    FormatError(String),
}

impl std::fmt::Display for NgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NgramError::IoError(e) => write!(f, "파일 읽기 오류: {}", e),
            NgramError::ParseError(s) => write!(f, "JSON 파싱 오류: {}", s),
            NgramError::FormatError(s) => write!(f, "모델 형식 오류: {}", s),
        }
    }
}

impl std::error::Error for NgramError {}

impl From<std::io::Error> for NgramError {
    fn from(e: std::io::Error) -> Self {
        NgramError::IoError(e)
    }
}

/// N-gram 모델
///
/// 유니그램과 바이그램 빈도 데이터를 저장하고
/// 텍스트의 로그 확률 스코어를 계산합니다.
#[derive(Debug, Clone)]
pub struct NgramModel {
    /// 유니그램 빈도: 문자 -> 빈도
    unigrams: HashMap<char, u64>,
    /// 바이그램 빈도: (첫 번째 문자, 두 번째 문자) -> 빈도
    bigrams: HashMap<(char, char), u64>,
    /// 유니그램 총 빈도
    total_unigrams: u64,
}

impl NgramModel {
    /// JSON 파일에서 N-gram 모델 로드
    ///
    /// # 파일 형식
    /// ```json
    /// {
    ///   "unigrams": { "가": 12345, "나": 6789 },
    ///   "bigrams": { "가|나": 4567, "나|다": 2345 }
    /// }
    /// ```
    pub fn load(path: &str) -> Result<Self, NgramError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // serde_json으로 파싱
        let value: serde_json::Value = serde_json::from_reader(reader)
            .map_err(|e| NgramError::ParseError(e.to_string()))?;

        Self::from_json_value(&value)
    }

    /// JSON 문자열에서 모델 로드
    pub fn from_json(json_str: &str) -> Result<Self, NgramError> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| NgramError::ParseError(e.to_string()))?;

        Self::from_json_value(&value)
    }

    /// serde_json::Value에서 모델 생성
    fn from_json_value(value: &serde_json::Value) -> Result<Self, NgramError> {
        // 유니그램 파싱
        let unigrams_obj = value
            .get("unigrams")
            .and_then(|v| v.as_object())
            .ok_or_else(|| NgramError::FormatError("unigrams 필드가 없습니다".into()))?;

        let mut unigrams = HashMap::new();
        let mut total_unigrams = 0u64;

        for (key, val) in unigrams_obj {
            let char = key.chars().next().ok_or_else(|| {
                NgramError::FormatError(format!("빈 유니그램 키: {}", key))
            })?;
            let count = val.as_u64().ok_or_else(|| {
                NgramError::FormatError(format!("유효하지 않은 빈도값: {}", key))
            })?;
            unigrams.insert(char, count);
            total_unigrams += count;
        }

        // 바이그램 파싱
        let bigrams_obj = value
            .get("bigrams")
            .and_then(|v| v.as_object())
            .ok_or_else(|| NgramError::FormatError("bigrams 필드가 없습니다".into()))?;

        let mut bigrams = HashMap::new();

        for (key, val) in bigrams_obj {
            // "가|나" 형식 파싱
            let parts: Vec<&str> = key.split('|').collect();
            if parts.len() != 2 {
                return Err(NgramError::FormatError(format!(
                    "잘못된 바이그램 형식: {} (expected 'X|Y')",
                    key
                )));
            }

            let first = parts[0].chars().next().ok_or_else(|| {
                NgramError::FormatError(format!("빈 바이그램 첫 번째 문자: {}", key))
            })?;
            let second = parts[1].chars().next().ok_or_else(|| {
                NgramError::FormatError(format!("빈 바이그램 두 번째 문자: {}", key))
            })?;

            let count = val.as_u64().ok_or_else(|| {
                NgramError::FormatError(format!("유효하지 않은 빈도값: {}", key))
            })?;

            bigrams.insert((first, second), count);
        }

        Ok(Self {
            unigrams,
            bigrams,
            total_unigrams,
        })
    }

    /// 빈 모델 생성 (테스트용)
    pub fn empty() -> Self {
        Self {
            unigrams: HashMap::new(),
            bigrams: HashMap::new(),
            total_unigrams: 0,
        }
    }

    /// 유니그램 빈도 조회
    pub fn unigram_count(&self, c: char) -> u64 {
        self.unigrams.get(&c).copied().unwrap_or(0)
    }

    /// 바이그램 빈도 조회
    pub fn bigram_count(&self, first: char, second: char) -> u64 {
        self.bigrams.get(&(first, second)).copied().unwrap_or(0)
    }

    /// 총 유니그램 빈도
    pub fn total_unigrams(&self) -> u64 {
        self.total_unigrams
    }

    /// 텍스트의 바이그램 로그 확률 평균 계산
    ///
    /// Add-k 스무딩을 적용한 로그 확률:
    /// log P(w_i | w_{i-1}) = log((C(w_{i-1}, w_i) + k) / (C(w_{i-1}) + k*V))
    ///
    /// # Returns
    /// 바이그램 로그 확률의 평균 (텍스트가 비어있으면 f64::NEG_INFINITY)
    pub fn score(&self, text: &str) -> f64 {
        self.score_with_config(text, &NgramConfig::default())
    }

    /// 설정을 적용한 스코어 계산
    pub fn score_with_config(&self, text: &str, config: &NgramConfig) -> f64 {
        let chars: Vec<char> = text.chars().collect();

        if chars.len() < 2 {
            // 1글자 이하면 유니그램 확률만 사용
            if chars.is_empty() {
                return f64::NEG_INFINITY;
            }
            return self.unigram_log_prob(chars[0], config);
        }

        let mut log_prob_sum = 0.0;
        let mut count = 0;

        for window in chars.windows(2) {
            let first = window[0];
            let second = window[1];

            let bigram_count = self.bigram_count(first, second) as f64;
            let context_count = self.unigram_count(first) as f64;

            // Add-k 스무딩
            let k = config.smoothing_k;
            let v = config.vocab_size as f64;

            let prob = (bigram_count + k) / (context_count + k * v);
            log_prob_sum += prob.ln();
            count += 1;
        }

        if count == 0 {
            f64::NEG_INFINITY
        } else {
            log_prob_sum / count as f64
        }
    }

    /// 유니그램 로그 확률
    fn unigram_log_prob(&self, c: char, config: &NgramConfig) -> f64 {
        let count = self.unigram_count(c) as f64;
        let total = self.total_unigrams as f64;

        if total == 0.0 {
            return f64::NEG_INFINITY;
        }

        let k = config.smoothing_k;
        let v = config.vocab_size as f64;

        let prob = (count + k) / (total + k * v);
        prob.ln()
    }

    /// 모델에 데이터가 있는지 확인
    pub fn is_empty(&self) -> bool {
        self.unigrams.is_empty() && self.bigrams.is_empty()
    }

    /// 유니그램 수
    pub fn unigram_count_total(&self) -> usize {
        self.unigrams.len()
    }

    /// 바이그램 수
    pub fn bigram_count_total(&self) -> usize {
        self.bigrams.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_model_json() -> &'static str {
        r#"{
            "metadata": { "corpus_size": 1000 },
            "unigrams": { "안": 100, "녕": 80, "하": 90, "세": 70, "요": 60 },
            "bigrams": { "안|녕": 50, "녕|하": 30, "하|세": 40, "세|요": 35 }
        }"#
    }

    #[test]
    fn test_load_from_json() {
        let model = NgramModel::from_json(sample_model_json()).unwrap();

        assert_eq!(model.unigram_count('안'), 100);
        assert_eq!(model.unigram_count('녕'), 80);
        assert_eq!(model.unigram_count('없'), 0); // 없는 문자

        assert_eq!(model.bigram_count('안', '녕'), 50);
        assert_eq!(model.bigram_count('녕', '하'), 30);
        assert_eq!(model.bigram_count('없', '음'), 0); // 없는 바이그램
    }

    #[test]
    fn test_score_calculation() {
        let model = NgramModel::from_json(sample_model_json()).unwrap();

        // 학습된 텍스트는 높은 스코어
        let score_known = model.score("안녕하세요");
        assert!(score_known > f64::NEG_INFINITY);

        // 빈 텍스트
        let score_empty = model.score("");
        assert!(score_empty == f64::NEG_INFINITY);

        // 학습되지 않은 텍스트는 낮은 스코어
        let score_unknown = model.score("없는문장");
        assert!(score_unknown < score_known);
    }

    #[test]
    fn test_empty_model() {
        let model = NgramModel::empty();
        assert!(model.is_empty());
        // Empty model still computes score with smoothing
        let score = model.score("안녕");
        assert!(score < -5.0); // Very low score due to no data
    }

    #[test]
    fn test_single_char() {
        let model = NgramModel::from_json(sample_model_json()).unwrap();
        let score = model.score("안");
        assert!(score > f64::NEG_INFINITY);
    }

    #[test]
    fn test_json_format_error() {
        let invalid_json = r#"{ "unigrams": "not an object" }"#;
        let result = NgramModel::from_json(invalid_json);
        assert!(matches!(result, Err(NgramError::FormatError(_))));
    }

    #[test]
    fn test_invalid_bigram_format() {
        let invalid_bigram = r#"{
            "unigrams": { "가": 10 },
            "bigrams": { "가나": 5 }
        }"#;
        let result = NgramModel::from_json(invalid_bigram);
        assert!(matches!(result, Err(NgramError::FormatError(_))));
    }
}
