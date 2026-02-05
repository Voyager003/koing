//! 영문 -> 한글 통합 변환기

use crate::core::hangul_fsm::HangulFsm;
use crate::core::jamo_mapper::map_to_jamo;

/// 영문 문자열을 한글 문자열로 변환
/// 변환할 수 없는 문자(숫자, 특수문자, 매핑 없는 영문)는 그대로 유지
pub fn convert(input: &str) -> String {
    let mut fsm = HangulFsm::new();

    for c in input.chars() {
        if let Some(jamo) = map_to_jamo(c) {
            fsm.feed(jamo);
        } else {
            fsm.feed_passthrough(c);
        }
    }

    fsm.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_conversion() {
        assert_eq!(convert("rkskek"), "가나다");
        assert_eq!(convert("dkssudgktpdy"), "안녕하세요");
    }

    #[test]
    fn test_jongseong() {
        assert_eq!(convert("gksrmf"), "한글");
        assert_eq!(convert("dkswl"), "안지");
    }

    #[test]
    fn test_complex_vowel() {
        assert_eq!(convert("dhksfy"), "완료");
    }

    #[test]
    fn test_complex_jongseong() {
        assert_eq!(convert("dlfr"), "읽");
    }

    #[test]
    fn test_double_consonant() {
        assert_eq!(convert("Tks"), "싼");
        assert_eq!(convert("Rk"), "까");
    }

    #[test]
    fn test_mixed_input() {
        assert_eq!(convert("123rksk"), "123가나");
        assert_eq!(convert("rk!sk"), "가!나");
    }

    #[test]
    fn test_english_passthrough() {
        // 매핑되지 않는 영문자는 그대로
        assert_eq!(convert("X"), "X");
        assert_eq!(convert("rkXsk"), "가X나");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(convert(""), "");
    }

    #[test]
    fn test_jongseong_split() {
        // 종성이 다음 초성으로 분리
        assert_eq!(convert("rkrkrl"), "가가기");
    }
}
