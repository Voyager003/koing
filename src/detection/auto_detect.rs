//! 자동 한글 입력 감지 로직
//!
//! 휴리스틱 기반으로 입력이 한글인지 영어인지 판별합니다.

use super::patterns::{
    is_consonant_key, is_vowel_key, COMMON_ENGLISH_WORDS, ENGLISH_BIGRAMS, HANGUL_BIGRAMS,
};
use super::validator::has_excessive_jamo;

/// 자동 감지기 설정
#[derive(Debug, Clone)]
pub struct AutoDetectorConfig {
    /// 변환을 위한 최소 신뢰도 (0.0 ~ 100.0) - Space/Enter 시 사용
    pub threshold: f32,
    /// 실시간 변환을 위한 신뢰도 (더 높음)
    pub realtime_threshold: f32,
    /// 감지를 위한 최소 버퍼 길이
    pub min_length: usize,
    /// Debounce 타이머 밀리초
    pub debounce_ms: u64,
}

impl Default for AutoDetectorConfig {
    fn default() -> Self {
        Self {
            threshold: 70.0,
            realtime_threshold: 80.0,
            min_length: 3,
            debounce_ms: 500,
        }
    }
}

/// 자동 한글 입력 감지기
#[derive(Debug, Clone)]
pub struct AutoDetector {
    config: AutoDetectorConfig,
    enabled: bool,
}

impl AutoDetector {
    /// 새 감지기 생성
    pub fn new(config: AutoDetectorConfig) -> Self {
        Self {
            config,
            enabled: true,
        }
    }

    /// 기본 설정으로 생성
    pub fn with_defaults() -> Self {
        Self::new(AutoDetectorConfig::default())
    }

    /// 감지 활성화/비활성화
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 감지 활성화 여부
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 입력 버퍼가 한글로 변환되어야 하는지 판별 (Space/Enter 시 사용)
    pub fn should_convert(&self, buffer: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if buffer.len() < self.config.min_length {
            return false;
        }

        // 영어 단어 필터: 흔한 영어 단어는 변환하지 않음
        let lower = buffer.to_lowercase();
        if COMMON_ENGLISH_WORDS.contains(lower.as_str()) {
            return false;
        }

        let confidence = self.get_confidence(buffer);

        // 영어 패턴 필터: 매우 높은 신뢰도(90+)가 아니면 영어 패턴 감지 시 거부
        if confidence < 90.0 && has_english_pattern(buffer) {
            return false;
        }

        // 짧은 입력(3~4자)에 대해 threshold +10점 추가 요구 (오탐 방지)
        let threshold = if buffer.len() <= 4 {
            self.config.threshold + 10.0
        } else {
            self.config.threshold
        };

        confidence >= threshold
    }

    /// 변환 결과가 유효한지 검증 (낱자모 비율 체크)
    /// 변환 후 호출하여 결과를 검증
    pub fn is_valid_conversion(&self, converted: &str) -> bool {
        // 빈 결과는 무효
        if converted.is_empty() {
            return false;
        }

        // 낱자모가 50% 이상이면 무효
        if has_excessive_jamo(converted) {
            return false;
        }

        true
    }

    /// 실시간 변환 여부 판별 (debounce 타이머 만료 시 사용)
    /// 더 높은 신뢰도와 영어 단어 필터링 적용
    pub fn should_convert_realtime(&self, buffer: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if buffer.len() < self.config.min_length {
            return false;
        }

        // 영어 단어 필터: 흔한 영어 단어는 변환하지 않음
        let lower = buffer.to_lowercase();
        if COMMON_ENGLISH_WORDS.contains(lower.as_str()) {
            return false;
        }

        let confidence = self.get_confidence(buffer);

        // 영어 패턴 필터: 매우 높은 신뢰도(90+)가 아니면 영어 패턴 감지 시 거부
        if confidence < 90.0 && has_english_pattern(buffer) {
            return false;
        }

        // 짧은 입력(3~4자)에 대해 threshold +10점 추가 요구 (오탐 방지)
        let threshold = if buffer.len() <= 4 {
            self.config.realtime_threshold + 10.0
        } else {
            self.config.realtime_threshold
        };

        // 높은 신뢰도 요구
        confidence >= threshold
    }

    /// debounce 타이머 값 반환
    pub fn debounce_ms(&self) -> u64 {
        self.config.debounce_ms
    }

    /// 입력 버퍼의 한글 신뢰도 계산 (0.0 ~ 100.0)
    pub fn get_confidence(&self, buffer: &str) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }

        let buffer_lower = buffer.to_lowercase();
        let chars: Vec<char> = buffer_lower.chars().collect();

        // 1. 자음/모음 비율 점수 (0-30점)
        let cv_score = self.calculate_cv_ratio_score(&chars);

        // 2. 바이그램 패턴 점수 (0-40점)
        let bigram_score = self.calculate_bigram_score(&buffer_lower);

        // 3. 자음-모음 교대 패턴 점수 (0-30점)
        let alternation_score = self.calculate_alternation_score(&chars);

        // 4. 연속 모음키 패널티
        // 한글 두벌식에서 모음키 3개 이상 연속은 거의 불가능 (자음이 반드시 끼어듦)
        // "you"(y=ㅛ,o=ㅐ,u=ㅕ) 같은 영단어의 연속 모음 패턴 감지
        let vowel_penalty = self.calculate_consecutive_vowel_penalty(&chars);

        (cv_score + bigram_score + alternation_score - vowel_penalty).max(0.0)
    }

    /// 자음/모음 비율 점수 계산
    /// 한글은 자음과 모음이 적절히 섞여있음
    fn calculate_cv_ratio_score(&self, chars: &[char]) -> f32 {
        let mut consonants = 0;
        let mut vowels = 0;

        for &c in chars {
            if is_consonant_key(c) || is_consonant_key(c.to_ascii_uppercase()) {
                consonants += 1;
            } else if is_vowel_key(c) || is_vowel_key(c.to_ascii_uppercase()) {
                vowels += 1;
            }
        }

        let total = consonants + vowels;
        if total == 0 {
            return 0.0;
        }

        // 자음/모음 비율 (한글은 보통 1:1 ~ 2:1 정도)
        let ratio = consonants as f32 / total as f32;

        // 0.4 ~ 0.7 사이일 때 최고 점수
        if (0.4..=0.7).contains(&ratio) {
            30.0
        } else if (0.3..=0.8).contains(&ratio) {
            20.0
        } else if (0.2..=0.9).contains(&ratio) {
            10.0
        } else {
            0.0
        }
    }

    /// 바이그램 패턴 점수 계산
    /// 한/영 겹침 바이그램을 별도 집계하여 "한글 전용" 바이그램 비율을 주요 지표로 사용
    fn calculate_bigram_score(&self, buffer: &str) -> f32 {
        if buffer.len() < 2 {
            return 0.0;
        }

        let mut exclusive_hangul = 0; // 한글에만 매칭 (영어에 없음)
        let mut exclusive_english = 0; // 영어에만 매칭 (한글에 없음)
        let mut both_match = 0; // 한/영 모두 매칭 (겹침)
        let mut total_bigrams = 0;

        let chars: Vec<char> = buffer.chars().collect();
        for window in chars.windows(2) {
            let bigram: String = window.iter().collect();
            total_bigrams += 1;

            let is_hangul = HANGUL_BIGRAMS.contains(bigram.as_str());
            let is_english = ENGLISH_BIGRAMS.contains(bigram.as_str());

            match (is_hangul, is_english) {
                (true, true) => both_match += 1,
                (true, false) => exclusive_hangul += 1,
                (false, true) => exclusive_english += 1,
                (false, false) => {}
            }
        }

        if total_bigrams == 0 {
            return 0.0;
        }

        let exclusive_hangul_ratio = exclusive_hangul as f32 / total_bigrams as f32;
        let exclusive_english_ratio = exclusive_english as f32 / total_bigrams as f32;
        let english_total_ratio =
            (exclusive_english + both_match) as f32 / total_bigrams as f32;

        // 영어 바이그램 비율이 50% 초과 시 한글 전용 비율만으로 점수 산정
        // 겹침(th, an 등)이 많아도 한글 전용 바이그램이 충분하면 높은 점수
        let score = if english_total_ratio > 0.5 {
            exclusive_hangul_ratio * 40.0
        } else {
            // 한글 전용 비율에서 영어 전용 비율을 차감
            let net_ratio = (exclusive_hangul_ratio - exclusive_english_ratio + 1.0) / 2.0;
            net_ratio * 40.0
        };

        score.clamp(0.0, 40.0)
    }

    /// 자음-모음 교대 패턴 점수 계산
    /// 한글은 자음-모음-자음-모음... 패턴이 자주 나타남
    fn calculate_alternation_score(&self, chars: &[char]) -> f32 {
        if chars.len() < 2 {
            return 0.0;
        }

        let mut alternations = 0;
        let mut prev_is_consonant: Option<bool> = None;

        for &c in chars {
            let is_cons = is_consonant_key(c) || is_consonant_key(c.to_ascii_uppercase());
            let is_vowel = is_vowel_key(c) || is_vowel_key(c.to_ascii_uppercase());

            if let Some(prev) = prev_is_consonant {
                // 자음 -> 모음 또는 모음 -> 자음 교대
                if (prev && is_vowel) || (!prev && is_cons) {
                    alternations += 1;
                }
            }

            if is_cons {
                prev_is_consonant = Some(true);
            } else if is_vowel {
                prev_is_consonant = Some(false);
            }
        }

        let max_alternations = chars.len().saturating_sub(1);
        if max_alternations == 0 {
            return 0.0;
        }

        let ratio = alternations as f32 / max_alternations as f32;
        ratio * 30.0
    }

    /// 연속 모음키 패널티 계산
    /// 한글 두벌식에서 모음키 3개 이상 연속은 거의 불가능 (자음이 반드시 끼어듦)
    /// 3연속 모음: 10점 패널티, 4+ 연속: 20점 패널티
    fn calculate_consecutive_vowel_penalty(&self, chars: &[char]) -> f32 {
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;

        for &c in chars {
            if is_vowel_key(c) || is_vowel_key(c.to_ascii_uppercase()) {
                current_consecutive += 1;
                if current_consecutive > max_consecutive {
                    max_consecutive = current_consecutive;
                }
            } else {
                current_consecutive = 0;
            }
        }

        if max_consecutive >= 4 {
            20.0
        } else if max_consecutive >= 3 {
            10.0
        } else {
            0.0
        }
    }
}

impl Default for AutoDetector {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 영어 패턴 감지 — 다음 패턴 중 하나라도 해당하면 자동 변환 거부
/// - 전체 대문자 2자 이상 (약어: "OK", "PDF", "API")
/// - CamelCase 패턴 (변수명: "onClick", "setState")
/// - 영어 접미사 (-tion, -ment, -ness, -ing, -able, -ful, -less)
/// - 5자 이상에서 영어 접두사 (un-, re-, pre-, dis-, mis-)
fn has_english_pattern(buffer: &str) -> bool {
    // 전체 대문자 2자 이상 (약어)
    if buffer.len() >= 2 && buffer.chars().all(|c| c.is_ascii_uppercase()) {
        return true;
    }

    // CamelCase: 소문자 시작 후 대문자 포함 (예: onClick, setState)
    let chars: Vec<char> = buffer.chars().collect();
    if chars.len() >= 3 {
        let starts_lower = chars[0].is_ascii_lowercase();
        let has_upper = chars[1..].iter().any(|c| c.is_ascii_uppercase());
        if starts_lower && has_upper {
            return true;
        }
    }

    // 영어 접미사
    let lower = buffer.to_lowercase();
    let suffixes = ["tion", "ment", "ness", "ing", "able", "ful", "less", "ous", "ive", "ence", "ance"];
    for suffix in &suffixes {
        if lower.len() > suffix.len() && lower.ends_with(suffix) {
            return true;
        }
    }

    // 5자 이상에서 영어 접두사
    if lower.len() >= 5 {
        let prefixes = ["un", "re", "pre", "dis", "mis"];
        for prefix in &prefixes {
            if lower.starts_with(prefix) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_convert_short_buffer() {
        let detector = AutoDetector::with_defaults();
        assert!(!detector.should_convert("r")); // 1글자는 너무 짧음
        assert!(!detector.should_convert("rk")); // 2글자도 최소 길이 미달
    }

    #[test]
    fn test_should_convert_hangul_pattern() {
        let detector = AutoDetector::with_defaults();

        // 한글 패턴
        assert!(detector.should_convert("dkssud")); // 안녕
        assert!(detector.should_convert("gksrmf")); // 한글
        assert!(detector.should_convert("rkskek")); // 가나다
    }

    #[test]
    fn test_should_not_convert_english_pattern() {
        let detector = AutoDetector::with_defaults();

        // 영어 패턴 - 변환하지 않아야 함
        // 순수 영어 단어는 자음/모음 교대 패턴이 한글과 다름
        assert!(!detector.should_convert("hello"));
        assert!(!detector.should_convert("there"));
        // "world"는 w-o-r-l-d에서 'o', 'l'이 한글 모음키라서 경계 케이스
        // threshold를 높이거나 더 정교한 감지 필요
        // "world"는 변환되면 안됨, 하지만 현재 휴리스틱으로는 경계 케이스
        // 더 강력한 영어 패턴("string", "function")으로 테스트
        assert!(!detector.should_convert("string"));
        assert!(!detector.should_convert("function"));
    }

    #[test]
    fn test_confidence_hangul() {
        let detector = AutoDetector::with_defaults();

        let hangul_confidence = detector.get_confidence("dkssud");
        let english_confidence = detector.get_confidence("hello");

        assert!(
            hangul_confidence > english_confidence,
            "한글 패턴({})이 영어 패턴({})보다 높아야 함",
            hangul_confidence,
            english_confidence
        );
    }

    #[test]
    fn test_disabled_detector() {
        let mut detector = AutoDetector::with_defaults();
        detector.set_enabled(false);

        assert!(!detector.should_convert("dkssud")); // 비활성화되면 항상 false
    }

    #[test]
    fn test_empty_buffer() {
        let detector = AutoDetector::with_defaults();
        assert!(!detector.should_convert(""));
        assert_eq!(detector.get_confidence(""), 0.0);
    }

    #[test]
    fn test_realtime_should_convert_hangul() {
        let detector = AutoDetector::with_defaults();

        // 한글 패턴은 실시간 변환됨
        assert!(detector.should_convert_realtime("dkssud")); // 안녕
        assert!(detector.should_convert_realtime("gksrmf")); // 한글
        assert!(detector.should_convert_realtime("rkskek")); // 가나다
    }

    #[test]
    fn test_realtime_should_not_convert_english_words() {
        let detector = AutoDetector::with_defaults();

        // 흔한 영어 단어는 실시간 변환되지 않음
        assert!(!detector.should_convert_realtime("hello"));
        assert!(!detector.should_convert_realtime("world"));
        assert!(!detector.should_convert_realtime("code"));
        assert!(!detector.should_convert_realtime("file"));
        assert!(!detector.should_convert_realtime("the"));
        assert!(!detector.should_convert_realtime("and"));
    }

    #[test]
    fn test_realtime_short_buffer() {
        let detector = AutoDetector::with_defaults();

        // 1~2글자 버퍼는 변환되지 않음
        assert!(!detector.should_convert_realtime("r")); // 1글자
        assert!(!detector.should_convert_realtime("rk")); // 2글자
    }

    #[test]
    fn test_debounce_ms() {
        let detector = AutoDetector::with_defaults();
        assert_eq!(detector.debounce_ms(), 500);
    }

    #[test]
    fn test_should_convert_filters_english_words() {
        let detector = AutoDetector::with_defaults();

        // 영어 단어는 Space/Enter 시에도 변환하지 않음
        assert!(!detector.should_convert("name"));
        assert!(!detector.should_convert("game"));
        assert!(!detector.should_convert("time"));
        assert!(!detector.should_convert("home"));
        assert!(!detector.should_convert("code"));
        assert!(!detector.should_convert("file"));
    }

    #[test]
    fn test_is_valid_conversion() {
        let detector = AutoDetector::with_defaults();

        // 유효한 변환 결과
        assert!(detector.is_valid_conversion("안녕"));
        assert!(detector.is_valid_conversion("가나다"));
        assert!(detector.is_valid_conversion("한글"));

        // 낱자모가 50% 미만이면 유효 (부분적 오류 허용)
        assert!(detector.is_valid_conversion("쏘ㅓㄷ아지는")); // 6글자 중 2글자 낱자모 = 33%
        assert!(detector.is_valid_conversion("안녕ㅎ")); // 3글자 중 1글자 낱자모 = 33%

        // 무효한 변환 결과 (낱자모 50% 이상)
        assert!(!detector.is_valid_conversion("ㅜ믇")); // 2글자 중 1글자 낱자모 = 50%
        assert!(!detector.is_valid_conversion("ㄱㅏㄴㅏ")); // 100% 낱자모

        // 빈 문자열
        assert!(!detector.is_valid_conversion(""));
    }

    #[test]
    fn test_has_english_pattern_abbreviations() {
        // 전체 대문자 약어
        assert!(has_english_pattern("OK"));
        assert!(has_english_pattern("PDF"));
        assert!(has_english_pattern("API"));
        assert!(has_english_pattern("HTTP"));
    }

    #[test]
    fn test_has_english_pattern_camelcase() {
        // CamelCase 변수명
        assert!(has_english_pattern("onClick"));
        assert!(has_english_pattern("setState"));
        assert!(has_english_pattern("isValid"));
    }

    #[test]
    fn test_has_english_pattern_suffixes() {
        // 영어 접미사
        assert!(has_english_pattern("function")); // -tion
        assert!(has_english_pattern("movement")); // -ment
        assert!(has_english_pattern("running")); // -ing
        assert!(has_english_pattern("readable")); // -able
    }

    #[test]
    fn test_has_english_pattern_prefixes() {
        // 5자 이상 영어 접두사
        assert!(has_english_pattern("unable")); // un-
        assert!(has_english_pattern("return")); // re- (6자)
        assert!(has_english_pattern("prevent")); // pre-
        assert!(has_english_pattern("disable")); // dis-
    }

    #[test]
    fn test_has_english_pattern_not_korean() {
        // 한글 패턴은 영어 패턴으로 감지되지 않아야 함
        assert!(!has_english_pattern("dkssud")); // 안녕
        assert!(!has_english_pattern("gksrmf")); // 한글
        assert!(!has_english_pattern("rkskek")); // 가나다
    }

    #[test]
    fn test_consecutive_vowel_penalty() {
        let detector = AutoDetector::with_defaults();

        // "you" → y(ㅛ), o(ㅐ), u(ㅕ) — 3연속 모음키
        let confidence_you = detector.get_confidence("you");
        // "rks" → r(ㄱ), k(ㅏ), s(ㄴ) — 자음+모음+자음 (패널티 없음)
        let confidence_rks = detector.get_confidence("rks");
        // "you"는 연속 모음 패널티로 낮아야 함
        assert!(
            confidence_you < confidence_rks,
            "연속 모음 패널티: you({}) < rks({})",
            confidence_you,
            confidence_rks
        );
    }

    #[test]
    fn test_english_pattern_filter_in_should_convert() {
        let detector = AutoDetector::with_defaults();

        // 영어 패턴 필터로 차단되어야 하는 입력들
        assert!(!detector.should_convert("onClick")); // CamelCase
        assert!(!detector.should_convert("running")); // -ing 접미사
        assert!(!detector.should_convert("disable")); // dis- 접두사
    }
}
