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

/// 한글 텍스트의 음절 구조 자연스러움 검사
///
/// 연속 희귀 음절 >= 2 또는 희귀 비율 >= 0.5 이면 false (비자연스러움)
pub fn check_syllable_structure(text: &str) -> bool {
    let mut consecutive_rare = 0;
    let mut total_syllables = 0;
    let mut rare_count = 0;

    for ch in text.chars() {
        if let Some((cho, jung, _)) = decompose_syllable(ch) {
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
        } else {
            // 낱자모 또는 비한글: consecutive 리셋
            consecutive_rare = 0;
        }
    }

    if total_syllables > 0 && (rare_count as f64 / total_syllables as f64) >= 0.5 {
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
}
