//! macOS 버전 런타임 감지
//! sysctlbyname("kern.osproductversion")으로 OS 버전을 파싱하고 OnceLock으로 캐싱

use std::sync::OnceLock;

/// macOS 버전 정보
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacOSVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// 캐싱된 macOS 버전 (앱 수명 동안 1회만 조회)
static MACOS_VERSION: OnceLock<MacOSVersion> = OnceLock::new();

extern "C" {
    fn sysctlbyname(
        name: *const i8,
        oldp: *mut std::ffi::c_void,
        oldlenp: *mut usize,
        newp: *const std::ffi::c_void,
        newlen: usize,
    ) -> i32;
}

/// sysctlbyname으로 macOS 버전 문자열을 가져와 파싱
fn detect_version() -> MacOSVersion {
    let mut buf = [0u8; 32];
    let mut len = buf.len();
    let name = b"kern.osproductversion\0";

    let ret = unsafe {
        sysctlbyname(
            name.as_ptr() as *const i8,
            buf.as_mut_ptr() as *mut std::ffi::c_void,
            &mut len,
            std::ptr::null(),
            0,
        )
    };

    if ret != 0 || len == 0 {
        log::warn!("sysctlbyname 실패, 기본값 macOS 13.0.0 사용");
        return MacOSVersion {
            major: 13,
            minor: 0,
            patch: 0,
        };
    }

    // null terminator 제거
    let version_str = std::str::from_utf8(&buf[..len.saturating_sub(1)])
        .unwrap_or("13.0.0");

    parse_version(version_str)
}

/// "15.2.1" 같은 버전 문자열을 파싱
fn parse_version(s: &str) -> MacOSVersion {
    let parts: Vec<u64> = s
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    MacOSVersion {
        major: parts.first().copied().unwrap_or(13),
        minor: parts.get(1).copied().unwrap_or(0),
        patch: parts.get(2).copied().unwrap_or(0),
    }
}

/// 캐싱된 macOS 버전 가져오기
pub fn get_macos_version() -> MacOSVersion {
    *MACOS_VERSION.get_or_init(detect_version)
}

/// macOS Sonoma (14.x) 이상인지 확인
pub fn is_sonoma_or_later() -> bool {
    get_macos_version().major >= 14
}

/// macOS Sequoia (15.x) 이상인지 확인
pub fn is_sequoia_or_later() -> bool {
    get_macos_version().major >= 15
}

impl std::fmt::Display for MacOSVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = parse_version("15.2.1");
        assert_eq!(v.major, 15);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 1);
    }

    #[test]
    fn test_parse_version_two_parts() {
        let v = parse_version("14.0");
        assert_eq!(v.major, 14);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_is_sonoma_or_later() {
        let v13 = MacOSVersion { major: 13, minor: 6, patch: 0 };
        let v14 = MacOSVersion { major: 14, minor: 0, patch: 0 };
        let v15 = MacOSVersion { major: 15, minor: 1, patch: 0 };
        assert!(v13.major < 14);  // Ventura는 Sonoma 미만
        assert!(v14.major >= 14);
        assert!(v15.major >= 14);
    }

    #[test]
    fn test_detect_version_runs() {
        let v = get_macos_version();
        assert!(v.major >= 13, "macOS 13 이상이어야 함: {}", v);
    }
}
