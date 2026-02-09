//! 입력 소스(한/영) 전환 기능
//! Carbon API의 TIS (Text Input Source) 함수 사용

use crate::platform::os_version::is_sonoma_or_later;
use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

/// 캐싱된 입력 소스 상태 (true = 영문)
static INPUT_SOURCE_IS_ENGLISH: AtomicBool = AtomicBool::new(true);
/// 캐시 유효 여부
static INPUT_SOURCE_CACHE_VALID: AtomicBool = AtomicBool::new(false);

/// 입력 소스 캐시 무효화
/// FlagsChanged, switch_to_korean(), switch_to_english() 완료 후 호출
pub fn invalidate_input_source_cache() {
    INPUT_SOURCE_CACHE_VALID.store(false, Ordering::Release);
}

// Carbon TIS 타입 정의
type TISInputSourceRef = *mut std::ffi::c_void;
type CFIndex = isize;

/// 캐싱된 한글 입력 소스 (CFRetain으로 소유권 유지)
static KOREAN_SOURCE_CACHE: OnceLock<usize> = OnceLock::new();

/// 캐싱된 영문 입력 소스 (CFRetain으로 소유권 유지)
static ENGLISH_SOURCE_CACHE: OnceLock<usize> = OnceLock::new();

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

/// ASCII case-insensitive substring 검사 (String 할당 없음)
fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() {
        return true;
    }
    haystack
        .as_bytes()
        .windows(needle_bytes.len())
        .any(|window| window.eq_ignore_ascii_case(needle_bytes))
}

/// 입력 소스 ID가 한글 입력기인지 확인
/// macOS 기본 한글 + 서드파티 입력기(구름, 한글 등) 포함
/// ASCII case-insensitive 비교로 String 할당 없이 검사
pub(crate) fn is_korean_input_source_id(id: &str) -> bool {
    contains_ascii_case_insensitive(id, "korean")
        || contains_ascii_case_insensitive(id, "hangul")
        || contains_ascii_case_insensitive(id, "gureum")
        || contains_ascii_case_insensitive(id, "hangeul")
}

/// 현재 영문 입력 소스인지 확인 (캐시 활용)
pub fn is_english_input_source() -> bool {
    // 캐시가 유효하면 atomic 읽기만으로 즉시 반환
    if INPUT_SOURCE_CACHE_VALID.load(Ordering::Acquire) {
        return INPUT_SOURCE_IS_ENGLISH.load(Ordering::Acquire);
    }

    // 캐시 미스 — TIS API 호출 후 캐시 갱신
    let is_english = if let Some(id) = get_current_input_source_id() {
        !is_korean_input_source_id(&id)
    } else {
        true
    };

    INPUT_SOURCE_IS_ENGLISH.store(is_english, Ordering::Release);
    INPUT_SOURCE_CACHE_VALID.store(true, Ordering::Release);
    is_english
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

/// 한글 입력 소스 참조를 캐싱 (최초 1회만 검색)
fn get_cached_korean_source() -> Option<TISInputSourceRef> {
    let ptr = *KOREAN_SOURCE_CACHE.get_or_init(|| {
        unsafe {
            let source_list = TISCreateInputSourceList(ptr::null(), true);
            if source_list.is_null() {
                return 0;
            }

            let count = CFArrayGetCount(source_list);
            let mut found: usize = 0;

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
                if source_id.to_string() == KOREAN_INPUT_SOURCE_ID {
                    // 의도적 CFRetain: 앱 수명 동안 캐싱하며 CFRelease하지 않음.
                    // ~64바이트 고정 할당으로 실질적 리크 영향 없음.
                    CFRetain(source_ptr as CFTypeRef);
                    found = source_ptr as usize;
                    break;
                }
            }

            CFRelease(source_list as CFTypeRef);
            found
        }
    });

    if ptr == 0 { None } else { Some(ptr as TISInputSourceRef) }
}

/// 입력 소스 전환 후 실제로 전환되었는지 검증 (Sonoma+ 대응)
/// Sonoma에서 TISSelectInputSource가 비동기로 동작할 수 있음
fn verify_switch(expected_check: impl Fn(&str) -> bool) -> bool {
    if !is_sonoma_or_later() {
        // Ventura 이하에서는 동기 전환이므로 검증 불필요
        return true;
    }

    const POLL_INTERVAL_MS: u64 = 5;
    const MAX_WAIT_MS: u64 = 50;
    let max_tries = MAX_WAIT_MS / POLL_INTERVAL_MS;

    for _ in 0..max_tries {
        if let Some(id) = get_current_input_source_id() {
            if expected_check(&id) {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    // 마지막 확인
    get_current_input_source_id()
        .map(|id| expected_check(&id))
        .unwrap_or(false)
}

/// 한글 입력 소스로 전환
pub fn switch_to_korean() -> Result<(), String> {
    // 이미 한글이면 전환 불필요
    if let Some(id) = get_current_input_source_id() {
        if is_korean_input_source_id(&id) {
            return Ok(());
        }
    }

    // 캐싱된 소스로 즉시 전환 (리스트 순회 없음)
    let result = if let Some(source) = get_cached_korean_source() {
        let ret = unsafe { TISSelectInputSource(source) };
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("TISSelectInputSource 실패: 오류 코드 {}", ret))
        }
    } else {
        // 캐시 실패 시 폴백
        switch_to_input_source(KOREAN_INPUT_SOURCE_ID)
    };

    // Sonoma+에서 전환 완료 검증
    if result.is_ok() && !verify_switch(|id| is_korean_input_source_id(id)) {
        log::warn!("한글 전환 검증 실패 — 전환이 지연되었을 수 있음");
    }

    // 전환 후 캐시 무효화
    invalidate_input_source_cache();

    result
}

/// 영문 입력 소스 참조를 캐싱 (최초 1회만 검색, ABC 또는 US)
fn get_cached_english_source() -> Option<TISInputSourceRef> {
    let ptr = *ENGLISH_SOURCE_CACHE.get_or_init(|| {
        unsafe {
            let source_list = TISCreateInputSourceList(ptr::null(), true);
            if source_list.is_null() {
                return 0;
            }

            let count = CFArrayGetCount(source_list);
            let mut found: usize = 0;

            // ABC를 우선, 없으면 US
            let target_ids = [ENGLISH_INPUT_SOURCE_ID, ENGLISH_US_INPUT_SOURCE_ID];

            for target_id in &target_ids {
                for i in 0..count {
                    let source_ptr = CFArrayGetValueAtIndex(source_list, i) as TISInputSourceRef;
                    if source_ptr.is_null() {
                        continue;
                    }

                    let source_id_ref =
                        TISGetInputSourceProperty(source_ptr, kTISPropertyInputSourceID);
                    if source_id_ref.is_null() {
                        continue;
                    }

                    let source_id = CFString::wrap_under_get_rule(source_id_ref as CFStringRef);
                    if source_id.to_string() == *target_id {
                        CFRetain(source_ptr as CFTypeRef);
                        found = source_ptr as usize;
                        break;
                    }
                }

                if found != 0 {
                    break;
                }
            }

            CFRelease(source_list as CFTypeRef);
            found
        }
    });

    if ptr == 0 { None } else { Some(ptr as TISInputSourceRef) }
}

/// 영문 입력 소스로 전환
pub fn switch_to_english() -> Result<(), String> {
    // 이미 영문이면 전환 불필요
    if is_english_input_source() {
        return Ok(());
    }

    // 캐싱된 소스로 즉시 전환 시도
    let result = if let Some(source) = get_cached_english_source() {
        let ret = unsafe { TISSelectInputSource(source) };
        if ret == 0 {
            Ok(())
        } else {
            // 캐시된 소스 실패 시 폴백
            switch_to_input_source(ENGLISH_INPUT_SOURCE_ID)
                .or_else(|_| switch_to_input_source(ENGLISH_US_INPUT_SOURCE_ID))
        }
    } else {
        // 캐시 실패 시 폴백
        switch_to_input_source(ENGLISH_INPUT_SOURCE_ID)
            .or_else(|_| switch_to_input_source(ENGLISH_US_INPUT_SOURCE_ID))
    };

    // Sonoma+에서 전환 완료 검증
    if result.is_ok() && !verify_switch(|id| !is_korean_input_source_id(id)) {
        log::warn!("영문 전환 검증 실패 — 전환이 지연되었을 수 있음");
    }

    // 전환 후 캐시 무효화
    invalidate_input_source_cache();

    result
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
