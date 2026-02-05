//! 한글/영어 패턴 데이터
//!
//! 두벌식 자판에서 영어에서는 드문 조합이지만 한글로는 자연스러운 패턴들을 정의합니다.

use std::collections::HashSet;

lazy_static::lazy_static! {
    /// 한글 가능성이 높은 바이그램 패턴
    /// 두벌식에서 자음+모음 조합 (예: rk = ㄱ+ㅏ)
    pub static ref HANGUL_BIGRAMS: HashSet<&'static str> = {
        let mut set = HashSet::new();

        // 자음 키 (영문) + ㅏ (k)
        for c in ["rk", "sk", "ek", "fk", "ak", "qk", "tk", "dk", "wk", "ck", "zk", "xk", "vk", "gk"] {
            set.insert(c);
        }
        // 자음 키 + ㅓ (j)
        for c in ["rj", "sj", "ej", "fj", "aj", "qj", "tj", "dj", "wj", "cj", "zj", "xj", "vj", "gj"] {
            set.insert(c);
        }
        // 자음 키 + ㅗ (h)
        for c in ["rh", "sh", "eh", "fh", "ah", "qh", "th", "dh", "wh", "ch", "zh", "xh", "vh", "gh"] {
            set.insert(c);
        }
        // 자음 키 + ㅜ (n)
        for c in ["rn", "sn", "en", "fn", "an", "qn", "tn", "dn", "wn", "cn", "zn", "xn", "vn", "gn"] {
            set.insert(c);
        }
        // 자음 키 + ㅡ (m)
        for c in ["rm", "sm", "em", "fm", "am", "qm", "tm", "dm", "wm", "cm", "zm", "xm", "vm", "gm"] {
            set.insert(c);
        }
        // 자음 키 + ㅣ (l)
        for c in ["rl", "sl", "el", "fl", "al", "ql", "tl", "dl", "wl", "cl", "zl", "xl", "vl", "gl"] {
            set.insert(c);
        }
        // 자음 키 + ㅑ (i)
        for c in ["ri", "si", "ei", "fi", "ai", "qi", "ti", "di", "wi", "ci", "zi", "xi", "vi", "gi"] {
            set.insert(c);
        }
        // 자음 키 + ㅕ (u)
        for c in ["ru", "su", "eu", "fu", "au", "qu", "tu", "du", "wu", "cu", "zu", "xu", "vu", "gu"] {
            set.insert(c);
        }
        // 자음 키 + ㅛ (y)
        for c in ["ry", "sy", "ey", "fy", "ay", "qy", "ty", "dy", "wy", "cy", "zy", "xy", "vy", "gy"] {
            set.insert(c);
        }
        // 자음 키 + ㅠ (b)
        for c in ["rb", "sb", "eb", "fb", "ab", "qb", "tb", "db", "wb", "cb", "zb", "xb", "vb", "gb"] {
            set.insert(c);
        }

        set
    };

    /// 영어에서 매우 흔한 바이그램 - 한글 가능성 낮음
    pub static ref ENGLISH_BIGRAMS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        for b in [
            "th", "he", "in", "er", "an", "re", "on", "at", "en", "nd",
            "ti", "es", "or", "te", "of", "ed", "is", "it", "al", "ar",
            "st", "to", "nt", "ng", "se", "ha", "as", "ou", "io", "le",
            "ve", "co", "me", "de", "hi", "ri", "ro", "ic", "ne", "ea",
            "ra", "ce", "li", "ch", "ll", "be", "ma", "si", "om", "ur",
        ] {
            set.insert(b);
        }
        set
    };

    /// 흔한 한글 단어의 두벌식 패턴
    #[allow(dead_code)]
    pub static ref COMMON_KOREAN_PATTERNS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        for p in [
            // 인사말
            "dkssud",      // 안녕
            "dkssudgktpdy",// 안녕하세요
            "rkatkgkqslek",// 감사합니다

            // 자주 쓰는 단어
            "gks",   // 한
            "rmf",   // 글
            "gksrmf",// 한글
            "ek",    // 다
            "sk",    // 나
            "rk",    // 가
            "tlek",  // 시다
            "qslek", // 합니다
            "dlTek", // 있다
            "djqt",  // 없
            "djqtek",// 없다

            // 조사
            "dl",  // 이
            "eul", // 를
            "rl",  // 을
            "gn",  // ??? (확인 필요)
        ] {
            set.insert(p);
        }
        set
    };
}

/// 두벌식 자음 키 목록
pub const CONSONANT_KEYS: &[char] = &[
    'r', 'R', // ㄱ, ㄲ
    's',      // ㄴ
    'e', 'E', // ㄷ, ㄸ
    'f',      // ㄹ
    'a',      // ㅁ
    'q', 'Q', // ㅂ, ㅃ
    't', 'T', // ㅅ, ㅆ
    'd',      // ㅇ
    'w', 'W', // ㅈ, ㅉ
    'c',      // ㅊ
    'z',      // ㅋ
    'x',      // ㅌ
    'v',      // ㅍ
    'g',      // ㅎ
];

/// 두벌식 모음 키 목록
pub const VOWEL_KEYS: &[char] = &[
    'k',      // ㅏ
    'o',      // ㅐ
    'i',      // ㅑ
    'O',      // ㅒ
    'j',      // ㅓ
    'p',      // ㅔ
    'u',      // ㅕ
    'P',      // ㅖ
    'h',      // ㅗ
    'y',      // ㅛ
    'n',      // ㅜ
    'b',      // ㅠ
    'm',      // ㅡ
    'l',      // ㅣ
];

/// 문자가 두벌식 자음 키인지 확인
pub fn is_consonant_key(c: char) -> bool {
    CONSONANT_KEYS.contains(&c)
}

/// 문자가 두벌식 모음 키인지 확인
pub fn is_vowel_key(c: char) -> bool {
    VOWEL_KEYS.contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hangul_bigrams() {
        assert!(HANGUL_BIGRAMS.contains("rk")); // ㄱㅏ
        assert!(HANGUL_BIGRAMS.contains("gk")); // ㅎㅏ
        // "th"는 영어에서 흔하지만 두벌식에서도 ㅅ+ㅗ로 유효함
        // 따라서 HANGUL_BIGRAMS에 포함될 수 있음
        assert!(HANGUL_BIGRAMS.contains("th")); // ㅅ+ㅗ
    }

    #[test]
    fn test_english_bigrams() {
        assert!(ENGLISH_BIGRAMS.contains("th"));
        assert!(ENGLISH_BIGRAMS.contains("er"));
        assert!(!ENGLISH_BIGRAMS.contains("rk"));
    }

    #[test]
    fn test_consonant_vowel_keys() {
        assert!(is_consonant_key('r'));
        assert!(is_consonant_key('R'));
        assert!(is_vowel_key('k'));
        assert!(is_vowel_key('l'));
        assert!(!is_consonant_key('k'));
        assert!(!is_vowel_key('r'));
    }
}
