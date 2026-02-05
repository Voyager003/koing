//! 입력 소스(한/영) 전환 기능
//! Carbon API의 TIS (Text Input Source) 함수 사용

use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::ptr;

// Carbon TIS 타입 정의
type TISInputSourceRef = *mut std::ffi::c_void;
type CFIndex = isize;

// Carbon 프레임워크 링크
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    fn TISSelectInputSource(inputSource: TISInputSourceRef) -> i32;
    fn TISCreateInputSourceList(
        properties: core_foundation::dictionary::CFDictionaryRef,
        includeAllInstalled: bool,
    ) -> CFArrayRef;
    fn TISGetInputSourceProperty(
        inputSource: TISInputSourceRef,
        propertyKey: CFStringRef,
    ) -> CFTypeRef;

    // 상수 키 (런타임에 가져와야 함)
    static kTISPropertyInputSourceID: CFStringRef;
}

// Core Foundation 배열 함수
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const std::ffi::c_void;
}

/// 한글 입력 소스 ID (macOS 기본 한글)
const KOREAN_INPUT_SOURCE_ID: &str = "com.apple.inputmethod.Korean.2SetKorean";
/// 영문 입력 소스 ID (macOS ABC)
const ENGLISH_INPUT_SOURCE_ID: &str = "com.apple.keylayout.ABC";
/// 영문 입력 소스 ID (US)
const ENGLISH_US_INPUT_SOURCE_ID: &str = "com.apple.keylayout.US";

/// 현재 입력 소스 ID 가져오기
pub fn get_current_input_source_id() -> Option<String> {
    unsafe {
        let current = TISCopyCurrentKeyboardInputSource();
        if current.is_null() {
            return None;
        }

        let source_id = TISGetInputSourceProperty(current, kTISPropertyInputSourceID);
        CFRelease(current as CFTypeRef);

        if source_id.is_null() {
            return None;
        }

        let cf_string = CFString::wrap_under_get_rule(source_id as CFStringRef);
        Some(cf_string.to_string())
    }
}

/// 현재 영문 입력 소스인지 확인
pub fn is_english_input_source() -> bool {
    if let Some(id) = get_current_input_source_id() {
        // 한글 입력 소스가 아니면 영문으로 간주
        !id.contains("Korean")
    } else {
        // 알 수 없으면 영문으로 가정
        true
    }
}

/// 특정 입력 소스 ID로 전환
fn switch_to_input_source(target_id: &str) -> Result<(), String> {
    unsafe {
        // 모든 키보드 입력 소스 목록 가져오기
        let source_list = TISCreateInputSourceList(ptr::null(), true);
        if source_list.is_null() {
            return Err("입력 소스 목록을 가져올 수 없습니다".to_string());
        }

        let count = CFArrayGetCount(source_list);

        for i in 0..count {
            let source_ptr = CFArrayGetValueAtIndex(source_list, i) as TISInputSourceRef;
            if source_ptr.is_null() {
                continue;
            }

            let source_id_ref = TISGetInputSourceProperty(source_ptr, kTISPropertyInputSourceID);
            if source_id_ref.is_null() {
                continue;
            }

            let source_id = CFString::wrap_under_get_rule(source_id_ref as CFStringRef);
            let source_id_str = source_id.to_string();

            if source_id_str == target_id {
                let result = TISSelectInputSource(source_ptr);
                CFRelease(source_list as CFTypeRef);
                if result == 0 {
                    log::info!("입력 소스 전환 성공: {}", target_id);
                    return Ok(());
                } else {
                    return Err(format!("TISSelectInputSource 실패: 오류 코드 {}", result));
                }
            }
        }

        CFRelease(source_list as CFTypeRef);
        Err(format!("입력 소스를 찾을 수 없습니다: {}", target_id))
    }
}

/// 한글 입력 소스로 전환
pub fn switch_to_korean() -> Result<(), String> {
    // 이미 한글이면 전환 불필요
    if let Some(id) = get_current_input_source_id() {
        if id.contains("Korean") {
            log::debug!("이미 한글 입력 소스입니다");
            return Ok(());
        }
    }

    switch_to_input_source(KOREAN_INPUT_SOURCE_ID)
}

/// 영문 입력 소스로 전환
pub fn switch_to_english() -> Result<(), String> {
    // 이미 영문이면 전환 불필요
    if is_english_input_source() {
        log::debug!("이미 영문 입력 소스입니다");
        return Ok(());
    }

    // ABC 먼저 시도, 실패하면 US 시도
    switch_to_input_source(ENGLISH_INPUT_SOURCE_ID)
        .or_else(|_| switch_to_input_source(ENGLISH_US_INPUT_SOURCE_ID))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // GUI 환경에서만 테스트 가능
    fn test_get_current_input_source() {
        let id = get_current_input_source_id();
        assert!(id.is_some());
        println!("현재 입력 소스: {:?}", id);
    }

    #[test]
    #[ignore] // GUI 환경에서만 테스트 가능
    fn test_is_english_input_source() {
        let is_english = is_english_input_source();
        println!("영문 입력 소스 여부: {}", is_english);
    }
}
