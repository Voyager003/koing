//! 두벌식 자판 영문 키 -> 한글 자모 매핑

/// 자모 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jamo {
    /// 자음 (cho_index: 초성 인덱스, jong_index: 종성 인덱스, None이면 종성 불가)
    Consonant {
        cho_index: u32,
        jong_index: Option<u32>,
    },
    /// 모음 (jung_index: 중성 인덱스)
    Vowel { jung_index: u32 },
}

impl Jamo {
    /// 초성 인덱스 반환 (자음인 경우만)
    pub fn choseong_index(&self) -> Option<u32> {
        match self {
            Jamo::Consonant { cho_index, .. } => Some(*cho_index),
            Jamo::Vowel { .. } => None,
        }
    }

    /// 중성 인덱스 반환 (모음인 경우만)
    pub fn jungseong_index(&self) -> Option<u32> {
        match self {
            Jamo::Vowel { jung_index } => Some(*jung_index),
            Jamo::Consonant { .. } => None,
        }
    }

    /// 종성 인덱스 반환 (자음이고 종성 가능한 경우만)
    pub fn jongseong_index(&self) -> Option<u32> {
        match self {
            Jamo::Consonant { jong_index, .. } => *jong_index,
            Jamo::Vowel { .. } => None,
        }
    }

    /// 자음인지 확인
    pub fn is_consonant(&self) -> bool {
        matches!(self, Jamo::Consonant { .. })
    }

    /// 모음인지 확인
    pub fn is_vowel(&self) -> bool {
        matches!(self, Jamo::Vowel { .. })
    }
}

/// 영문 문자 하나를 자모로 변환
/// 매핑에 없는 문자(숫자, 특수문자 등)는 None 반환
pub fn map_to_jamo(c: char) -> Option<Jamo> {
    // 초성 인덱스 순서 (19개):
    // ㄱ(0) ㄲ(1) ㄴ(2) ㄷ(3) ㄸ(4) ㄹ(5) ㅁ(6) ㅂ(7) ㅃ(8) ㅅ(9)
    // ㅆ(10) ㅇ(11) ㅈ(12) ㅉ(13) ㅊ(14) ㅋ(15) ㅌ(16) ㅍ(17) ㅎ(18)
    //
    // 종성 인덱스 순서 (28개, 0 = 없음):
    // 없음(0) ㄱ(1) ㄲ(2) ㄳ(3) ㄴ(4) ㄵ(5) ㄶ(6) ㄷ(7) ㄹ(8) ㄺ(9)
    // ㄻ(10) ㄼ(11) ㄽ(12) ㄾ(13) ㄿ(14) ㅀ(15) ㅁ(16) ㅂ(17) ㅄ(18) ㅅ(19)
    // ㅆ(20) ㅇ(21) ㅈ(22) ㅊ(23) ㅋ(24) ㅌ(25) ㅍ(26) ㅎ(27)
    //
    // 중성 인덱스 순서 (21개):
    // ㅏ(0) ㅐ(1) ㅑ(2) ㅒ(3) ㅓ(4) ㅔ(5) ㅕ(6) ㅖ(7) ㅗ(8) ㅘ(9)
    // ㅙ(10) ㅚ(11) ㅛ(12) ㅜ(13) ㅝ(14) ㅞ(15) ㅟ(16) ㅠ(17) ㅡ(18) ㅢ(19) ㅣ(20)

    match c {
        // 자음 매핑 (영문 -> 초성 인덱스, 종성 인덱스)
        'r' => Some(Jamo::Consonant {
            cho_index: 0,
            jong_index: Some(1),
        }), // ㄱ
        'R' => Some(Jamo::Consonant {
            cho_index: 1,
            jong_index: Some(2),
        }), // ㄲ
        's' => Some(Jamo::Consonant {
            cho_index: 2,
            jong_index: Some(4),
        }), // ㄴ
        'e' => Some(Jamo::Consonant {
            cho_index: 3,
            jong_index: Some(7),
        }), // ㄷ
        'E' => Some(Jamo::Consonant {
            cho_index: 4,
            jong_index: None,
        }), // ㄸ (종성 불가)
        'f' => Some(Jamo::Consonant {
            cho_index: 5,
            jong_index: Some(8),
        }), // ㄹ
        'a' => Some(Jamo::Consonant {
            cho_index: 6,
            jong_index: Some(16),
        }), // ㅁ
        'q' => Some(Jamo::Consonant {
            cho_index: 7,
            jong_index: Some(17),
        }), // ㅂ
        'Q' => Some(Jamo::Consonant {
            cho_index: 8,
            jong_index: None,
        }), // ㅃ (종성 불가)
        't' => Some(Jamo::Consonant {
            cho_index: 9,
            jong_index: Some(19),
        }), // ㅅ
        'T' => Some(Jamo::Consonant {
            cho_index: 10,
            jong_index: Some(20),
        }), // ㅆ
        'd' => Some(Jamo::Consonant {
            cho_index: 11,
            jong_index: Some(21),
        }), // ㅇ
        'w' => Some(Jamo::Consonant {
            cho_index: 12,
            jong_index: Some(22),
        }), // ㅈ
        'W' => Some(Jamo::Consonant {
            cho_index: 13,
            jong_index: None,
        }), // ㅉ (종성 불가)
        'c' => Some(Jamo::Consonant {
            cho_index: 14,
            jong_index: Some(23),
        }), // ㅊ
        'z' => Some(Jamo::Consonant {
            cho_index: 15,
            jong_index: Some(24),
        }), // ㅋ
        'x' => Some(Jamo::Consonant {
            cho_index: 16,
            jong_index: Some(25),
        }), // ㅌ
        'v' => Some(Jamo::Consonant {
            cho_index: 17,
            jong_index: Some(26),
        }), // ㅍ
        'g' => Some(Jamo::Consonant {
            cho_index: 18,
            jong_index: Some(27),
        }), // ㅎ

        // 모음 매핑 (영문 -> 중성 인덱스)
        'k' => Some(Jamo::Vowel { jung_index: 0 }),  // ㅏ
        'o' => Some(Jamo::Vowel { jung_index: 1 }),  // ㅐ
        'i' => Some(Jamo::Vowel { jung_index: 2 }),  // ㅑ
        'O' => Some(Jamo::Vowel { jung_index: 3 }),  // ㅒ
        'j' => Some(Jamo::Vowel { jung_index: 4 }),  // ㅓ
        'p' => Some(Jamo::Vowel { jung_index: 5 }),  // ㅔ
        'u' => Some(Jamo::Vowel { jung_index: 6 }),  // ㅕ
        'P' => Some(Jamo::Vowel { jung_index: 7 }),  // ㅖ
        'h' => Some(Jamo::Vowel { jung_index: 8 }),  // ㅗ
        'y' => Some(Jamo::Vowel { jung_index: 12 }), // ㅛ
        'n' => Some(Jamo::Vowel { jung_index: 13 }), // ㅜ
        'b' => Some(Jamo::Vowel { jung_index: 17 }), // ㅠ
        'm' => Some(Jamo::Vowel { jung_index: 18 }), // ㅡ
        'l' => Some(Jamo::Vowel { jung_index: 20 }), // ㅣ

        _ => None,
    }
}

/// 영문 키가 자음인지 확인
pub fn is_consonant(c: char) -> bool {
    matches!(map_to_jamo(c), Some(Jamo::Consonant { .. }))
}

/// 영문 키가 모음인지 확인
pub fn is_vowel(c: char) -> bool {
    matches!(map_to_jamo(c), Some(Jamo::Vowel { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consonant_mapping() {
        // 기본 자음
        assert!(matches!(
            map_to_jamo('r'),
            Some(Jamo::Consonant {
                cho_index: 0,
                jong_index: Some(1)
            })
        ));
        assert!(matches!(
            map_to_jamo('s'),
            Some(Jamo::Consonant {
                cho_index: 2,
                jong_index: Some(4)
            })
        ));
        assert!(matches!(
            map_to_jamo('g'),
            Some(Jamo::Consonant {
                cho_index: 18,
                jong_index: Some(27)
            })
        ));

        // 쌍자음
        assert!(matches!(
            map_to_jamo('R'),
            Some(Jamo::Consonant {
                cho_index: 1,
                jong_index: Some(2)
            })
        ));
        assert!(matches!(
            map_to_jamo('T'),
            Some(Jamo::Consonant {
                cho_index: 10,
                jong_index: Some(20)
            })
        ));

        // 종성 불가 쌍자음
        assert!(matches!(
            map_to_jamo('E'),
            Some(Jamo::Consonant {
                cho_index: 4,
                jong_index: None
            })
        ));
        assert!(matches!(
            map_to_jamo('Q'),
            Some(Jamo::Consonant {
                cho_index: 8,
                jong_index: None
            })
        ));
        assert!(matches!(
            map_to_jamo('W'),
            Some(Jamo::Consonant {
                cho_index: 13,
                jong_index: None
            })
        ));
    }

    #[test]
    fn test_vowel_mapping() {
        assert!(matches!(
            map_to_jamo('k'),
            Some(Jamo::Vowel { jung_index: 0 })
        )); // ㅏ
        assert!(matches!(
            map_to_jamo('h'),
            Some(Jamo::Vowel { jung_index: 8 })
        )); // ㅗ
        assert!(matches!(
            map_to_jamo('l'),
            Some(Jamo::Vowel { jung_index: 20 })
        )); // ㅣ
    }

    #[test]
    fn test_unmapped_characters() {
        assert!(map_to_jamo('1').is_none());
        assert!(map_to_jamo('!').is_none());
        assert!(map_to_jamo(' ').is_none());
        assert!(map_to_jamo('X').is_none()); // 대문자 X는 매핑 없음
    }

    #[test]
    fn test_is_consonant() {
        assert!(is_consonant('r'));
        assert!(is_consonant('R'));
        assert!(is_consonant('s'));
        assert!(!is_consonant('k'));
        assert!(!is_consonant('1'));
    }

    #[test]
    fn test_is_vowel() {
        assert!(is_vowel('k'));
        assert!(is_vowel('h'));
        assert!(is_vowel('l'));
        assert!(!is_vowel('r'));
        assert!(!is_vowel('1'));
    }

    #[test]
    fn test_jamo_methods() {
        let consonant = map_to_jamo('r').unwrap();
        assert_eq!(consonant.choseong_index(), Some(0));
        assert_eq!(consonant.jongseong_index(), Some(1));
        assert_eq!(consonant.jungseong_index(), None);
        assert!(consonant.is_consonant());
        assert!(!consonant.is_vowel());

        let vowel = map_to_jamo('k').unwrap();
        assert_eq!(vowel.jungseong_index(), Some(0));
        assert_eq!(vowel.choseong_index(), None);
        assert_eq!(vowel.jongseong_index(), None);
        assert!(vowel.is_vowel());
        assert!(!vowel.is_consonant());
    }
}
