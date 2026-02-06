//! N-gram 검증 시스템 설정
//!
//! 스코어링 및 판정에 사용되는 설정값 정의

/// N-gram 검증 설정
#[derive(Debug, Clone)]
pub struct NgramConfig {
    /// Add-k 스무딩 상수 (라플라스 스무딩)
    /// 0에 가까울수록 OOV(미등록 단어)에 낮은 확률 부여
    pub smoothing_k: f64,

    /// 어휘 크기 (한글 완성형 음절 수)
    /// 11,172 = 19(초성) × 21(중성) × 28(종성)
    pub vocab_size: usize,

    /// 한글 변환 판정 임계값 (로그 확률)
    /// 스코어가 이 값 이상이면 한글로 변환
    /// 낮을수록 더 관대하게 변환 허용
    pub threshold: f64,

    /// N-gram 모델 파일 경로
    pub model_path: String,
}

impl Default for NgramConfig {
    fn default() -> Self {
        Self {
            smoothing_k: 0.001,
            vocab_size: 11172,      // 한글 완성형 음절 수
            threshold: -10.0,       // 로그 확률 기준
            model_path: String::new(),
        }
    }
}

impl NgramConfig {
    /// 새 설정 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// 모델 경로 설정
    pub fn with_model_path(mut self, path: impl Into<String>) -> Self {
        self.model_path = path.into();
        self
    }

    /// 임계값 설정
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// 스무딩 상수 설정
    pub fn with_smoothing(mut self, k: f64) -> Self {
        self.smoothing_k = k;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NgramConfig::default();
        assert_eq!(config.vocab_size, 11172);
        assert!((config.smoothing_k - 0.001).abs() < f64::EPSILON);
        assert!((config.threshold - (-10.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_builder_pattern() {
        let config = NgramConfig::new()
            .with_model_path("data/ngram_model.json")
            .with_threshold(-8.0)
            .with_smoothing(0.01);

        assert_eq!(config.model_path, "data/ngram_model.json");
        assert!((config.threshold - (-8.0)).abs() < f64::EPSILON);
        assert!((config.smoothing_k - 0.01).abs() < f64::EPSILON);
    }
}
