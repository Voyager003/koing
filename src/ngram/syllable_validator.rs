//! 한글 음절 구조 자연스러움 검사
//!
//! 영문 → 한글 변환 결과가 실제 한국어에서 자연스러운 음절 구조인지 판별합니다.
//! 초성+중성 조합이 극히 희귀한 경우를 걸러냅니다.

use crate::core::unicode::decompose_syllable;

/// 초성+중성 조합이 한국어에서 극히 희귀한지 판별
///
/// 초성 idx: 0=ㄱ 1=ㄲ 2=ㄴ 3=ㄷ 4=ㄸ 5=ㄹ 6=ㅁ 7=ㅂ 8=ㅃ 9=ㅅ 10=ㅆ
///           11=ㅇ 12=ㅈ 13=ㅉ 14=ㅊ 15=ㅋ 16=ㅌ 17=ㅍ 18=ㅎ
/// 중성 idx: 0=ㅏ 1=ㅐ 2=ㅑ 3=ㅒ 4=ㅓ 5=ㅔ 6=ㅕ 7=ㅖ 8=ㅗ 9=ㅘ 10=ㅙ
///           11=ㅚ 12=ㅛ 13=ㅜ 14=ㅝ 15=ㅞ 16=ㅟ 17=ㅠ 18=ㅡ 19=ㅢ 20=ㅣ
fn is_rare_onset(cho: u32, jung: u32) -> bool {
    // Rule 1: ㅒ(3) — 걔(0), 얘(11), 쟤(12) 제외하고 대부분 희귀
    if jung == 3 && !matches!(cho, 0 | 11 | 12) {
        return true;
    }

    // Rule 2: ㅙ(10) — 왜(11) 제외, 대부분 희귀
    if jung == 10 && cho != 11 {
        return true;
    }

    // Rule 3: ㅞ(15) — 웨(11) 제외, 대부분 희귀
    if jung == 15 && cho != 11 {
        return true;
    }

    // Rule 4: 쌍자음(ㄲ1,ㄸ4,ㅃ8,ㅉ13) + y계열(ㅑ2,ㅖ7,ㅛ12,ㅠ17) 및 ㅢ(19)
    if matches!(cho, 1 | 4 | 8 | 13) && matches!(jung, 2 | 7 | 12 | 17 | 19) {
        return true;
    }

    // Rule 5: ㅑ(2) — 허용: 갸(0), 냐(2), 샤(9), 야(11) 만 자연스러움
    // 쟈(12), 랴(5), 먀(6), 뱌(7), 챠(14), 캬(15), 탸(16), 퍄(17) 등 차단
    if jung == 2 && !matches!(cho, 0 | 2 | 9 | 11) {
        return true;
    }

    false
}

/// 종성→초성 전이가 한국어에서 극히 드문지 판별
///
/// 종성 idx: 0=없음 1=ㄱ 2=ㄲ 3=ㄳ 4=ㄴ 5=ㄵ 6=ㄶ 7=ㄷ 8=ㄹ 9=ㄺ 10=ㄻ
///           11=ㄼ 12=ㄽ 13=ㄾ 14=ㄿ 15=ㅀ 16=ㅁ 17=ㅂ 18=ㅄ 19=ㅅ 20=ㅆ
///           21=ㅇ 22=ㅈ 23=ㅊ 24=ㅋ 25=ㅌ 26=ㅍ 27=ㅎ
/// 초성 idx: 0=ㄱ 1=ㄲ 2=ㄴ 3=ㄷ 4=ㄸ 5=ㄹ 6=ㅁ 7=ㅂ 8=ㅃ 9=ㅅ 10=ㅆ
///           11=ㅇ 12=ㅈ 13=ㅉ 14=ㅊ 15=ㅋ 16=ㅌ 17=ㅍ 18=ㅎ
fn is_rare_transition(prev_jong: u32, next_cho: u32) -> bool {
    // 종성 없음 → 전이 무관
    if prev_jong == 0 {
        return false;
    }

    // 한국어에서 극히 드문 종성→초성 전이 패턴
    match (prev_jong, next_cho) {
        // ㅂ종성(17) → ㅍ초성(17): 극히 드묾
        (17, 17) => true,
        // ㄷ종성(7) → ㅌ초성(16): 극히 드묾
        (7, 16) => true,
        // ㄱ종성(1) → ㅋ초성(15): 극히 드묾
        (1, 15) => true,
        // ㅂ종성(17) → ㅃ초성(8): 극히 드묾
        (17, 8) => true,
        // ㄷ종성(7) → ㄸ초성(4): 극히 드묾
        (7, 4) => true,
        // ㅈ종성(22) → ㅉ초성(13): 극히 드묾
        (22, 13) => true,
        // ㅊ종성(23) → ㅊ초성(14): 극히 드묾
        (23, 14) => true,
        // ㅋ종성(24) → ㅋ초성(15): 극히 드묾
        (24, 15) => true,
        // ㅌ종성(25) → ㅌ초성(16): 극히 드묾
        (25, 16) => true,
        // ㅍ종성(26) → ㅍ초성(17): 극히 드묾
        (26, 17) => true,
        _ => false,
    }
}

/// 한글 텍스트의 음절 구조 자연스러움 검사
///
/// 연속 희귀 음절 >= 2 또는 희귀 비율 >= 0.5 이면 false (비자연스러움)
/// 추가: 연속 음절 간 종성→초성 전이 자연스러움 검사
pub fn check_syllable_structure(text: &str) -> bool {
    let mut consecutive_rare = 0;
    let mut total_syllables = 0;
    let mut rare_count = 0;
    let mut rare_transitions = 0;
    let mut prev_jongseong: Option<u32> = None;

    for ch in text.chars() {
        if let Some((cho, jung, jong)) = decompose_syllable(ch) {
            total_syllables += 1;
            if is_rare_onset(cho, jung) {
                rare_count += 1;
                consecutive_rare += 1;
                if consecutive_rare >= 2 {
                    return false;
                }
            } else {
                consecutive_rare = 0;
            }

            // 종성→초성 전이 검사
            if let Some(prev_jong) = prev_jongseong {
                if is_rare_transition(prev_jong, cho) {
                    rare_transitions += 1;
                }
            }
            prev_jongseong = Some(jong);
        } else {
            // 낱자모 또는 비한글: consecutive 리셋
            consecutive_rare = 0;
            prev_jongseong = None;
        }
    }

    if total_syllables > 0 && (rare_count as f64 / total_syllables as f64) >= 0.5 {
        return false;
    }

    // 희귀 전이가 2개 이상이면 비자연스러움
    if rare_transitions >= 2 {
        return false;
    }

    // 3음절 이하에서 희귀 전이 1개이고 희귀 onset도 있으면 거부
    if total_syllables <= 3 && rare_transitions >= 1 && rare_count >= 1 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_syllables_pass() {
        assert!(check_syllable_structure("안녕"));
        assert!(check_syllable_structure("한글"));
        assert!(check_syllable_structure("가나다"));
        assert!(check_syllable_structure("안녕하세요"));
    }

    #[test]
    fn test_rare_syllable_ratio() {
        // "ㅇ먀뇨" — 먀 is rare (ㅁ+ㅑ), 뇨 is common (ㄴ+ㅛ)
        // but 먀 alone: 1 rare out of 2 syllables (ㅇ is jamo, not syllable) = 0.5 → false
        // Actually "ㅇ먀뇨" has ㅇ(jamo), 먀(syllable, rare), 뇨(syllable, common)
        // rare_count=1, total=2, ratio=0.5 → false
        assert!(!check_syllable_structure("먀뇨")); // 1/2 = 0.5 → false
    }

    #[test]
    fn test_consecutive_rare() {
        // 퍄(rare) + 견(common) = only 1 consecutive rare, but ratio 1/2 = 0.5 → false
        assert!(!check_syllable_structure("퍄견"));
    }

    #[test]
    fn test_single_rare_in_long_text() {
        // 1 rare out of 3+ syllables should pass (ratio < 0.5)
        assert!(check_syllable_structure("먀나다")); // 1/3 = 0.33 → true
    }

    #[test]
    fn test_is_rare_onset_rules() {
        // Rule 1: ㅒ — 걔(cho=0) should NOT be rare
        assert!(!is_rare_onset(0, 3)); // 걔
        assert!(!is_rare_onset(11, 3)); // 얘
        assert!(is_rare_onset(6, 3)); // 먜 — rare

        // Rule 2: ㅙ — 왜(cho=11) should NOT be rare
        assert!(!is_rare_onset(11, 10)); // 왜
        assert!(is_rare_onset(0, 10)); // 괘 — rare

        // Rule 3: ㅞ — 웨(cho=11) should NOT be rare
        assert!(!is_rare_onset(11, 15)); // 웨
        assert!(is_rare_onset(0, 15)); // 궤... actually this is 괞? No — 궤 — rare by rule

        // Rule 4: 쌍자음 + y계열
        assert!(is_rare_onset(1, 2)); // ㄲ+ㅑ — rare
        assert!(is_rare_onset(4, 7)); // ㄸ+ㅖ — rare

        // Rule 5: ㅑ — 화이트리스트 (갸, 냐, 샤, 야만 허용)
        assert!(is_rare_onset(6, 2)); // 먀 — rare
        assert!(is_rare_onset(17, 2)); // 퍄 — rare
        assert!(is_rare_onset(12, 2)); // 쟈 — rare
        assert!(is_rare_onset(5, 2)); // 랴 — rare
        assert!(!is_rare_onset(0, 2)); // 갸 — not rare
        assert!(!is_rare_onset(2, 2)); // 냐 — not rare
        assert!(!is_rare_onset(9, 2)); // 샤 — not rare
        assert!(!is_rare_onset(11, 2)); // 야 — not rare
    }

    #[test]
    fn test_wifi_blocked() {
        // "wifi" → "쟈랴" — 쟈(ㅈ+ㅑ), 랴(ㄹ+ㅑ) 모두 희귀 → 연속 2개로 차단
        assert!(!check_syllable_structure("쟈랴"));
    }

    #[test]
    fn test_rare_transition() {
        // ㅂ종성→ㅍ초성: 극히 드묾
        assert!(is_rare_transition(17, 17));
        // ㄷ종성→ㅌ초성: 극히 드묾
        assert!(is_rare_transition(7, 16));
        // ㄱ종성→ㅋ초성: 극히 드묾
        assert!(is_rare_transition(1, 15));
        // 종성 없음 → 전이 무관
        assert!(!is_rare_transition(0, 17));
        // ㄴ종성→ㅇ초성: 자연스러운 전이
        assert!(!is_rare_transition(4, 11));
        // ㄹ종성→ㅇ초성: 자연스러운 전이
        assert!(!is_rare_transition(8, 11));
    }

    #[test]
    fn test_natural_transitions_pass() {
        // "한글" — ㄴ종성→ㄱ초성: 자연스러움
        assert!(check_syllable_structure("한글"));
        // "안녕" — ㄴ종성→ㄴ초성: 자연스러움
        assert!(check_syllable_structure("안녕"));
        // "가나다라" — 종성 없음, 전이 검사 스킵
        assert!(check_syllable_structure("가나다라"));
    }
}
