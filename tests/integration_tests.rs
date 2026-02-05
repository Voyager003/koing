//! 통합 테스트 - Phase 1 핵심 변환 로직

use koing::convert;

#[test]
fn test_basic_jamo_composition() {
    assert_eq!(convert("rkskek"), "가나다");
    assert_eq!(convert("dkssudgktpdy"), "안녕하세요");
}

#[test]
fn test_jongseong_handling() {
    assert_eq!(convert("gksrmf"), "한글");
    assert_eq!(convert("dkswl"), "안지"); // ㄴ+ㅈ -> 종성ㄴ + 초성ㅈ
}

#[test]
fn test_complex_vowel() {
    assert_eq!(convert("dhksfy"), "완료");
}

#[test]
fn test_complex_jongseong() {
    assert_eq!(convert("dlfr"), "읽"); // ㄹㄱ 복합종성
}

#[test]
fn test_double_consonant() {
    assert_eq!(convert("Tks"), "싼"); // ㅆ
    assert_eq!(convert("Rk"), "까"); // ㄲ
}

#[test]
fn test_mixed_input() {
    assert_eq!(convert("123rksk"), "123가나"); // 숫자는 그대로
    assert_eq!(convert("rk!sk"), "가!나"); // 특수문자에서 끊김
}

#[test]
fn test_empty_string() {
    assert_eq!(convert(""), "");
}

#[test]
fn test_jongseong_to_next_choseong() {
    // 종성 -> 다음 초성 분리
    assert_eq!(convert("rkrkrl"), "가가기"); // ㄱ이 종성->초성으로
}

#[test]
fn test_consonant_only() {
    assert_eq!(convert("r"), "ㄱ");
    assert_eq!(convert("rs"), "ㄱㄴ");
    assert_eq!(convert("rsg"), "ㄱㄴㅎ");
}

#[test]
fn test_vowel_only() {
    assert_eq!(convert("k"), "ㅏ");
    assert_eq!(convert("kh"), "ㅏㅗ");
}

#[test]
fn test_unmapped_english() {
    // 매핑되지 않는 영문자(X, Y 등 일부)는 그대로 출력
    assert_eq!(convert("X"), "X");
    assert_eq!(convert("rkXsk"), "가X나");
}

#[test]
fn test_space_handling() {
    assert_eq!(convert("rk sk"), "가 나");
    assert_eq!(convert("gksrmf thtm"), "한글 소스"); // 소스 = thtm (ㅅㅗㅅㅡ)
}

#[test]
fn test_various_words() {
    assert_eq!(convert("zjavbxj"), "컴퓨터"); // 컴퓨터 = zjavbxj (ㅋㅓㅁㅍㅠㅌㅓ)
    assert_eq!(convert("vmfhrmfoa"), "프로그램"); // 프로그램 = vmfhrmfoa (ㅍㅡㄹㅗㄱㅡㄹㅐㅁ)
}
