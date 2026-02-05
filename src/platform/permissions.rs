//! Accessibility 권한 확인 및 요청

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use std::ptr;

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
