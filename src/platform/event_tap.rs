//! CGEventTap을 사용한 키보드 이벤트 감지

use crate::detection::AutoDetector;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 키 버퍼 - 입력된 영문 키를 누적
pub struct KeyBuffer {
    buffer: String,
    max_size: usize,
}

impl KeyBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: String::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, c: char) {
        if self.buffer.len() >= self.max_size {
            // 오래된 문자 제거
            self.buffer.remove(0);
        }
        self.buffer.push(c);
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn get(&self) -> &str {
        &self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// macOS 키코드를 문자로 변환 (US 키보드 레이아웃 기준)
fn keycode_to_char(keycode: u16, shift: bool) -> Option<char> {
    // macOS Virtual Keycode -> ASCII 문자
    // 참고: https://eastmanreference.com/complete-list-of-applescript-key-codes
    let base = match keycode {
        0 => 'a',
        1 => 's',
        2 => 'd',
        3 => 'f',
        4 => 'h',
        5 => 'g',
        6 => 'z',
        7 => 'x',
        8 => 'c',
        9 => 'v',
        11 => 'b',
        12 => 'q',
        13 => 'w',
        14 => 'e',
        15 => 'r',
        16 => 'y',
        17 => 't',
        18 => '1',
        19 => '2',
        20 => '3',
        21 => '4',
        22 => '6',
        23 => '5',
        24 => '=',
        25 => '9',
        26 => '7',
        27 => '-',
        28 => '8',
        29 => '0',
        30 => ']',
        31 => 'o',
        32 => 'u',
        33 => '[',
        34 => 'i',
        35 => 'p',
        37 => 'l',
        38 => 'j',
        39 => '\'',
        40 => 'k',
        41 => ';',
        42 => '\\',
        43 => ',',
        44 => '/',
        45 => 'n',
        46 => 'm',
        47 => '.',
        50 => '`',
        _ => return None,
    };

    Some(if shift {
        base.to_ascii_uppercase()
    } else {
        base
    })
}

/// 단축키 설정
#[derive(Clone, Copy)]
pub struct HotkeyConfig {
    /// Option 키 필요 여부
    pub require_option: bool,
    /// Space 키코드 (49)
    pub trigger_keycode: u16,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            require_option: true,
            trigger_keycode: 49, // Space
        }
    }
}

/// 이벤트 탭 핸들러에서 사용할 공유 상태
pub struct EventTapState {
    pub buffer: Mutex<KeyBuffer>,
    pub hotkey: HotkeyConfig,
    pub running: AtomicBool,
    pub auto_detector: Mutex<AutoDetector>,
    pub on_convert: Mutex<Option<Box<dyn Fn(String) + Send + 'static>>>,
}

impl EventTapState {
    pub fn new(hotkey: HotkeyConfig) -> Self {
        Self {
            buffer: Mutex::new(KeyBuffer::new(100)),
            hotkey,
            running: AtomicBool::new(true),
            auto_detector: Mutex::new(AutoDetector::default()),
            on_convert: Mutex::new(None),
        }
    }

    pub fn set_convert_callback<F>(&self, callback: F)
    where
        F: Fn(String) + Send + 'static,
    {
        let mut on_convert = self.on_convert.lock().unwrap();
        *on_convert = Some(Box::new(callback));
    }

    /// 자동 감지 활성화/비활성화
    pub fn set_auto_detect_enabled(&self, enabled: bool) {
        if let Ok(mut detector) = self.auto_detector.lock() {
            detector.set_enabled(enabled);
        }
    }

    /// 자동 감지 활성화 여부
    pub fn is_auto_detect_enabled(&self) -> bool {
        self.auto_detector
            .lock()
            .map(|d| d.is_enabled())
            .unwrap_or(false)
    }
}

/// 이벤트 탭 시작
/// 반환: 성공 시 EventTapState의 Arc, 실패 시 에러 메시지
pub fn start_event_tap(state: Arc<EventTapState>) -> Result<(), String> {
    let state_clone = Arc::clone(&state);

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown, CGEventType::FlagsChanged],
        move |_proxy, event_type, event| handle_event(&state_clone, event_type, event),
    )
    .map_err(|_| "CGEventTap 생성 실패. Accessibility 권한을 확인하세요.")?;

    unsafe {
        let loop_source = tap
            .mach_port
            .create_runloop_source(0)
            .map_err(|_| "RunLoop source 생성 실패")?;

        CFRunLoop::get_current().add_source(&loop_source, kCFRunLoopCommonModes);

        tap.enable();

        log::info!("Event tap 시작됨");

        // 메인 런루프 실행 (블로킹)
        CFRunLoop::run_current();
    }

    Ok(())
}

/// 이벤트 처리
fn handle_event(
    state: &EventTapState,
    event_type: CGEventType,
    event: &CGEvent,
) -> Option<CGEvent> {
    if !state.running.load(Ordering::SeqCst) {
        return Some(event.clone());
    }

    match event_type {
        CGEventType::KeyDown => {
            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
            let flags = event.get_flags();

            // 단축키 체크 (Option + Space)
            if keycode == state.hotkey.trigger_keycode {
                let option_pressed = flags.contains(CGEventFlags::CGEventFlagAlternate);

                if state.hotkey.require_option && option_pressed {
                    // 변환 트리거
                    let buffer_content = {
                        let mut buffer = state.buffer.lock().unwrap();
                        let content = buffer.get().to_string();
                        buffer.clear();
                        content
                    };

                    if !buffer_content.is_empty() {
                        if let Some(callback) = state.on_convert.lock().unwrap().as_ref() {
                            callback(buffer_content);
                        }
                    }

                    // 이벤트 소비 (Option+Space가 입력되지 않도록)
                    return None;
                }
            }

            // 일반 키 입력 처리
            let shift_pressed = flags.contains(CGEventFlags::CGEventFlagShift);

            // 버퍼 초기화 조건: Tab, Escape, 방향키
            if matches!(keycode, 48 | 53 | 123..=126) {
                state.buffer.lock().unwrap().clear();
                return Some(event.clone());
            }

            // Space 또는 Enter 입력 시 자동 감지 체크
            if keycode == 49 || keycode == 36 {
                // 49 = Space, 36 = Enter
                let should_convert = {
                    let buffer = state.buffer.lock().unwrap();
                    let detector = state.auto_detector.lock().unwrap();
                    detector.should_convert(buffer.get())
                };

                if should_convert {
                    // 자동 변환 트리거
                    let buffer_content = {
                        let mut buffer = state.buffer.lock().unwrap();
                        let content = buffer.get().to_string();
                        buffer.clear();
                        content
                    };

                    if !buffer_content.is_empty() {
                        if let Some(callback) = state.on_convert.lock().unwrap().as_ref() {
                            callback(buffer_content);
                        }
                    }

                    // 이벤트는 통과시킴 (Space/Enter는 정상 입력되어야 함)
                    return Some(event.clone());
                }

                // 자동 감지 조건 미충족 시 버퍼만 초기화
                state.buffer.lock().unwrap().clear();
                return Some(event.clone());
            }

            // 문자 키 처리
            if let Some(c) = keycode_to_char(keycode, shift_pressed) {
                state.buffer.lock().unwrap().push(c);
            }

            Some(event.clone())
        }
        _ => Some(event.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_buffer() {
        let mut buffer = KeyBuffer::new(5);
        buffer.push('a');
        buffer.push('b');
        buffer.push('c');
        assert_eq!(buffer.get(), "abc");

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_key_buffer_overflow() {
        let mut buffer = KeyBuffer::new(3);
        buffer.push('a');
        buffer.push('b');
        buffer.push('c');
        buffer.push('d');
        assert_eq!(buffer.get(), "bcd");
    }

    #[test]
    fn test_keycode_to_char() {
        assert_eq!(keycode_to_char(0, false), Some('a'));
        assert_eq!(keycode_to_char(0, true), Some('A'));
        assert_eq!(keycode_to_char(15, false), Some('r'));
        assert_eq!(keycode_to_char(15, true), Some('R'));
    }

    #[test]
    fn test_hotkey_config_default() {
        let config = HotkeyConfig::default();
        assert!(config.require_option);
        assert_eq!(config.trigger_keycode, 49);
    }
}
