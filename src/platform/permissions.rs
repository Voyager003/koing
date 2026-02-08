//! Accessibility 권한 확인 및 요청

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use std::ptr;
use std::time::Duration;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(
        options: *const core_foundation::dictionary::__CFDictionary,
    ) -> bool;
}

/// Accessibility 권한이 있는지 확인
pub fn check_accessibility_permission() -> bool {
    unsafe { AXIsProcessTrustedWithOptions(ptr::null()) }
}

/// Accessibility 권한 확인 및 없으면 시스템 설정 창 열기
/// prompt가 true이면 권한 요청 다이얼로그 표시
pub fn request_accessibility_permission(prompt: bool) -> bool {
    unsafe {
        if prompt {
            let key = CFString::new("AXTrustedCheckOptionPrompt");
            let value = CFBoolean::true_value();
            let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
            AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
        } else {
            AXIsProcessTrustedWithOptions(ptr::null())
        }
    }
}

/// Accessibility 권한이 부여될 때까지 폴링 대기
/// Sequoia에서 TCC DB 업데이트가 지연될 수 있으므로 주기적으로 확인
/// 반환: 권한 획득 여부
pub fn wait_for_accessibility_permission(timeout: Duration) -> bool {
    const POLL_INTERVAL: Duration = Duration::from_millis(500);

    // 먼저 다이얼로그와 함께 확인
    if request_accessibility_permission(true) {
        return true;
    }

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        std::thread::sleep(POLL_INTERVAL);
        if check_accessibility_permission() {
            return true;
        }
    }

    false
}

/// 권한 상태를 사람이 읽을 수 있는 문자열로 반환
pub fn permission_status_string() -> &'static str {
    if check_accessibility_permission() {
        "Accessibility 권한: 허용됨"
    } else {
        "Accessibility 권한: 필요함 (시스템 설정에서 허용해주세요)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_permission() {
        // 권한 여부와 관계없이 함수가 크래시 없이 실행되어야 함
        let _ = check_accessibility_permission();
    }

    #[test]
    fn test_permission_status_string() {
        let status = permission_status_string();
        assert!(status.contains("Accessibility"));
    }
}
