//! 유니코드 한글 조합/분해 유틸리티

/// 한글 음절 시작 코드포인트 (가)
const HANGUL_SYLLABLE_BASE: u32 = 0xAC00;

/// 초성 개수
const CHOSEONG_COUNT: u32 = 19;
/// 중성 개수
const JUNGSEONG_COUNT: u32 = 21;
/// 종성 개수 (종성 없음 포함)
const JONGSEONG_COUNT: u32 = 28;

/// 초성/중성/종성 인덱스로 완성된 한글 유니코드 생성
/// - choseong: 초성 인덱스 (0~18)
/// - jungseong: 중성 인덱스 (0~20)
/// - jongseong: 종성 인덱스 (0~27, 0 = 종성 없음)
pub fn compose_syllable(choseong: u32, jungseong: u32, jongseong: u32) -> Option<char> {
    if choseong >= CHOSEONG_COUNT || jungseong >= JUNGSEONG_COUNT || jongseong >= JONGSEONG_COUNT {
        return None;
    }
    let code = HANGUL_SYLLABLE_BASE
        + (choseong * JUNGSEONG_COUNT + jungseong) * JONGSEONG_COUNT
        + jongseong;
    char::from_u32(code)
}

/// 완성형 한글을 초성/중성/종성 인덱스로 분해
/// 반환: (초성 인덱스, 중성 인덱스, 종성 인덱스)
pub fn decompose_syllable(c: char) -> Option<(u32, u32, u32)> {
    let code = c as u32;
    if !(HANGUL_SYLLABLE_BASE..=HANGUL_SYLLABLE_BASE + 11171).contains(&code) {
        return None;
    }
    let offset = code - HANGUL_SYLLABLE_BASE;
    let jongseong = offset % JONGSEONG_COUNT;
    let jungseong = (offset / JONGSEONG_COUNT) % JUNGSEONG_COUNT;
    let choseong = offset / (JUNGSEONG_COUNT * JONGSEONG_COUNT);
    Some((choseong, jungseong, jongseong))
}

/// 두 중성을 복합 모음으로 조합
/// 반환: 복합 모음 인덱스 (실패 시 None)
pub fn combine_jungseong(first: u32, second: u32) -> Option<u32> {
    // 복합 모음 조합 테이블
    // ㅗ(8) + ㅏ(0) = ㅘ(9)
    // ㅗ(8) + ㅐ(1) = ㅙ(10)
    // ㅗ(8) + ㅣ(20) = ㅚ(11)
    // ㅜ(13) + ㅓ(4) = ㅝ(14)
    // ㅜ(13) + ㅔ(5) = ㅞ(15)
    // ㅜ(13) + ㅣ(20) = ㅟ(16)
    // ㅡ(18) + ㅣ(20) = ㅢ(19)
    match (first, second) {
        (8, 0) => Some(9),    // ㅗ + ㅏ = ㅘ
        (8, 1) => Some(10),   // ㅗ + ㅐ = ㅙ
        (8, 20) => Some(11),  // ㅗ + ㅣ = ㅚ
        (13, 4) => Some(14),  // ㅜ + ㅓ = ㅝ
        (13, 5) => Some(15),  // ㅜ + ㅔ = ㅞ
        (13, 20) => Some(16), // ㅜ + ㅣ = ㅟ
        (18, 20) => Some(19), // ㅡ + ㅣ = ㅢ
        _ => None,
    }
}

/// 두 종성을 복합 종성으로 조합
/// 반환: 복합 종성 인덱스 (실패 시 None)
pub fn combine_jongseong(first: u32, second: u32) -> Option<u32> {
    // 복합 종성 조합 테이블
    // 종성 인덱스: 없음(0) ㄱ(1) ㄲ(2) ㄳ(3) ㄴ(4) ㄵ(5) ㄶ(6) ㄷ(7) ㄹ(8) ㄺ(9)
    // ㄻ(10) ㄼ(11) ㄽ(12) ㄾ(13) ㄿ(14) ㅀ(15) ㅁ(16) ㅂ(17) ㅄ(18) ㅅ(19)
    // ㅆ(20) ㅇ(21) ㅈ(22) ㅊ(23) ㅋ(24) ㅌ(25) ㅍ(26) ㅎ(27)
    match (first, second) {
        (1, 19) => Some(3),   // ㄱ + ㅅ = ㄳ
        (4, 22) => Some(5),   // ㄴ + ㅈ = ㄵ
        (4, 27) => Some(6),   // ㄴ + ㅎ = ㄶ
        (8, 1) => Some(9),    // ㄹ + ㄱ = ㄺ
        (8, 16) => Some(10),  // ㄹ + ㅁ = ㄻ
        (8, 17) => Some(11),  // ㄹ + ㅂ = ㄼ
        (8, 19) => Some(12),  // ㄹ + ㅅ = ㄽ
        (8, 25) => Some(13),  // ㄹ + ㅌ = ㄾ
        (8, 26) => Some(14),  // ㄹ + ㅍ = ㄿ
        (8, 27) => Some(15),  // ㄹ + ㅎ = ㅀ
        (17, 19) => Some(18), // ㅂ + ㅅ = ㅄ
        _ => None,
    }
}

/// 복합 종성을 분리
/// 반환: (첫 번째 종성 인덱스, 두 번째 종성의 초성 인덱스)
/// 두 번째 값은 다음 글자의 초성으로 사용됨
pub fn split_jongseong(jong: u32) -> Option<(u32, u32)> {
    // 복합 종성 분리 테이블
    // (남는 종성 인덱스, 분리되는 자음의 초성 인덱스)
    match jong {
        3 => Some((1, 9)),   // ㄳ -> ㄱ(종성1) + ㅅ(초성9)
        5 => Some((4, 12)),  // ㄵ -> ㄴ(종성4) + ㅈ(초성12)
        6 => Some((4, 18)),  // ㄶ -> ㄴ(종성4) + ㅎ(초성18)
        9 => Some((8, 0)),   // ㄺ -> ㄹ(종성8) + ㄱ(초성0)
        10 => Some((8, 6)),  // ㄻ -> ㄹ(종성8) + ㅁ(초성6)
        11 => Some((8, 7)),  // ㄼ -> ㄹ(종성8) + ㅂ(초성7)
        12 => Some((8, 9)),  // ㄽ -> ㄹ(종성8) + ㅅ(초성9)
        13 => Some((8, 16)), // ㄾ -> ㄹ(종성8) + ㅌ(초성16)
        14 => Some((8, 17)), // ㄿ -> ㄹ(종성8) + ㅍ(초성17)
        15 => Some((8, 18)), // ㅀ -> ㄹ(종성8) + ㅎ(초성18)
        18 => Some((17, 9)), // ㅄ -> ㅂ(종성17) + ㅅ(초성9)
        _ => None,
    }
}

/// 단일 종성을 초성 인덱스로 변환
/// 종성이 다음 글자의 초성으로 이동할 때 사용
pub fn jongseong_to_choseong(jong: u32) -> Option<u32> {
    // 종성 인덱스 -> 초성 인덱스 변환
    match jong {
        1 => Some(0),   // ㄱ
        2 => Some(1),   // ㄲ
        4 => Some(2),   // ㄴ
        7 => Some(3),   // ㄷ
        8 => Some(5),   // ㄹ
        16 => Some(6),  // ㅁ
        17 => Some(7),  // ㅂ
        19 => Some(9),  // ㅅ
        20 => Some(10), // ㅆ
        21 => Some(11), // ㅇ
        22 => Some(12), // ㅈ
        23 => Some(14), // ㅊ
        24 => Some(15), // ㅋ
        25 => Some(16), // ㅌ
        26 => Some(17), // ㅍ
        27 => Some(18), // ㅎ
        _ => None,
    }
}

/// 초성만 있을 때 해당 자모 문자 반환 (호환용 자모)
pub fn choseong_to_jamo_char(cho: u32) -> Option<char> {
    if cho < 19 {
        // 호환용 자모: 초성 순서와 다르므로 직접 매핑
        #[rustfmt::skip]
        let jamo_codes: [u32; 19] = [
            0x3131, // ㄱ
            0x3132, // ㄲ
            0x3134, // ㄴ
            0x3137, // ㄷ
            0x3138, // ㄸ
            0x3139, // ㄹ
            0x3141, // ㅁ
            0x3142, // ㅂ
            0x3143, // ㅃ
            0x3145, // ㅅ
            0x3146, // ㅆ
            0x3147, // ㅇ
            0x3148, // ㅈ
            0x3149, // ㅉ
            0x314A, // ㅊ
            0x314B, // ㅋ
            0x314C, // ㅌ
            0x314D, // ㅍ
            0x314E, // ㅎ
        ];
        char::from_u32(jamo_codes[cho as usize])
    } else {
        None
    }
}

/// 중성만 있을 때 해당 모음 문자 반환 (호환용 자모)
pub fn jungseong_to_jamo_char(jung: u32) -> Option<char> {
    if jung < 21 {
        // 호환용 모음 자모: ㅏ(0x314F) ~ ㅣ(0x3163)
        let jamo_codes: [u32; 21] = [
            0x314F, // ㅏ
            0x3150, // ㅐ
            0x3151, // ㅑ
            0x3152, // ㅒ
            0x3153, // ㅓ
            0x3154, // ㅔ
            0x3155, // ㅕ
            0x3156, // ㅖ
            0x3157, // ㅗ
            0x3158, // ㅘ
            0x3159, // ㅙ
            0x315A, // ㅚ
            0x315B, // ㅛ
            0x315C, // ㅜ
            0x315D, // ㅝ
            0x315E, // ㅞ
            0x315F, // ㅟ
            0x3160, // ㅠ
            0x3161, // ㅡ
            0x3162, // ㅢ
            0x3163, // ㅣ
        ];
        char::from_u32(jamo_codes[jung as usize])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_syllable() {
        // 가 = 초성 ㄱ(0) + 중성 ㅏ(0) + 종성 없음(0)
        assert_eq!(compose_syllable(0, 0, 0), Some('가'));
        // 각 = 초성 ㄱ(0) + 중성 ㅏ(0) + 종성 ㄱ(1)
        assert_eq!(compose_syllable(0, 0, 1), Some('각'));
        // 한 = 초성 ㅎ(18) + 중성 ㅏ(0) + 종성 ㄴ(4)
        assert_eq!(compose_syllable(18, 0, 4), Some('한'));
        // 글 = 초성 ㄱ(0) + 중성 ㅡ(18) + 종성 ㄹ(8)
        assert_eq!(compose_syllable(0, 18, 8), Some('글'));
    }

    #[test]
    fn test_decompose_syllable() {
        assert_eq!(decompose_syllable('가'), Some((0, 0, 0)));
        assert_eq!(decompose_syllable('각'), Some((0, 0, 1)));
        assert_eq!(decompose_syllable('한'), Some((18, 0, 4)));
        assert_eq!(decompose_syllable('글'), Some((0, 18, 8)));

        // 한글이 아닌 문자
        assert_eq!(decompose_syllable('a'), None);
        assert_eq!(decompose_syllable('1'), None);
    }

    #[test]
    fn test_combine_jungseong() {
        assert_eq!(combine_jungseong(8, 0), Some(9)); // ㅗ + ㅏ = ㅘ
        assert_eq!(combine_jungseong(8, 1), Some(10)); // ㅗ + ㅐ = ㅙ
        assert_eq!(combine_jungseong(8, 20), Some(11)); // ㅗ + ㅣ = ㅚ
        assert_eq!(combine_jungseong(13, 4), Some(14)); // ㅜ + ㅓ = ㅝ
        assert_eq!(combine_jungseong(13, 5), Some(15)); // ㅜ + ㅔ = ㅞ
        assert_eq!(combine_jungseong(13, 20), Some(16)); // ㅜ + ㅣ = ㅟ
        assert_eq!(combine_jungseong(18, 20), Some(19)); // ㅡ + ㅣ = ㅢ

        // 조합 불가
        assert_eq!(combine_jungseong(0, 0), None);
        assert_eq!(combine_jungseong(8, 8), None);
    }

    #[test]
    fn test_combine_jongseong() {
        assert_eq!(combine_jongseong(1, 19), Some(3)); // ㄱ + ㅅ = ㄳ
        assert_eq!(combine_jongseong(4, 22), Some(5)); // ㄴ + ㅈ = ㄵ
        assert_eq!(combine_jongseong(4, 27), Some(6)); // ㄴ + ㅎ = ㄶ
        assert_eq!(combine_jongseong(8, 1), Some(9)); // ㄹ + ㄱ = ㄺ
        assert_eq!(combine_jongseong(8, 16), Some(10)); // ㄹ + ㅁ = ㄻ
        assert_eq!(combine_jongseong(8, 17), Some(11)); // ㄹ + ㅂ = ㄼ
        assert_eq!(combine_jongseong(8, 19), Some(12)); // ㄹ + ㅅ = ㄽ
        assert_eq!(combine_jongseong(17, 19), Some(18)); // ㅂ + ㅅ = ㅄ

        // 조합 불가
        assert_eq!(combine_jongseong(1, 1), None);
    }

    #[test]
    fn test_split_jongseong() {
        assert_eq!(split_jongseong(3), Some((1, 9))); // ㄳ -> ㄱ + ㅅ
        assert_eq!(split_jongseong(9), Some((8, 0))); // ㄺ -> ㄹ + ㄱ
        assert_eq!(split_jongseong(18), Some((17, 9))); // ㅄ -> ㅂ + ㅅ

        // 단일 종성은 분리 불가
        assert_eq!(split_jongseong(1), None);
        assert_eq!(split_jongseong(4), None);
    }

    #[test]
    fn test_jongseong_to_choseong() {
        assert_eq!(jongseong_to_choseong(1), Some(0)); // ㄱ
        assert_eq!(jongseong_to_choseong(4), Some(2)); // ㄴ
        assert_eq!(jongseong_to_choseong(8), Some(5)); // ㄹ
        assert_eq!(jongseong_to_choseong(27), Some(18)); // ㅎ

        // 복합 종성은 변환 불가 (split_jongseong 사용해야 함)
        assert_eq!(jongseong_to_choseong(3), None); // ㄳ
        assert_eq!(jongseong_to_choseong(9), None); // ㄺ
    }

    #[test]
    fn test_choseong_to_jamo_char() {
        assert_eq!(choseong_to_jamo_char(0), Some('ㄱ'));
        assert_eq!(choseong_to_jamo_char(1), Some('ㄲ'));
        assert_eq!(choseong_to_jamo_char(2), Some('ㄴ'));
        assert_eq!(choseong_to_jamo_char(18), Some('ㅎ'));
        assert_eq!(choseong_to_jamo_char(19), None);
    }

    #[test]
    fn test_jungseong_to_jamo_char() {
        assert_eq!(jungseong_to_jamo_char(0), Some('ㅏ'));
        assert_eq!(jungseong_to_jamo_char(8), Some('ㅗ'));
        assert_eq!(jungseong_to_jamo_char(20), Some('ㅣ'));
        assert_eq!(jungseong_to_jamo_char(21), None);
    }
}
