//! CGEventTap을 사용한 키보드 이벤트 감지

use crate::detection::AutoDetector;
use crate::platform::input_source::{is_english_input_source, switch_to_korean};
use crate::platform::text_replacer::KOING_SYNTHETIC_EVENT_MARKER;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

extern "C" {
    /// macOS CoreGraphics: 이벤트 탭 활성화/비활성화
    fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);
}

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

    /// 마지막 n개의 문자 삭제 후 새 문자열 추가
    pub fn replace_last(&mut self, remove_count: usize, new_text: &str) {
        for _ in 0..remove_count {
            self.buffer.pop();
        }
        for c in new_text.chars() {
            self.push(c);
        }
    }
}

/// Debounce 타이머 명령
#[derive(Debug)]
pub enum DebounceCommand {
    /// 타이머 리셋 (키 입력 시)
    Reset,
    /// 타이머 취소 (버퍼 클리어 시)
    Cancel,
    /// 즉시 트리거 (비한글 키 입력 시)
    Trigger,
}

/// 한글 전환 타이머 명령
#[derive(Debug)]
pub enum SwitchCommand {
    /// 타이머 시작/리셋 (자동 변환 후)
    Reset,
    /// 타이머 취소 (새 키 입력 시)
    Cancel,
}

/// 변환 이력 (Undo용)
#[derive(Debug, Clone)]
pub struct ConversionHistory {
    /// 원본 영문 텍스트
    pub original: String,
    /// 변환된 한글 텍스트
    pub converted: String,
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

/// 두벌식 자판에서 자음/모음으로 매핑되는 키인지 확인
fn is_hangul_key(c: char) -> bool {
    // 두벌식 자음 키
    const CONSONANT_KEYS: &[char] = &[
        'r', 'R', // ㄱ, ㄲ
        's', // ㄴ
        'e', 'E', // ㄷ, ㄸ
        'f', // ㄹ
        'a', // ㅁ
        'q', 'Q', // ㅂ, ㅃ
        't', 'T', // ㅅ, ㅆ
        'd', // ㅇ
        'w', 'W', // ㅈ, ㅉ
        'c', // ㅊ
        'z', // ㅋ
        'x', // ㅌ
        'v', // ㅍ
        'g', // ㅎ
    ];

    // 두벌식 모음 키
    const VOWEL_KEYS: &[char] = &[
        'k', // ㅏ
        'o', // ㅐ
        'i', // ㅑ
        'O', // ㅒ
        'j', // ㅓ
        'p', // ㅔ
        'u', // ㅕ
        'P', // ㅖ
        'h', // ㅗ
        'y', // ㅛ
        'n', // ㅜ
        'b', // ㅠ
        'm', // ㅡ
        'l', // ㅣ
    ];

    CONSONANT_KEYS.contains(&c) || VOWEL_KEYS.contains(&c)
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
    pub on_convert: Mutex<Option<Box<dyn Fn(String, bool) + Send + 'static>>>,
    /// Undo 콜백 (한글 텍스트, 원본 영문 텍스트)
    pub on_undo: Mutex<Option<Box<dyn Fn(String, String) + Send + 'static>>>,
    /// 실시간 모드 활성화 여부
    pub realtime_mode: AtomicBool,
    /// Debounce 타이머 채널
    pub debounce_tx: Mutex<Option<Sender<DebounceCommand>>>,
    /// 한글 전환 타이머 채널
    pub switch_tx: Mutex<Option<Sender<SwitchCommand>>>,
    /// 마지막 키 입력 시간 (ms 단위 epoch)
    pub last_key_time: AtomicU64,
    /// 변환 이력 (Undo용)
    pub conversion_history: Mutex<Option<ConversionHistory>>,
    /// 텍스트 교체 중 여부 (레이스 컨디션 방지)
    pub is_replacing: AtomicBool,
    /// 변환 감지 debounce 시간 (ms)
    pub debounce_ms: AtomicU64,
    /// 한글 자판 전환 지연 시간 (ms)
    pub switch_delay_ms: AtomicU64,
    /// CGEventTap mach port (이벤트 탭 재활성화용)
    tap_port: AtomicPtr<std::ffi::c_void>,
}

impl EventTapState {
    pub fn new(hotkey: HotkeyConfig) -> Self {
        Self {
            buffer: Mutex::new(KeyBuffer::new(100)),
            hotkey,
            running: AtomicBool::new(true),
            auto_detector: Mutex::new(AutoDetector::default()),
            on_convert: Mutex::new(None),
            on_undo: Mutex::new(None),
            realtime_mode: AtomicBool::new(true), // 기본 활성화
            debounce_tx: Mutex::new(None),
            switch_tx: Mutex::new(None),
            last_key_time: AtomicU64::new(0),
            conversion_history: Mutex::new(None),
            is_replacing: AtomicBool::new(false),
            debounce_ms: AtomicU64::new(300),
            switch_delay_ms: AtomicU64::new(0),
            tap_port: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    pub fn set_convert_callback<F>(&self, callback: F)
    where
        F: Fn(String, bool) + Send + 'static,
    {
        let mut on_convert = self.on_convert.lock().unwrap();
        *on_convert = Some(Box::new(callback));
    }

    pub fn set_undo_callback<F>(&self, callback: F)
    where
        F: Fn(String, String) + Send + 'static,
    {
        let mut on_undo = self.on_undo.lock().unwrap();
        *on_undo = Some(Box::new(callback));
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

    /// 실시간 모드 활성화/비활성화
    pub fn set_realtime_mode(&self, enabled: bool) {
        self.realtime_mode.store(enabled, Ordering::SeqCst);
    }

    /// 실시간 모드 활성화 여부
    pub fn is_realtime_mode(&self) -> bool {
        self.realtime_mode.load(Ordering::SeqCst)
    }

    /// 변환 감지 debounce 시간 설정
    pub fn set_debounce_ms(&self, ms: u64) {
        self.debounce_ms.store(ms, Ordering::SeqCst);
    }

    /// 변환 감지 debounce 시간 읽기
    pub fn get_debounce_ms(&self) -> u64 {
        self.debounce_ms.load(Ordering::SeqCst)
    }

    /// 한글 자판 전환 지연 시간 설정
    pub fn set_switch_delay_ms(&self, ms: u64) {
        self.switch_delay_ms.store(ms, Ordering::SeqCst);
    }

    /// 한글 자판 전환 지연 시간 읽기
    pub fn get_switch_delay_ms(&self) -> u64 {
        self.switch_delay_ms.load(Ordering::SeqCst)
    }

    /// Debounce 타이머에 명령 전송
    fn send_debounce_command(&self, cmd: DebounceCommand) {
        if let Ok(tx_guard) = self.debounce_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(cmd);
            }
        }
    }

    /// 한글 전환 타이머에 명령 전송
    pub fn send_switch_command(&self, cmd: SwitchCommand) {
        if let Ok(tx_guard) = self.switch_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                let _ = tx.send(cmd);
            }
        }
    }

    /// 변환 이력 저장 (Undo용)
    pub fn save_conversion_history(&self, original: String, converted: String) {
        if let Ok(mut history) = self.conversion_history.lock() {
            *history = Some(ConversionHistory {
                original,
                converted,
            });
        }
    }

    /// 이벤트 탭 mach port 설정
    fn set_tap_port(&self, port: *mut std::ffi::c_void) {
        self.tap_port.store(port, Ordering::SeqCst);
    }

    /// 비활성화된 이벤트 탭 재활성화
    fn reenable_tap(&self) {
        let port = self.tap_port.load(Ordering::SeqCst);
        if !port.is_null() {
            unsafe {
                CGEventTapEnable(port, true);
            }
            log::warn!("이벤트 탭 재활성화됨");
        }
    }

    /// 변환 이력 가져오기 (Undo용)
    pub fn take_conversion_history(&self) -> Option<ConversionHistory> {
        if let Ok(mut history) = self.conversion_history.lock() {
            history.take()
        } else {
            None
        }
    }
}

/// Debounce 타이머 스레드 시작
fn start_debounce_timer(state: Arc<EventTapState>) {
    let (tx, rx) = mpsc::channel::<DebounceCommand>();

    // 채널 설정
    {
        let mut tx_guard = state.debounce_tx.lock().unwrap();
        *tx_guard = Some(tx);
    }

    let state_for_timer = Arc::clone(&state);

    thread::spawn(move || {
        let mut last_reset = Instant::now();

        loop {
            // 타임아웃 대기
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(DebounceCommand::Reset) => {
                    last_reset = Instant::now();
                }
                Ok(DebounceCommand::Cancel) => {
                    last_reset = Instant::now() + Duration::from_secs(3600); // 먼 미래로
                }
                Ok(DebounceCommand::Trigger) => {
                    // 즉시 트리거
                    trigger_realtime_conversion(&state_for_timer);
                    last_reset = Instant::now() + Duration::from_secs(3600);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let debounce_duration = Duration::from_millis(
                        state_for_timer.debounce_ms.load(Ordering::SeqCst),
                    );
                    if last_reset.elapsed() >= debounce_duration {
                        trigger_realtime_conversion(&state_for_timer);
                        last_reset = Instant::now() + Duration::from_secs(3600);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    log::info!("Debounce 타이머 스레드 종료");
                    break;
                }
            }
        }
    });
}

/// 한글 전환 타이머 스레드 시작
/// 자동 변환 후 500ms간 추가 입력이 없으면 한글 입력 소스로 전환
fn start_switch_timer(state: Arc<EventTapState>) {
    let (tx, rx) = mpsc::channel::<SwitchCommand>();

    // 채널 설정
    {
        let mut tx_guard = state.switch_tx.lock().unwrap();
        *tx_guard = Some(tx);
    }

    let state_for_timer = Arc::clone(&state);

    thread::spawn(move || {
        // 초기 상태: 비활성 (먼 미래)
        let mut last_reset = Instant::now() + Duration::from_secs(3600);
        // 한글 전환이 이미 실행된 상태인지 추적
        // true이면 추가 Reset을 무시하여 이중 전환 방지
        let mut switch_fired = false;

        loop {
            match rx.recv_timeout(Duration::from_millis(5)) {
                Ok(SwitchCommand::Reset) => {
                    if switch_fired {
                        // 이미 한글 전환 완료 상태 — 중복 Reset 무시
                        log::info!("[SwitchTimer] 이미 전환됨, Reset 무시");
                    } else {
                        last_reset = Instant::now();
                    }
                }
                Ok(SwitchCommand::Cancel) => {
                    last_reset = Instant::now() + Duration::from_secs(3600);
                    // 새 타이핑 시작 → 다음 변환 사이클에서 전환 허용
                    switch_fired = false;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let switch_delay_ms = state_for_timer.switch_delay_ms.load(Ordering::SeqCst);
                    if switch_delay_ms == 0 {
                        // 0ms는 main.rs에서 직접 전환 — 타이머 불필요
                        continue;
                    }
                    let switch_delay = Duration::from_millis(switch_delay_ms);
                    if !switch_fired && last_reset.elapsed() >= switch_delay {
                        log::info!(
                            "[SwitchTimer] {}ms 경과, 한글 입력 소스로 전환",
                            switch_delay.as_millis()
                        );
                        if let Err(e) = switch_to_korean() {
                            log::warn!("[SwitchTimer] 한글 전환 실패: {}", e);
                        }
                        last_reset = Instant::now() + Duration::from_secs(3600);
                        switch_fired = true;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    log::info!("한글 전환 타이머 스레드 종료");
                    break;
                }
            }
        }
    });
}

/// 실시간 변환 트리거
fn trigger_realtime_conversion(state: &EventTapState) {
    if !state.is_realtime_mode() {
        return;
    }

    // 텍스트 교체 중이면 실시간 변환 스킵 (레이스 컨디션 방지)
    if state.is_replacing.load(Ordering::SeqCst) {
        return;
    }

    let should_convert = {
        let buffer = state.buffer.lock().unwrap();
        if buffer.is_empty() {
            return;
        }
        let detector = state.auto_detector.lock().unwrap();
        detector.should_convert_realtime(buffer.get())
    };

    if should_convert {
        let buffer_content = {
            let mut buffer = state.buffer.lock().unwrap();
            let content = buffer.get().to_string();
            buffer.clear();
            content
        };

        if !buffer_content.is_empty() {
            log::info!("[실시간] 변환 트리거: '{}'", buffer_content);
            if let Some(callback) = state.on_convert.lock().unwrap().as_ref() {
                callback(buffer_content, false); // 실시간 debounce
            }
        }
    }
}

/// 이벤트 탭 시작
/// 반환: 성공 시 EventTapState의 Arc, 실패 시 에러 메시지
pub fn start_event_tap(state: Arc<EventTapState>) -> Result<(), String> {
    // Debounce 타이머 시작
    start_debounce_timer(Arc::clone(&state));
    // 한글 전환 타이머 시작
    start_switch_timer(Arc::clone(&state));

    let state_clone = Arc::clone(&state);

    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown, CGEventType::FlagsChanged],
        move |_proxy, event_type, event| handle_event(&state_clone, event_type, event),
    )
    .map_err(|_| "CGEventTap 생성 실패. Accessibility 권한을 확인하세요.")?;

    // mach port 포인터 저장 (TapDisabledByTimeout 시 재활성화용)
    use core_foundation::base::TCFType;
    let raw_port = tap.mach_port.as_concrete_TypeRef() as *mut std::ffi::c_void;
    state.set_tap_port(raw_port);

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

    // macOS가 이벤트 탭을 비활성화했으면 즉시 재활성화
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        log::warn!("이벤트 탭 비활성화 감지: {:?}", event_type);
        state.reenable_tap();
        return Some(event.clone());
    }

    // Koing이 생성한 합성 이벤트는 처리하지 않고 통과
    if matches!(event_type, CGEventType::KeyDown) {
        let user_data = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
        if user_data == KOING_SYNTHETIC_EVENT_MARKER {
            return Some(event.clone());
        }
    }

    match event_type {
        CGEventType::KeyDown => {
            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
            let flags = event.get_flags();
            let option_pressed = flags.contains(CGEventFlags::CGEventFlagAlternate);

            // Option + Z = Undo (마지막 변환 되돌리기)
            if keycode == 6 && option_pressed {
                // 6 = Z key
                if let Some(history) = state.take_conversion_history() {
                    log::info!(
                        "[Undo] '{}' → '{}'",
                        history.converted,
                        history.original
                    );
                    // Undo 콜백 호출 (원본 텍스트로 복원)
                    if let Some(callback) = state.on_undo.lock().unwrap().as_ref() {
                        callback(history.converted, history.original);
                    }
                    return None;
                }
                return Some(event.clone());
            }

            // 단축키 체크 (Option + Space)
            if keycode == state.hotkey.trigger_keycode
                && state.hotkey.require_option
                && option_pressed
            {
                // Debounce 및 한글 전환 타이머 취소 (수동 전환이므로 즉시 전환됨)
                state.send_debounce_command(DebounceCommand::Cancel);
                state.send_switch_command(SwitchCommand::Cancel);

                // 변환 트리거
                let buffer_content = {
                    let mut buffer = state.buffer.lock().unwrap();
                    let content = buffer.get().to_string();
                    buffer.clear();
                    content
                };

                if !buffer_content.is_empty() {
                    if let Some(callback) = state.on_convert.lock().unwrap().as_ref() {
                        callback(buffer_content, true); // 수동 단축키
                    }
                }

                // 이벤트 소비 (Option+Space가 입력되지 않도록)
                return None;
            }

            // 일반 키 입력 처리
            let shift_pressed = flags.contains(CGEventFlags::CGEventFlagShift);

            // 버퍼 초기화 조건: Tab, Escape, 방향키
            if matches!(keycode, 48 | 53 | 123..=126) {
                state.buffer.lock().unwrap().clear();
                state.send_debounce_command(DebounceCommand::Cancel);
                state.send_switch_command(SwitchCommand::Cancel);
                return Some(event.clone());
            }

            // Space 또는 Enter 입력 시 자동 감지 체크
            if keycode == 49 || keycode == 36 {
                // 49 = Space, 36 = Enter
                // Debounce 취소 (한글 전환 타이머는 유지 — Space는 단어 경계이므로)
                state.send_debounce_command(DebounceCommand::Cancel);

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
                            callback(buffer_content, false); // 자동 감지
                        }
                    }

                    // 자동 변환 시 Space/Enter 이벤트를 소비 (입력되지 않음)
                    return None;
                }

                // 자동 감지 조건 미충족 시 버퍼만 초기화
                state.buffer.lock().unwrap().clear();
                return Some(event.clone());
            }

            // 문자 키 처리 - 영문 입력 모드일 때만 버퍼링
            if let Some(c) = keycode_to_char(keycode, shift_pressed) {
                // 현재 입력 소스 확인 (한글 모드면 버퍼링 안함)
                if !is_english_input_source() {
                    // 한글 입력 모드: 버퍼 클리어하고 패스스루
                    state.buffer.lock().unwrap().clear();
                    state.send_debounce_command(DebounceCommand::Cancel);
                    state.send_switch_command(SwitchCommand::Cancel);
                    return Some(event.clone());
                }

                // 한글 키인지 확인
                let is_hangul = is_hangul_key(c);

                state.buffer.lock().unwrap().push(c);

                // 타이핑 중이므로 한글 전환 타이머 취소
                state.send_switch_command(SwitchCommand::Cancel);

                // 실시간 모드에서 debounce 처리
                if state.is_realtime_mode() {
                    if is_hangul {
                        // 한글 키: debounce 타이머 리셋
                        state.send_debounce_command(DebounceCommand::Reset);
                    } else {
                        // 비한글 키 (숫자, 특수문자 등): 즉시 변환 체크 후 버퍼 유지
                        // 단, 버퍼에 한글 패턴이 있을 때만
                        let buffer_before = {
                            let buffer = state.buffer.lock().unwrap();
                            // 마지막 문자(비한글 키) 제외한 버퍼
                            let s = buffer.get();
                            if s.len() > 1 {
                                s[..s.len() - 1].to_string()
                            } else {
                                String::new()
                            }
                        };

                        if !buffer_before.is_empty() {
                            let should_convert = {
                                let detector = state.auto_detector.lock().unwrap();
                                detector.should_convert_realtime(&buffer_before)
                            };

                            if should_convert {
                                // 비한글 키 직전까지 변환
                                {
                                    let mut buffer = state.buffer.lock().unwrap();
                                    buffer.clear();
                                    buffer.push(c); // 비한글 키는 버퍼에 남김
                                }

                                log::info!(
                                    "[실시간-즉시] 변환 트리거: '{}' (비한글 키: '{}')",
                                    buffer_before,
                                    c
                                );
                                if let Some(callback) = state.on_convert.lock().unwrap().as_ref() {
                                    callback(buffer_before, false); // 실시간 즉시
                                }
                            }
                        }
                    }
                }
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
