//! 텍스트 교체 (Backspace + Paste 시뮬레이션)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use cocoa::appkit::NSPasteboard;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::{msg_send, sel, sel_impl};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// Koing이 생성한 합성 이벤트를 식별하는 마커 값
pub const KOING_SYNTHETIC_EVENT_MARKER: i64 = 0x4B4F494E47; // "KOING"

lazy_static::lazy_static! {
    /// 클립보드 작업 직렬화를 위한 글로벌 Mutex
    static ref CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());
}

/// 클립보드 내용을 백업하고 복원하는 구조체
pub struct ClipboardBackup {
    content: Option<String>,
}

impl ClipboardBackup {
    /// 현재 클립보드 내용 백업
    pub fn save() -> Self {
        let content = get_clipboard_string();
        Self { content }
    }

    /// 백업한 내용으로 클립보드 복원
    pub fn restore(self) {
        if let Some(content) = self.content {
            set_clipboard_string(&content);
        }
    }
}

/// 클립보드에서 문자열 가져오기
pub fn get_clipboard_string() -> Option<String> {
    unsafe {
        let pasteboard: id = NSPasteboard::generalPasteboard(nil);
        let types: id =
            NSArray::arrayWithObject(nil, NSString::alloc(nil).init_str("public.utf8-plain-text"));
        let available: id = msg_send![pasteboard, availableTypeFromArray: types];

        if available == nil {
            return None;
        }

        let string: id = msg_send![pasteboard, stringForType: available];
        if string == nil {
            return None;
        }

        let cstr: *const i8 = msg_send![string, UTF8String];
        if cstr.is_null() {
            return None;
        }

        Some(
            std::ffi::CStr::from_ptr(cstr)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// 클립보드에 문자열 설정
pub fn set_clipboard_string(content: &str) {
    unsafe {
        let pasteboard: id = NSPasteboard::generalPasteboard(nil);
        let _: () = msg_send![pasteboard, clearContents];

        let ns_string = NSString::alloc(nil).init_str(content);
        let types =
            NSArray::arrayWithObject(nil, NSString::alloc(nil).init_str("public.utf8-plain-text"));

        let _: () = msg_send![pasteboard, declareTypes: types owner: nil];
        let _: () = msg_send![pasteboard, setString: ns_string forType: NSString::alloc(nil).init_str("public.utf8-plain-text")];
    }
}

/// 클립보드 설정 완료 대기 (폴링 방식)
/// - expected: 기대하는 클립보드 내용
/// - max_wait_ms: 최대 대기 시간 (밀리초)
/// - 반환: 설정 완료 여부
fn wait_for_clipboard(expected: &str, max_wait_ms: u64) -> bool {
    const POLL_INTERVAL_MS: u64 = 5;
    let max_tries = max_wait_ms / POLL_INTERVAL_MS;

    for _ in 0..max_tries {
        if let Some(content) = get_clipboard_string() {
            if content == expected {
                return true;
            }
        }
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }

    // 마지막 확인
    get_clipboard_string()
        .map(|content| content == expected)
        .unwrap_or(false)
}

/// 키 이벤트 시뮬레이션
fn simulate_key(keycode: CGKeyCode, key_down: bool, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::Private)
        .map_err(|_| "CGEventSource 생성 실패")?;

    let event =
        CGEvent::new_keyboard_event(source, keycode, key_down).map_err(|_| "CGEvent 생성 실패")?;

    event.set_flags(flags);
    event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, KOING_SYNTHETIC_EVENT_MARKER);
    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// Backspace 키 시뮬레이션
fn simulate_backspace() -> Result<(), String> {
    const BACKSPACE_KEYCODE: CGKeyCode = 51;
    simulate_key(BACKSPACE_KEYCODE, true, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(2));
    simulate_key(BACKSPACE_KEYCODE, false, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(2));
    Ok(())
}

/// Cmd+V (붙여넣기) 시뮬레이션
fn simulate_paste() -> Result<(), String> {
    const V_KEYCODE: CGKeyCode = 9;
    const COMMAND_KEYCODE: CGKeyCode = 55; // Left Command

    // 1. Command 키 다운
    simulate_key(COMMAND_KEYCODE, true, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(5));

    // 2. V 키 다운 (Command 플래그 포함)
    simulate_key(V_KEYCODE, true, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(5));

    // 3. V 키 업
    simulate_key(V_KEYCODE, false, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(5));

    // 4. Command 키 업
    simulate_key(COMMAND_KEYCODE, false, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(20));

    Ok(())
}

/// 텍스트 교체 실행
/// - backspace_count: 삭제할 문자 수
/// - new_text: 새로 입력할 텍스트
pub fn replace_text(backspace_count: usize, new_text: &str) -> Result<(), String> {
    if new_text.is_empty() {
        return Ok(());
    }

    // 클립보드 작업 직렬화 — 동시 변환 요청 방지
    let _lock = CLIPBOARD_MUTEX
        .lock()
        .map_err(|e| format!("클립보드 Mutex 획득 실패: {}", e))?;

    // 1. 클립보드 백업
    let backup = ClipboardBackup::save();

    // 2. Backspace로 기존 텍스트 삭제
    for _ in 0..backspace_count {
        simulate_backspace()?;
    }

    // 약간의 딜레이 (Backspace 처리 완료 대기)
    thread::sleep(Duration::from_millis(20));

    // 3. 새 텍스트를 클립보드에 복사
    set_clipboard_string(new_text);

    // 4. 클립보드 설정 완료 대기 (폴링 방식, 최대 100ms)
    if !wait_for_clipboard(new_text, 100) {
        log::warn!("클립보드 설정 확인 실패, 계속 진행");
    }

    // 5. Cmd+V로 붙여넣기
    simulate_paste()?;

    // 6. 클립보드 복원 (앱이 붙여넣기를 완료할 때까지 충분한 딜레이)
    // 느린 앱(Electron 등)도 붙여넣기를 완료할 수 있도록 1500ms 대기
    thread::sleep(Duration::from_millis(500));
    backup.restore();

    Ok(())
}

/// Undo 텍스트 교체 실행 (한글 → 원본 영문 복원)
/// - hangul_text: 현재 입력된 한글 텍스트
/// - original_text: 복원할 원본 영문 텍스트
pub fn undo_replace_text(hangul_text: &str, original_text: &str) -> Result<(), String> {
    if original_text.is_empty() {
        return Ok(());
    }

    // 클립보드 작업 직렬화 — 동시 변환 요청 방지
    let _lock = CLIPBOARD_MUTEX
        .lock()
        .map_err(|e| format!("클립보드 Mutex 획득 실패: {}", e))?;

    // 한글은 조합 문자이므로 chars().count()로 정확한 문자 수 계산
    let backspace_count = hangul_text.chars().count();

    // 1. 클립보드 백업
    let backup = ClipboardBackup::save();

    // 2. Backspace로 한글 텍스트 삭제
    for _ in 0..backspace_count {
        simulate_backspace()?;
    }

    // 약간의 딜레이
    thread::sleep(Duration::from_millis(20));

    // 3. 원본 영문 텍스트를 클립보드에 복사
    set_clipboard_string(original_text);

    // 4. 클립보드 설정 완료 대기 (폴링 방식, 최대 100ms)
    if !wait_for_clipboard(original_text, 100) {
        log::warn!("클립보드 설정 확인 실패, 계속 진행");
    }

    // 5. Cmd+V로 붙여넣기
    simulate_paste()?;

    // 6. 클립보드 복원
    // 느린 앱(Electron 등)도 붙여넣기를 완료할 수 있도록 1500ms 대기
    thread::sleep(Duration::from_millis(500));
    backup.restore();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // GUI 환경에서만 테스트 가능
    fn test_clipboard_operations() {
        let original = get_clipboard_string();

        set_clipboard_string("테스트 문자열");
        let result = get_clipboard_string();
        assert_eq!(result, Some("테스트 문자열".to_string()));

        // 원래 내용 복원
        if let Some(orig) = original {
            set_clipboard_string(&orig);
        }
    }

    #[test]
    #[ignore] // GUI 환경에서만 테스트 가능
    fn test_clipboard_backup() {
        let original = get_clipboard_string();
        set_clipboard_string("원본 내용");

        let backup = ClipboardBackup::save();
        set_clipboard_string("임시 내용");
        backup.restore();

        let restored = get_clipboard_string();
        assert_eq!(restored, Some("원본 내용".to_string()));

        // 원래 내용 복원
        if let Some(orig) = original {
            set_clipboard_string(&orig);
        }
    }
}
