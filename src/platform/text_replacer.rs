//! 텍스트 교체 (Backspace + Paste 시뮬레이션)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use cocoa::appkit::NSPasteboard;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc::{msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::platform::os_version::{is_sequoia_or_later, is_sonoma_or_later};

/// Koing이 생성한 합성 이벤트를 식별하는 마커 값
pub const KOING_SYNTHETIC_EVENT_MARKER: i64 = 0x4B4F494E47; // "KOING"

/// 버전별 타이밍 프로파일
/// Sonoma/Sequoia에서 보안 정책이 강화되어 더 긴 딜레이가 필요
struct TimingProfile {
    /// Backspace key down/up 사이 딜레이 (ms)
    backspace_key_delay_ms: u64,
    /// Paste 키 이벤트 사이 딜레이 (ms)
    paste_key_delay_ms: u64,
    /// Paste 완료 후 딜레이 (ms)
    paste_finish_delay_ms: u64,
    /// Backspace 완료 → 클립보드 복사 사이 딜레이 (ms)
    post_backspace_delay_ms: u64,
}

impl TimingProfile {
    fn for_current_os() -> Self {
        if is_sequoia_or_later() {
            Self {
                backspace_key_delay_ms: 4,
                paste_key_delay_ms: 10,
                paste_finish_delay_ms: 40,
                post_backspace_delay_ms: 40,
            }
        } else if is_sonoma_or_later() {
            Self {
                backspace_key_delay_ms: 3,
                paste_key_delay_ms: 8,
                paste_finish_delay_ms: 30,
                post_backspace_delay_ms: 30,
            }
        } else {
            // Ventura 이하: 기존 값 유지
            Self {
                backspace_key_delay_ms: 2,
                paste_key_delay_ms: 5,
                paste_finish_delay_ms: 20,
                post_backspace_delay_ms: 20,
            }
        }
    }
}

/// 캐싱된 타이밍 프로파일 (앱 수명 동안 1회만 생성)
static TIMING: std::sync::OnceLock<TimingProfile> = std::sync::OnceLock::new();

fn timing() -> &'static TimingProfile {
    TIMING.get_or_init(TimingProfile::for_current_os)
}

/// 클립보드 작업 직렬화를 위한 글로벌 Mutex
static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());

/// 클립보드 복원 세대 카운터 — 최신 변환만 복원 수행
static RESTORE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// 클립보드 복원 지연 시간 (ms) — 대상 앱이 paste를 처리할 충분한 시간
const DEFERRED_RESTORE_DELAY_MS: u64 = 1500;

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
        let type_str: id = NSString::alloc(nil).init_str("public.utf8-plain-text");
        let types: id = NSArray::arrayWithObject(nil, type_str);
        let available: id = msg_send![pasteboard, availableTypeFromArray: types];
        let _: () = msg_send![type_str, release];

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

        let ns_string: id = NSString::alloc(nil).init_str(content);
        let type_str: id = NSString::alloc(nil).init_str("public.utf8-plain-text");
        let types: id = NSArray::arrayWithObject(nil, type_str);

        let _: () = msg_send![pasteboard, declareTypes: types owner: nil];
        let for_type: id = NSString::alloc(nil).init_str("public.utf8-plain-text");
        let _: () = msg_send![pasteboard, setString: ns_string forType: for_type];
        let _: () = msg_send![for_type, release];
        let _: () = msg_send![type_str, release];
        let _: () = msg_send![ns_string, release];
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

/// 현재 OS 버전에 적합한 이벤트 소스 상태 ID 반환
/// Sequoia에서는 HIDSystemState가 더 안정적
fn event_source_state_id() -> CGEventSourceStateID {
    if is_sequoia_or_later() {
        CGEventSourceStateID::HIDSystemState
    } else {
        CGEventSourceStateID::Private
    }
}

/// 키 이벤트 시뮬레이션
fn simulate_key(keycode: CGKeyCode, key_down: bool, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(event_source_state_id())
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
    let t = timing();
    const BACKSPACE_KEYCODE: CGKeyCode = 51;
    simulate_key(BACKSPACE_KEYCODE, true, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(t.backspace_key_delay_ms));
    simulate_key(BACKSPACE_KEYCODE, false, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(t.backspace_key_delay_ms));
    Ok(())
}

/// Cmd+V (붙여넣기) 시뮬레이션
fn simulate_paste() -> Result<(), String> {
    let t = timing();
    const V_KEYCODE: CGKeyCode = 9;
    const COMMAND_KEYCODE: CGKeyCode = 55; // Left Command

    // 1. Command 키 다운
    simulate_key(COMMAND_KEYCODE, true, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(t.paste_key_delay_ms));

    // 2. V 키 다운 (Command 플래그 포함)
    simulate_key(V_KEYCODE, true, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(t.paste_key_delay_ms));

    // 3. V 키 업
    simulate_key(V_KEYCODE, false, CGEventFlags::CGEventFlagCommand)?;
    thread::sleep(Duration::from_millis(t.paste_key_delay_ms));

    // 4. Command 키 업
    simulate_key(COMMAND_KEYCODE, false, CGEventFlags::empty())?;
    thread::sleep(Duration::from_millis(t.paste_finish_delay_ms));

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

    let t = timing();

    // 2. Backspace로 기존 텍스트 삭제
    for _ in 0..backspace_count {
        simulate_backspace()?;
    }

    // 약간의 딜레이 (Backspace 처리 완료 대기)
    thread::sleep(Duration::from_millis(t.post_backspace_delay_ms));

    // 3. 새 텍스트를 클립보드에 복사
    set_clipboard_string(new_text);

    // 4. 클립보드 설정 완료 대기 (폴링 방식, 최대 100ms)
    if !wait_for_clipboard(new_text, 100) {
        log::warn!("클립보드 설정 확인 실패, 계속 진행");
    }

    // 5. Cmd+V로 붙여넣기
    simulate_paste()?;

    // 6. 클립보드 복원을 지연 처리
    // 대상 앱이 paste를 처리하기 전에 복원하면 이전 클립보드 내용이 출력됨
    schedule_deferred_restore(backup.content);

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

    let t = timing();

    // 2. Backspace로 한글 텍스트 삭제
    for _ in 0..backspace_count {
        simulate_backspace()?;
    }

    // 약간의 딜레이
    thread::sleep(Duration::from_millis(t.post_backspace_delay_ms));

    // 3. 원본 영문 텍스트를 클립보드에 복사
    set_clipboard_string(original_text);

    // 4. 클립보드 설정 완료 대기 (폴링 방식, 최대 100ms)
    if !wait_for_clipboard(original_text, 100) {
        log::warn!("클립보드 설정 확인 실패, 계속 진행");
    }

    // 5. Cmd+V로 붙여넣기
    simulate_paste()?;

    // 6. 클립보드 복원을 지연 처리
    schedule_deferred_restore(backup.content);

    Ok(())
}

/// 클립보드 복원을 백그라운드 스레드에서 지연 실행
/// 세대 카운터로 최신 변환만 복원하고, 이전 변환의 복원은 자동 취소
fn schedule_deferred_restore(content: Option<String>) {
    let gen = RESTORE_GENERATION.fetch_add(1, AtomicOrdering::SeqCst);
    if let Some(content) = content {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(DEFERRED_RESTORE_DELAY_MS));
            // 이후 새 변환이 없었을 때만 복원
            if RESTORE_GENERATION.load(AtomicOrdering::SeqCst) == gen + 1 {
                let _lock = CLIPBOARD_MUTEX.lock();
                set_clipboard_string(&content);
            }
        });
    }
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
