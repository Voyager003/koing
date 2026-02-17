//! 입력 소스(한/영) 전환 기능
//! Carbon API의 TIS (Text Input Source) 함수 사용

use crate::platform::os_version::is_sonoma_or_later;
use core_foundation::array::CFArrayRef;
use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

/// 캐싱된 입력 소스 상태 (true = 영문)
static INPUT_SOURCE_IS_ENGLISH: AtomicBool = AtomicBool::new(true);
/// 캐시 유효 여부
static INPUT_SOURCE_CACHE_VALID: AtomicBool = AtomicBool::new(false);
/// 캐시 마지막 갱신 시간 (epoch ms)
static INPUT_SOURCE_CACHE_TIME: AtomicU64 = AtomicU64::new(0);
/// 캐시 유효 기간 (ms) — 외부 도구(InputSource Pro 등)의 입력 소스 변경을 감지하기 위한 TTL
const INPUT_SOURCE_CACHE_TTL_MS: u64 = 100;

/// 현재 시간을 epoch ms로 반환
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

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
///
/// 주의: 한글 IME의 영문 서브모드(Korean.Roman)도 "korean"을 포함하므로
/// 이 함수만으로는 실제 한글 타이핑 중인지 알 수 없음.
/// `is_korean_english_submode`와 함께 사용할 것.
pub(crate) fn is_korean_input_source_id(id: &str) -> bool {
    contains_ascii_case_insensitive(id, "korean")
        || contains_ascii_case_insensitive(id, "hangul")
        || contains_ascii_case_insensitive(id, "gureum")
        || contains_ascii_case_insensitive(id, "hangeul")
}

/// 한글 IME의 영문 서브모드(A 모드)인지 확인
/// e.g., com.apple.inputmethod.Korean.Roman
/// 입력 소스 ID에 "korean"과 "roman"이 모두 포함되면 영문 서브모드로 판단
fn is_korean_english_submode(id: &str) -> bool {
    is_korean_input_source_id(id) && contains_ascii_case_insensitive(id, "roman")
}

/// 현재 스레드가 메인 스레드인지 확인
fn is_main_thread() -> bool {
    extern "C" {
        fn pthread_main_np() -> i32;
    }
    unsafe { pthread_main_np() != 0 }
}

/// TIS API를 호출하여 입력 소스 캐시 갱신 (반드시 메인 스레드에서 호출)
fn refresh_input_source_cache() {
    let is_english = if let Some(id) = get_current_input_source_id() {
        !is_korean_input_source_id(&id) || is_korean_english_submode(&id)
    } else {
        true
    };
    INPUT_SOURCE_IS_ENGLISH.store(is_english, Ordering::Release);
    INPUT_SOURCE_CACHE_TIME.store(current_time_ms(), Ordering::Release);
    INPUT_SOURCE_CACHE_VALID.store(true, Ordering::Release);
}

/// 현재 영문 입력 소스인지 확인 (TTL 기반 캐시 활용)
///
/// TIS API(TISCopyCurrentKeyboardInputSource 등)는 macOS 26.2+에서
/// 메인 큐에서만 호출 가능. 캐시 만료 시 메인 스레드로 디스패치하여 갱신.
pub fn is_english_input_source() -> bool {
    // 캐시가 유효하고 TTL 이내이면 atomic 읽기만으로 즉시 반환
    if INPUT_SOURCE_CACHE_VALID.load(Ordering::Acquire) {
        let cached_time = INPUT_SOURCE_CACHE_TIME.load(Ordering::Acquire);
        let now = current_time_ms();
        if now.saturating_sub(cached_time) < INPUT_SOURCE_CACHE_TTL_MS {
            return INPUT_SOURCE_IS_ENGLISH.load(Ordering::Acquire);
        }
        // TTL 만료 — 재조회
    }

    // 캐시 미스 또는 TTL 만료 — TIS API는 메인 스레드에서만 호출
    if is_main_thread() {
        refresh_input_source_cache();
    } else {
        // event tap 스레드 등: 메인 스레드에 동기 디스패치
        crate::platform::dispatch_to_main_sync(|| {
            refresh_input_source_cache();
        });
    }

    INPUT_SOURCE_IS_ENGLISH.load(Ordering::Acquire)
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

/// 입력 소스 전환 후 실제로 전환되었는지 검증
/// 모든 macOS 버전에서 실제 확인 수행 (dispatch_to_main 사용 시 비동기 가능)
fn verify_switch(expected_check: impl Fn(&str) -> bool) -> bool {
    // 즉시 확인 (동기 전환 케이스)
    if let Some(id) = get_current_input_source_id() {
        if expected_check(&id) {
            return true;
        }
    }

    let (poll_interval_ms, max_wait_ms) = if is_sonoma_or_later() {
        (10, 150)
    } else {
        (5, 50) // Ventura: 보통 동기 전환이므로 짧은 대기
    };

    let max_tries = max_wait_ms / poll_interval_ms;

    for _ in 0..max_tries {
        thread::sleep(Duration::from_millis(poll_interval_ms));
        if let Some(id) = get_current_input_source_id() {
            if expected_check(&id) {
                return true;
            }
        }
    }

    false
}

/// 한글 입력 소스로 전환 (캐시 실패 시 리스트 검색 폴백)
pub fn switch_to_korean() -> Result<(), String> {
    // 한글 타이핑 모드 검증 함수 (Korean.Roman 영문 서브모드 제외)
    let is_korean_typing_mode =
        |id: &str| is_korean_input_source_id(id) && !is_korean_english_submode(id);

    // 이미 한글 타이핑 모드이면 전환 불필요
    // Korean.Roman(영문 서브모드)은 영문으로 취급하여 전환 수행
    if let Some(id) = get_current_input_source_id() {
        if is_korean_typing_mode(&id) {
            return Ok(());
        }
    }

    // 1차 시도: 캐싱된 소스로 빠른 전환
    if let Some(source) = get_cached_korean_source() {
        let ret = unsafe { TISSelectInputSource(source) };
        if ret == 0 && verify_switch(&is_korean_typing_mode) {
            invalidate_input_source_cache();
            return Ok(());
        }
        let current_id = get_current_input_source_id().unwrap_or_else(|| "unknown".to_string());
        log::warn!("캐시 소스 한글 전환 실패 (ret={}, current={}), 리스트 검색으로 재시도", ret, current_id);
    }

    // 2차 시도: 입력 소스 리스트에서 직접 검색 (캐시 stale 대응)
    thread::sleep(Duration::from_millis(50));
    if let Ok(()) = switch_to_input_source(KOREAN_INPUT_SOURCE_ID) {
        if verify_switch(&is_korean_typing_mode) {
            invalidate_input_source_cache();
            return Ok(());
        }
    }

    // 최종 실패
    invalidate_input_source_cache();
    Err("한글 전환 최종 실패: 캐시 및 리스트 검색 모두 실패".to_string())
}

/// 메인 스레드에서 한글 입력 소스로 전환 (비동기)
/// TISSelectInputSource()는 메인 RunLoop이 있는 스레드에서 호출해야
/// 포커스된 앱의 실제 입력 모드가 변경됨.
/// Worker/timer 스레드에서 직접 호출하면 메뉴바만 바뀌고 실제 입력은 영문 유지.
pub fn switch_to_korean_on_main() {
    crate::platform::dispatch_to_main(|| {
        if let Err(e) = switch_to_korean() {
            log::warn!("한글 전환 실패 (main thread): {}", e);
        }
    });
}

/// 메인 스레드에서 한글 입력 소스로 전환 (동기 — 전환 완료까지 블록)
/// 변환 직후 is_replacing 해제 전에 사용하여, 전환 완료 전 키 입력이
/// 영문으로 처리되는 레이스 컨디션을 방지합니다.
pub fn switch_to_korean_on_main_sync() {
    crate::platform::dispatch_to_main_sync(|| {
        if let Err(e) = switch_to_korean() {
            log::warn!("한글 전환 실패 (main thread sync): {}", e);
        }
    });
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

    #[test]
    fn test_is_korean_input_source_id() {
        // macOS 기본 한글 입력기
        assert!(is_korean_input_source_id("com.apple.inputmethod.Korean.2SetKorean"));
        assert!(is_korean_input_source_id("com.apple.inputmethod.Korean.3SetKorean"));
        assert!(is_korean_input_source_id("com.apple.inputmethod.Korean.Roman"));

        // 서드파티 입력기
        assert!(is_korean_input_source_id("org.youknowone.inputmethod.Gureum.han2"));

        // 영문 입력기
        assert!(!is_korean_input_source_id("com.apple.keylayout.ABC"));
        assert!(!is_korean_input_source_id("com.apple.keylayout.US"));
    }

    #[test]
    fn test_is_korean_english_submode() {
        // 한글 IME 영문 서브모드 (A 모드) → 영문으로 취급해야 함
        assert!(is_korean_english_submode("com.apple.inputmethod.Korean.Roman"));

        // 한글 타이핑 모드 → 한글로 취급해야 함
        assert!(!is_korean_english_submode("com.apple.inputmethod.Korean.2SetKorean"));
        assert!(!is_korean_english_submode("com.apple.inputmethod.Korean.3SetKorean"));

        // 영문 입력기 → false (korean이 아니므로)
        assert!(!is_korean_english_submode("com.apple.keylayout.ABC"));
    }
}
