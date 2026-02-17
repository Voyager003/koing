//! 한글 → 영문 역변환 (두벌식 자판 기준)
//!
//! 완성형 한글을 두벌식 자판의 영문 키 시퀀스로 역변환합니다.
//! 기존 core 모듈의 unicode.rs를 활용합니다.

use crate::core::unicode::decompose_syllable;

/// 한글 문자열을 두벌식 영문 키 시퀀스로 역변환
///
/// # Examples
/// ```
/// use koing::ngram::korean_to_eng;
/// assert_eq!(korean_to_eng("안녕"), "dkssud");
/// assert_eq!(korean_to_eng("한글"), "gksrmf");
/// ```
pub fn korean_to_eng(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);

    for c in input.chars() {
        if let Some((cho, jung, jong)) = decompose_syllable(c) {
            // 초성 -> 영문
            if let Some(eng) = choseong_to_eng(cho) {
                result.push(eng);
            }
            // 중성 -> 영문 (복합 모음은 여러 키)
            jungseong_to_eng(jung, &mut result);
            // 종성 -> 영문 (복합 종성은 여러 키)
            jongseong_to_eng(jong, &mut result);
        } else {
            // 한글이 아닌 문자는 그대로 유지
            result.push(c);
        }
    }

    result
}

/// 초성 인덱스 -> 영문 키
fn choseong_to_eng(cho: u32) -> Option<char> {
    // 초성 인덱스: ㄱ(0) ㄲ(1) ㄴ(2) ㄷ(3) ㄸ(4) ㄹ(5) ㅁ(6) ㅂ(7) ㅃ(8) ㅅ(9)
    //            ㅆ(10) ㅇ(11) ㅈ(12) ㅉ(13) ㅊ(14) ㅋ(15) ㅌ(16) ㅍ(17) ㅎ(18)
    match cho {
        0 => Some('r'),  // ㄱ
        1 => Some('R'),  // ㄲ
        2 => Some('s'),  // ㄴ
        3 => Some('e'),  // ㄷ
        4 => Some('E'),  // ㄸ
        5 => Some('f'),  // ㄹ
        6 => Some('a'),  // ㅁ
        7 => Some('q'),  // ㅂ
        8 => Some('Q'),  // ㅃ
        9 => Some('t'),  // ㅅ
        10 => Some('T'), // ㅆ
        11 => Some('d'), // ㅇ
        12 => Some('w'), // ㅈ
        13 => Some('W'), // ㅉ
        14 => Some('c'), // ㅊ
        15 => Some('z'), // ㅋ
        16 => Some('x'), // ㅌ
        17 => Some('v'), // ㅍ
        18 => Some('g'), // ㅎ
        _ => None,
    }
}

/// 중성 인덱스 -> 영문 키 (복합 모음은 여러 키 추가)
fn jungseong_to_eng(jung: u32, result: &mut String) {
    // 중성 인덱스: ㅏ(0) ㅐ(1) ㅑ(2) ㅒ(3) ㅓ(4) ㅔ(5) ㅕ(6) ㅖ(7) ㅗ(8) ㅘ(9)
    //            ㅙ(10) ㅚ(11) ㅛ(12) ㅜ(13) ㅝ(14) ㅞ(15) ㅟ(16) ㅠ(17) ㅡ(18) ㅢ(19) ㅣ(20)
    match jung {
        0 => result.push('k'),  // ㅏ
        1 => result.push('o'),  // ㅐ
        2 => result.push('i'),  // ㅑ
        3 => result.push('O'),  // ㅒ
        4 => result.push('j'),  // ㅓ
        5 => result.push('p'),  // ㅔ
        6 => result.push('u'),  // ㅕ
        7 => result.push('P'),  // ㅖ
        8 => result.push('h'),  // ㅗ
        9 => {
            // ㅘ = ㅗ + ㅏ
            result.push('h');
            result.push('k');
        }
        10 => {
            // ㅙ = ㅗ + ㅐ
            result.push('h');
            result.push('o');
        }
        11 => {
            // ㅚ = ㅗ + ㅣ
            result.push('h');
            result.push('l');
        }
        12 => result.push('y'), // ㅛ
        13 => result.push('n'), // ㅜ
        14 => {
            // ㅝ = ㅜ + ㅓ
            result.push('n');
            result.push('j');
        }
        15 => {
            // ㅞ = ㅜ + ㅔ
            result.push('n');
            result.push('p');
        }
        16 => {
            // ㅟ = ㅜ + ㅣ
            result.push('n');
            result.push('l');
        }
        17 => result.push('b'), // ㅠ
        18 => result.push('m'), // ㅡ
        19 => {
            // ㅢ = ㅡ + ㅣ
            result.push('m');
            result.push('l');
        }
        20 => result.push('l'), // ㅣ
        _ => {}
    }
}

/// 종성 인덱스 -> 영문 키 (복합 종성은 여러 키 추가)
fn jongseong_to_eng(jong: u32, result: &mut String) {
    // 종성 인덱스: 없음(0) ㄱ(1) ㄲ(2) ㄳ(3) ㄴ(4) ㄵ(5) ㄶ(6) ㄷ(7) ㄹ(8) ㄺ(9)
    //            ㄻ(10) ㄼ(11) ㄽ(12) ㄾ(13) ㄿ(14) ㅀ(15) ㅁ(16) ㅂ(17) ㅄ(18) ㅅ(19)
    //            ㅆ(20) ㅇ(21) ㅈ(22) ㅊ(23) ㅋ(24) ㅌ(25) ㅍ(26) ㅎ(27)
    match jong {
        0 => {} // 종성 없음
        1 => result.push('r'),  // ㄱ
        2 => result.push('R'), // ㄲ (Shift+R 한 번)
        3 => {
            // ㄳ = ㄱ + ㅅ
            result.push('r');
            result.push('t');
        }
        4 => result.push('s'),  // ㄴ
        5 => {
            // ㄵ = ㄴ + ㅈ
            result.push('s');
            result.push('w');
        }
        6 => {
            // ㄶ = ㄴ + ㅎ
            result.push('s');
            result.push('g');
        }
        7 => result.push('e'),  // ㄷ
        8 => result.push('f'),  // ㄹ
        9 => {
            // ㄺ = ㄹ + ㄱ
            result.push('f');
            result.push('r');
        }
        10 => {
            // ㄻ = ㄹ + ㅁ
            result.push('f');
            result.push('a');
        }
        11 => {
            // ㄼ = ㄹ + ㅂ
            result.push('f');
            result.push('q');
        }
        12 => {
            // ㄽ = ㄹ + ㅅ
            result.push('f');
            result.push('t');
        }
        13 => {
            // ㄾ = ㄹ + ㅌ
            result.push('f');
            result.push('x');
        }
        14 => {
            // ㄿ = ㄹ + ㅍ
            result.push('f');
            result.push('v');
        }
        15 => {
            // ㅀ = ㄹ + ㅎ
            result.push('f');
            result.push('g');
        }
        16 => result.push('a'), // ㅁ
        17 => result.push('q'), // ㅂ
        18 => {
            // ㅄ = ㅂ + ㅅ
            result.push('q');
            result.push('t');
        }
        19 => result.push('t'), // ㅅ
        20 => result.push('T'), // ㅆ
        21 => result.push('d'), // ㅇ
        22 => result.push('w'), // ㅈ
        23 => result.push('c'), // ㅊ
        24 => result.push('z'), // ㅋ
        25 => result.push('x'), // ㅌ
        26 => result.push('v'), // ㅍ
        27 => result.push('g'), // ㅎ
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_syllables() {
        assert_eq!(korean_to_eng("가"), "rk");
        assert_eq!(korean_to_eng("나"), "sk");
        assert_eq!(korean_to_eng("다"), "ek");
        assert_eq!(korean_to_eng("라"), "fk");
    }

    #[test]
    fn test_with_jongseong() {
        assert_eq!(korean_to_eng("각"), "rkr");
        assert_eq!(korean_to_eng("한"), "gks");
        assert_eq!(korean_to_eng("글"), "rmf");
    }

    #[test]
    fn test_common_words() {
        assert_eq!(korean_to_eng("안녕"), "dkssud");
        assert_eq!(korean_to_eng("한글"), "gksrmf");
        assert_eq!(korean_to_eng("가나다"), "rkskek");
    }

    #[test]
    fn test_complex_vowels() {
        assert_eq!(korean_to_eng("완"), "dhks");  // ㅘ = ㅗ + ㅏ
        assert_eq!(korean_to_eng("웬"), "dnps");  // ㅞ = ㅜ + ㅔ
        assert_eq!(korean_to_eng("의"), "dml");   // ㅇ + ㅢ = d + m + l
        assert_eq!(korean_to_eng("원"), "dnjs");  // ㅝ = ㅜ + ㅓ
    }

    #[test]
    fn test_complex_jongseong() {
        assert_eq!(korean_to_eng("읽"), "dlfr");  // ㄺ = ㄹ + ㄱ
        assert_eq!(korean_to_eng("없"), "djqt");  // ㅄ = ㅂ + ㅅ
        assert_eq!(korean_to_eng("삶"), "tkfa");  // ㄻ = ㄹ + ㅁ
    }

    #[test]
    fn test_ssang_consonants() {
        assert_eq!(korean_to_eng("까"), "Rk");   // ㄲ
        assert_eq!(korean_to_eng("싸"), "Tk");   // ㅆ
        assert_eq!(korean_to_eng("빠"), "Qk");   // ㅃ
    }

    #[test]
    fn test_mixed_text() {
        assert_eq!(korean_to_eng("가1나"), "rk1sk");
        assert_eq!(korean_to_eng("안녕!"), "dkssud!");
    }

    #[test]
    fn test_non_hangul() {
        assert_eq!(korean_to_eng("abc"), "abc");
        assert_eq!(korean_to_eng("123"), "123");
        assert_eq!(korean_to_eng(""), "");
    }

    #[test]
    fn test_roundtrip_conversion() {
        // 영문 -> 한글 -> 영문 라운드트립 테스트
        use crate::core::converter::convert;

        let original = "dkssud"; // 안녕
        let korean = convert(original);
        assert_eq!(korean, "안녕");
        let back_to_eng = korean_to_eng(&korean);
        assert_eq!(back_to_eng, original);
    }
}
