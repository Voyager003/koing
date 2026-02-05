//! 텍스트 교체 (Backspace + Paste 시뮬레이션)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use cocoa::appkit::NSPasteboard;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::{msg_send, sel, sel_impl};
use std::thread;
use std::time::Duration;

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

/// 키 이벤트 시뮬레이션
fn simulate_key(keycode: CGKeyCode, key_down: bool, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "CGEventSource 생성 실패")?;

    let event =
        CGEvent::new_keyboard_event(source, keycode, key_down).map_err(|_| "CGEvent 생성 실패")?;

    event.set_flags(flags);
    event.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

/// Backspace 키 시뮬레이션
fn simulate_backspace() -> Result<(), String> {
    const BACKSPACE_KEYCODE: CGKeyCode = 51;
    simulate_key(BACKSPACE_KEYCODE, true, CGEventFlags::empty())?;
    thread::sleep(Duration::from_micros(500));
    simulate_key(BACKSPACE_KEYCODE, false, CGEventFlags::empty())?;
    thread::sleep(Duration::from_micros(500));
    Ok(())
}

/// Cmd+V (붙여넣기) 시뮬레이션
fn simulate_paste() -> Result<(), String> {
    const V_KEYCODE: CGKeyCode = 9;

    // Cmd 키 다운
    simulate_key(V_KEYCODE, true, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_micros(500));

    // Cmd 키 업
    simulate_key(V_KEYCODE, false, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(10));

    Ok(())
}

/// 텍스트 교체 실행
/// - backspace_count: 삭제할 문자 수
/// - new_text: 새로 입력할 텍스트
pub fn replace_text(backspace_count: usize, new_text: &str) -> Result<(), String> {
    if new_text.is_empty() {
        return Ok(());
    }

    // 1. 클립보드 백업
    let backup = ClipboardBackup::save();

    // 2. Backspace로 기존 텍스트 삭제
    for _ in 0..backspace_count {
        simulate_backspace()?;
    }

    // 약간의 딜레이
    thread::sleep(Duration::from_millis(10));

    // 3. 새 텍스트를 클립보드에 복사
    set_clipboard_string(new_text);

    // 4. Cmd+V로 붙여넣기
    simulate_paste()?;

    // 5. 클립보드 복원 (앱이 붙여넣기를 완료할 때까지 충분한 딜레이)
    // 50ms는 불충분 - 앱이 Cmd+V를 처리하기 전에 클립보드가 복원되는 race condition 발생
    thread::sleep(Duration::from_millis(300));
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
