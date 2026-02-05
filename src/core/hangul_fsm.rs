//! 한글 조합 유한 상태 기계 (FSM)

use crate::core::jamo_mapper::Jamo;
use crate::core::unicode::{
    choseong_to_jamo_char, combine_jongseong, combine_jungseong, compose_syllable,
    jongseong_to_choseong, jungseong_to_jamo_char, split_jongseong,
};

/// FSM 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// 아무것도 없음
    Empty,
    /// 초성만 입력됨
    Choseong,
    /// 초성+중성 (한 글자 조합 중)
    ChoseongJungseong,
    /// 초성+중성+종성 (한 글자 조합 중)
    ChoseongJungseongJongseong,
}

/// 한글 조합 FSM
pub struct HangulFsm {
    state: State,
    /// 현재 초성 인덱스
    choseong: u32,
    /// 현재 중성 인덱스
    jungseong: u32,
    /// 현재 종성 인덱스 (0 = 없음)
    jongseong: u32,
    /// 출력 버퍼
    output: String,
}

impl HangulFsm {
    /// 새 FSM 생성
    pub fn new() -> Self {
        Self {
            state: State::Empty,
            choseong: 0,
            jungseong: 0,
            jongseong: 0,
            output: String::new(),
        }
    }

    /// 자모를 입력하여 상태 전이
    pub fn feed(&mut self, jamo: Jamo) {
        match jamo {
            Jamo::Consonant {
                cho_index,
                jong_index,
            } => {
                self.feed_consonant(cho_index, jong_index);
            }
            Jamo::Vowel { jung_index } => {
                self.feed_vowel(jung_index);
            }
        }
    }

    /// 자음 입력 처리
    fn feed_consonant(&mut self, cho_index: u32, jong_index: Option<u32>) {
        match self.state {
            State::Empty => {
                // 초성으로 저장
                self.choseong = cho_index;
                self.state = State::Choseong;
            }
            State::Choseong => {
                // 기존 초성을 단독 자모로 출력하고, 새 초성으로 교체
                if let Some(c) = choseong_to_jamo_char(self.choseong) {
                    self.output.push(c);
                }
                self.choseong = cho_index;
                // state는 Choseong 유지
            }
            State::ChoseongJungseong => {
                // 종성으로 추가 시도
                if let Some(jong) = jong_index {
                    self.jongseong = jong;
                    self.state = State::ChoseongJungseongJongseong;
                } else {
                    // 종성 불가 자음 (ㄸ, ㅃ, ㅉ)
                    // 현재 글자 확정 후 새 초성으로
                    self.flush_current();
                    self.choseong = cho_index;
                    self.state = State::Choseong;
                }
            }
            State::ChoseongJungseongJongseong => {
                // 복합 종성 조합 시도
                if let Some(jong) = jong_index {
                    if let Some(combined) = combine_jongseong(self.jongseong, jong) {
                        self.jongseong = combined;
                        // state 유지
                    } else {
                        // 복합 종성 불가 -> 현재 글자 확정, 새 초성
                        self.flush_current();
                        self.choseong = cho_index;
                        self.state = State::Choseong;
                    }
                } else {
                    // 종성 불가 자음 -> 현재 글자 확정, 새 초성
                    self.flush_current();
                    self.choseong = cho_index;
                    self.state = State::Choseong;
                }
            }
        }
    }

    /// 모음 입력 처리
    fn feed_vowel(&mut self, jung_index: u32) {
        match self.state {
            State::Empty => {
                // 모음만 단독 출력
                if let Some(c) = jungseong_to_jamo_char(jung_index) {
                    self.output.push(c);
                }
                // state는 Empty 유지
            }
            State::Choseong => {
                // 초성 + 중성 조합
                self.jungseong = jung_index;
                self.state = State::ChoseongJungseong;
            }
            State::ChoseongJungseong => {
                // 복합 모음 조합 시도
                if let Some(combined) = combine_jungseong(self.jungseong, jung_index) {
                    self.jungseong = combined;
                    // state 유지
                } else {
                    // 복합 모음 불가 -> 현재 글자 확정 후 모음만 출력
                    self.flush_current();
                    if let Some(c) = jungseong_to_jamo_char(jung_index) {
                        self.output.push(c);
                    }
                    self.state = State::Empty;
                }
            }
            State::ChoseongJungseongJongseong => {
                // 종성을 다음 초성으로 분리
                // 복합 종성이면 마지막 자음만 분리, 단일 종성이면 전체 분리
                if let Some((remaining_jong, next_cho)) = split_jongseong(self.jongseong) {
                    // 복합 종성: 첫 자음은 종성으로 남기고, 둘째 자음은 다음 초성
                    self.jongseong = remaining_jong;
                    self.flush_current();
                    self.choseong = next_cho;
                    self.jungseong = jung_index;
                    self.state = State::ChoseongJungseong;
                } else {
                    // 단일 종성: 전체를 다음 초성으로
                    if let Some(next_cho) = jongseong_to_choseong(self.jongseong) {
                        self.jongseong = 0;
                        self.flush_current();
                        self.choseong = next_cho;
                        self.jungseong = jung_index;
                        self.state = State::ChoseongJungseong;
                    } else {
                        // 변환 불가 (이론상 발생하지 않음)
                        self.flush_current();
                        if let Some(c) = jungseong_to_jamo_char(jung_index) {
                            self.output.push(c);
                        }
                        self.state = State::Empty;
                    }
                }
            }
        }
    }

    /// 현재 조합 중인 글자를 출력 버퍼에 추가
    fn flush_current(&mut self) {
        match self.state {
            State::Empty => {}
            State::Choseong => {
                if let Some(c) = choseong_to_jamo_char(self.choseong) {
                    self.output.push(c);
                }
            }
            State::ChoseongJungseong => {
                if let Some(c) = compose_syllable(self.choseong, self.jungseong, 0) {
                    self.output.push(c);
                }
            }
            State::ChoseongJungseongJongseong => {
                if let Some(c) = compose_syllable(self.choseong, self.jungseong, self.jongseong) {
                    self.output.push(c);
                }
            }
        }
        self.reset_state();
    }

    /// 상태 초기화
    fn reset_state(&mut self) {
        self.state = State::Empty;
        self.choseong = 0;
        self.jungseong = 0;
        self.jongseong = 0;
    }

    /// 변환 불가 문자 처리 (숫자, 특수문자 등)
    pub fn feed_passthrough(&mut self, c: char) {
        self.flush_current();
        self.output.push(c);
    }

    /// FSM 종료 및 최종 결과 반환
    pub fn finish(mut self) -> String {
        self.flush_current();
        self.output
    }
}

impl Default for HangulFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::jamo_mapper::map_to_jamo;

    fn convert(input: &str) -> String {
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

    #[test]
    fn test_basic_syllable() {
        assert_eq!(convert("rk"), "가"); // ㄱ + ㅏ
        assert_eq!(convert("sk"), "나"); // ㄴ + ㅏ
        assert_eq!(convert("ek"), "다"); // ㄷ + ㅏ
    }

    #[test]
    fn test_with_jongseong() {
        assert_eq!(convert("rkr"), "각"); // ㄱ + ㅏ + ㄱ
        assert_eq!(convert("rks"), "간"); // ㄱ + ㅏ + ㄴ
        assert_eq!(convert("gks"), "한"); // ㅎ + ㅏ + ㄴ
    }

    #[test]
    fn test_jongseong_to_next_choseong() {
        assert_eq!(convert("rksk"), "가나"); // 각 -> ㄱ이 다음 초성으로
        assert_eq!(convert("dkswl"), "안지"); // ㄴ + ㅈ -> 안 + 지
    }

    #[test]
    fn test_complex_jungseong() {
        assert_eq!(convert("dhk"), "와"); // ㅗ + ㅏ = ㅘ -> 완전한 '와'
        assert_eq!(convert("dnj"), "워"); // ㅜ + ㅓ = ㅝ
        assert_eq!(convert("dml"), "의"); // ㅡ + ㅣ = ㅢ
    }

    #[test]
    fn test_complex_jongseong() {
        // d=ㅇ(초성11), k=ㅏ(중성0), f=ㄹ(종성8), r=ㄱ(종성1)
        // ㄹ(8) + ㄱ(1) = ㄺ(9) 복합종성 -> 앍
        assert_eq!(convert("dkfr"), "앍");
    }

    #[test]
    fn test_double_consonant() {
        assert_eq!(convert("Rk"), "까"); // ㄲ + ㅏ
        assert_eq!(convert("Tks"), "싼"); // ㅆ + ㅏ + ㄴ
    }

    #[test]
    fn test_passthrough() {
        assert_eq!(convert("123"), "123");
        assert_eq!(convert("rk!sk"), "가!나");
        assert_eq!(convert("rk sk"), "가 나");
    }

    #[test]
    fn test_consonant_only() {
        assert_eq!(convert("r"), "ㄱ");
        assert_eq!(convert("rs"), "ㄱㄴ");
    }

    #[test]
    fn test_vowel_only() {
        assert_eq!(convert("k"), "ㅏ");
        assert_eq!(convert("kh"), "ㅏㅗ");
    }

    #[test]
    fn test_empty() {
        assert_eq!(convert(""), "");
    }
}
