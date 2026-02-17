//! 한글/영어 패턴 데이터
//!
//! 두벌식 자판에서 영어에서는 드문 조합이지만 한글로는 자연스러운 패턴들을 정의합니다.

use std::collections::HashSet;
use std::sync::LazyLock;

/// 흔한 영어 단어 목록 - 이 단어들은 자동 변환에서 제외
pub static COMMON_ENGLISH_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
        let mut set = HashSet::new();
        // 3글자 영어 단어
        for w in ["the", "and", "for", "are", "but", "not", "you", "all",
                  "can", "had", "her", "was", "one", "our", "out", "has",
                  "his", "how", "its", "let", "may", "new", "now", "old",
                  "see", "two", "way", "who", "did", "get", "com", "use",
                  "man", "day", "too", "any", "put", "say", "she", "own",
                  "try", "set", "run", "end", "act", "ask", "big", "buy",
                  "cut", "far", "few", "got", "him", "hot", "job", "key",
                  "low", "lot", "off", "pay", "per", "red", "sit", "six",
                  "ten", "top", "war", "why", "yes", "yet", "ago", "air",
                  "art", "bad", "bed", "bit", "box", "car", "cup", "dog",
                  "eat", "eye", "fly", "fun", "god", "gun", "hit", "law",
                  "lie", "map", "oil", "old", "add", "age", "arm", "bag",
                  "bar", "bus", "cry", "die", "dry", "ear", "egg", "fan",
                  "fat", "fit", "gas", "hat", "ice", "kid", "lap", "leg",
                  "lip", "mix", "net", "nor", "nut", "odd", "pan", "pet",
                  "pie", "pin", "pop", "pot", "raw", "row", "sad", "sea",
                  "sky", "son", "sun", "tax", "tea", "tie", "tip", "via",
                  "wet", "win", "won", "app", "web", "api", "url", "dev"] {
            set.insert(w);
        }
        // 4글자 영어 단어
        for w in ["that", "with", "have", "this", "will", "your", "from",
                  "they", "been", "call", "come", "made", "find", "long",
                  "down", "side", "more", "each", "said", "time", "very",
                  "when", "make", "only", "here", "must", "into", "year",
                  "take", "them", "some", "then", "than", "look", "also",
                  "well", "back", "over", "such", "good", "give", "most",
                  "just", "even", "work", "know", "life", "hand", "part",
                  "code", "file", "test", "data", "user", "type", "name", "wifi",
                  "list", "help", "want", "need", "open", "save", "edit",
                  "view", "show", "hide", "read", "send", "copy", "move",
                  "text", "link", "next", "home", "page", "form", "true",
                  "null", "void", "self", "func", "main", "init", "free",
                  "size", "loop", "bool", "byte", "char", "case", "else",
                  "enum", "goto", "left", "last", "none", "pass", "push",
                  "pull", "sort", "stop", "wait", "wrap", "exit", "like"] {
            set.insert(w);
        }
        // 5글자+ 영어 단어
        for w in ["hello", "world", "there", "which", "their", "would", "about",
                  "these", "could", "other", "after", "first", "never", "where",
                  "those", "being", "every", "under", "think", "still", "while",
                  "found", "great", "right", "three", "place", "thing", "point",
                  "string", "function", "return", "public", "private", "static",
                  "class", "const", "import", "export", "default", "async", "await",
                  "break", "catch", "throw", "final", "super", "print", "input",
                  "output", "error", "value", "array", "index", "count", "start",
                  "false", "begin", "check", "clear", "close", "build", "write",
                  "event", "state", "props", "style", "click", "focus", "fetch"] {
            set.insert(w);
        }
        set
});

/// 한글 가능성이 높은 바이그램 패턴
/// 두벌식에서 자음+모음 조합 (예: rk = ㄱ+ㅏ)
pub static HANGUL_BIGRAMS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
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
});

/// 영어에서 매우 흔한 바이그램 - 한글 가능성 낮음
pub static ENGLISH_BIGRAMS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
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
});

/// 문자가 두벌식 자음 키인지 확인
pub fn is_consonant_key(c: char) -> bool {
    crate::core::jamo_mapper::is_consonant(c)
}

/// 문자가 두벌식 모음 키인지 확인
pub fn is_vowel_key(c: char) -> bool {
    crate::core::jamo_mapper::is_vowel(c)
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
