//! 변환 결과 검증 모듈
//!
//! 낱자모 검출 및 가짜 한글 필터링

/// 완성형 한글이 아닌 낱자모가 포함되어 있는지 검사
///
/// 호환용 자모 영역 (ㄱ-ㅎ, ㅏ-ㅣ): U+3131 ~ U+318E
/// 이 영역의 문자가 포함되어 있으면 불완전한 한글로 판단
pub fn has_incomplete_jamo(text: &str) -> bool {
    for ch in text.chars() {
        let cp = ch as u32;
        // 호환용 자모 영역
        if (0x3131..=0x318E).contains(&cp) {
            return true;
        }
    }
    false
}

/// 낱자모 비율 계산 (0.0 ~ 1.0)
/// 한글 문자(완성형 + 낱자모) 중 낱자모의 비율
pub fn incomplete_jamo_ratio(text: &str) -> f32 {
    let mut jamo_count = 0;
    let mut hangul_count = 0;

    for ch in text.chars() {
        let cp = ch as u32;
        if (0x3131..=0x318E).contains(&cp) {
            // 낱자모
            jamo_count += 1;
            hangul_count += 1;
        } else if (0xAC00..=0xD7A3).contains(&cp) {
            // 완성형 한글
            hangul_count += 1;
        }
    }

    if hangul_count == 0 {
        return 0.0;
    }

    jamo_count as f32 / hangul_count as f32
}

/// 낱자모가 과도하게 포함되어 있는지 검사 (threshold: 50%)
/// 한글 문자 중 낱자모가 50% 이상이면 true
pub fn has_excessive_jamo(text: &str) -> bool {
    incomplete_jamo_ratio(text) >= 0.5
}

/// 문자가 완성형 한글(가-힣)인지 확인
pub fn is_complete_hangul(ch: char) -> bool {
    let cp = ch as u32;
    (0xAC00..=0xD7A3).contains(&cp)
}

/// 변환 결과가 유효한 한글인지 검증
///
/// - 낱자모 포함 시 무효
/// - 완성형 한글, ASCII, 공백만 허용
pub fn is_valid_hangul_result(converted: &str) -> bool {
    // 1. 빈 문자열은 무효
    if converted.is_empty() {
        return false;
    }

    // 2. 낱자모 포함 시 무효
    if has_incomplete_jamo(converted) {
        return false;
    }

    // 3. 모든 문자가 완성형 한글 또는 허용 문자인지 확인
    for ch in converted.chars() {
        let is_hangul = is_complete_hangul(ch);
        let is_allowed = ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() || ch == ' ';

        if !is_hangul && !is_allowed {
            return false;
        }
    }

    true
}

/// 변환 결과에 완성형 한글이 하나라도 포함되어 있는지 확인
pub fn has_any_hangul(text: &str) -> bool {
    text.chars().any(is_complete_hangul)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_incomplete_jamo() {
        // 낱자모 포함
        assert!(has_incomplete_jamo("ㅜ믇"));
        assert!(has_incomplete_jamo("ㄱㅏㄴㅏ"));
        assert!(has_incomplete_jamo("ㄱ"));
        assert!(has_incomplete_jamo("ㅏ"));
        assert!(has_incomplete_jamo("안녕ㅎ"));

        // 완성형만
        assert!(!has_incomplete_jamo("안녕"));
        assert!(!has_incomplete_jamo("가나다"));
        assert!(!has_incomplete_jamo("한글"));

        // ASCII
        assert!(!has_incomplete_jamo("hello"));
        assert!(!has_incomplete_jamo("123"));
        assert!(!has_incomplete_jamo(""));
    }

    #[test]
    fn test_is_complete_hangul() {
        assert!(is_complete_hangul('가'));
        assert!(is_complete_hangul('힣'));
        assert!(is_complete_hangul('안'));

        assert!(!is_complete_hangul('ㄱ'));
        assert!(!is_complete_hangul('ㅏ'));
        assert!(!is_complete_hangul('a'));
        assert!(!is_complete_hangul('1'));
    }

    #[test]
    fn test_is_valid_hangul_result() {
        // 유효한 결과
        assert!(is_valid_hangul_result("안녕"));
        assert!(is_valid_hangul_result("가나다"));
        assert!(is_valid_hangul_result("한글 테스트"));
        assert!(is_valid_hangul_result("안녕하세요!"));

        // 무효한 결과 (낱자모 포함)
        assert!(!is_valid_hangul_result("ㅜ믇"));
        assert!(!is_valid_hangul_result("ㄱㅏ"));
        assert!(!is_valid_hangul_result("안녕ㅎ"));

        // 빈 문자열
        assert!(!is_valid_hangul_result(""));
    }

    #[test]
    fn test_has_any_hangul() {
        assert!(has_any_hangul("안녕"));
        assert!(has_any_hangul("hello 안녕"));
        assert!(has_any_hangul("가"));

        assert!(!has_any_hangul("hello"));
        assert!(!has_any_hangul("123"));
        assert!(!has_any_hangul("ㄱㅏ")); // 낱자모는 완성형이 아님
        assert!(!has_any_hangul(""));
    }

    #[test]
    fn test_incomplete_jamo_ratio() {
        // 100% 낱자모
        assert_eq!(incomplete_jamo_ratio("ㄱㅏㄴㅏ"), 1.0);

        // 50% 낱자모
        assert!((incomplete_jamo_ratio("ㅜ믇") - 0.5).abs() < 0.01);

        // 33% 낱자모
        let ratio = incomplete_jamo_ratio("쏘ㅓㄷ아지는");
        assert!(ratio > 0.3 && ratio < 0.4);

        // 0% 낱자모
        assert_eq!(incomplete_jamo_ratio("안녕"), 0.0);

        // 한글 없음
        assert_eq!(incomplete_jamo_ratio("hello"), 0.0);
    }

    #[test]
    fn test_has_excessive_jamo() {
        // 50% 이상 → true
        assert!(has_excessive_jamo("ㅜ믇")); // 50%
        assert!(has_excessive_jamo("ㄱㅏㄴㅏ")); // 100%

        // 50% 미만 → false
        assert!(!has_excessive_jamo("쏘ㅓㄷ아지는")); // ~33%
        assert!(!has_excessive_jamo("안녕")); // 0%
        assert!(!has_excessive_jamo("안녕ㅎ")); // 33%
    }
}
