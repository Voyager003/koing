//! CGEventTap을 사용한 키보드 이벤트 감지

use crate::detection::AutoDetector;
use crate::platform::input_source::{invalidate_input_source_cache, is_english_input_source, switch_to_korean};
use crate::platform::text_replacer::KOING_SYNTHETIC_EVENT_MARKER;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// Mutex lock with poison recovery — prevents cascading panics
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

extern "C" {
    /// macOS CoreGraphics: 이벤트 탭 활성화/비활성화
    fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);
    /// macOS CoreGraphics: 이벤트 탭 활성화 상태 확인
    fn CGEventTapIsEnabled(tap: *mut std::ffi::c_void) -> bool;
    /// macOS CoreFoundation: CFRunLoop 정지
    fn CFRunLoopStop(rl: *mut std::ffi::c_void);
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
        if self.buffer.chars().count() >= self.max_size {
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
        self.buffer.chars().count()
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebounceCommand {
    /// 타이머 리셋 (키 입력 시)
    Reset,
    /// 타이머 취소 (버퍼 클리어 시)
    Cancel,
    /// 즉시 트리거 (비한글 키 입력 시)
    Trigger,
    /// 타이머 스레드 종료
    Shutdown,
}

/// 한글 전환 타이머 명령
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwitchCommand {
    /// 타이머 시작/리셋 (자동 변환 후)
    Reset,
    /// 타이머 취소 (새 키 입력 시)
    Cancel,
    /// 타이머 스레드 종료
    Shutdown,
}

/// Condvar 기반 debounce 타이머 상태
struct DebounceTimerState {
    command: Option<DebounceCommand>,
}

/// Condvar 기반 switch 타이머 상태
struct SwitchTimerState {
    command: Option<SwitchCommand>,
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
    /// Koing 활성화 여부 (false이면 모든 이벤트를 그대로 통과)
    pub enabled: AtomicBool,
    pub auto_detector: Mutex<AutoDetector>,
    pub on_convert: Mutex<Option<Box<dyn Fn(String, bool) + Send + 'static>>>,
    /// Undo 콜백 (한글 텍스트, 원본 영문 텍스트)
    pub on_undo: Mutex<Option<Box<dyn Fn(String, String) + Send + 'static>>>,
    /// 실시간 모드 활성화 여부
    pub realtime_mode: AtomicBool,
    /// Debounce 타이머 Condvar 기반 상태
    debounce_cv: Arc<(Mutex<DebounceTimerState>, std::sync::Condvar)>,
    /// 한글 전환 타이머 Condvar 기반 상태
    switch_cv: Arc<(Mutex<SwitchTimerState>, std::sync::Condvar)>,
    /// 마지막 키 입력 시간 (ms 단위 epoch)
    pub last_key_time: AtomicU64,
    /// 변환 이력 (Undo용)
    pub conversion_history: Mutex<Option<ConversionHistory>>,
    /// 텍스트 교체 중 여부 (레이스 컨디션 방지)
    pub is_replacing: AtomicBool,
    /// debounce/실시간 변환이 버퍼를 소비한 직후 true로 설정.
    /// Space/Enter가 뒤따라 올 때 이벤트를 소비하여 race condition 방지.
    /// 새 문자 입력 시 false로 리셋.
    conversion_just_triggered: AtomicBool,
    /// 변환 감지 debounce 시간 (ms)
    pub debounce_ms: AtomicU64,
    /// 한글 자판 전환 지연 시간 (ms)
    pub switch_delay_ms: AtomicU64,
    /// 느린 변환 대기 시간 (ms) — 유효하지만 확신 낮은 한글용
    pub slow_debounce_ms: AtomicU64,
    /// CGEventTap mach port (이벤트 탭 재활성화용)
    tap_port: AtomicPtr<std::ffi::c_void>,
    /// 이벤트 탭 스레드의 CFRunLoop (정상 종료용)
    run_loop: AtomicPtr<std::ffi::c_void>,
    /// 재활성화 필요 플래그 (콜백에서 빠르게 반환 후 감시 스레드가 처리)
    needs_reenable: AtomicBool,
    /// 마지막 이벤트 수신 시간 (epoch ms, 헬스 모니터링용)
    last_event_time: AtomicU64,
}

impl EventTapState {
    pub fn new(hotkey: HotkeyConfig) -> Self {
        Self {
            buffer: Mutex::new(KeyBuffer::new(100)),
            hotkey,
            running: AtomicBool::new(true),
            enabled: AtomicBool::new(true),
            auto_detector: Mutex::new(AutoDetector::default()),
            on_convert: Mutex::new(None),
            on_undo: Mutex::new(None),
            realtime_mode: AtomicBool::new(true), // 기본 활성화
            debounce_cv: Arc::new((
                Mutex::new(DebounceTimerState { command: None }),
                std::sync::Condvar::new(),
            )),
            switch_cv: Arc::new((
                Mutex::new(SwitchTimerState { command: None }),
                std::sync::Condvar::new(),
            )),
            last_key_time: AtomicU64::new(0),
            conversion_history: Mutex::new(None),
            is_replacing: AtomicBool::new(false),
            conversion_just_triggered: AtomicBool::new(false),
            slow_debounce_ms: AtomicU64::new(1500),
            debounce_ms: AtomicU64::new(300),
            switch_delay_ms: AtomicU64::new(0),
            tap_port: AtomicPtr::new(std::ptr::null_mut()),
            run_loop: AtomicPtr::new(std::ptr::null_mut()),
            needs_reenable: AtomicBool::new(false),
            last_event_time: AtomicU64::new(0),
        }
    }

    pub fn set_convert_callback<F>(&self, callback: F)
    where
        F: Fn(String, bool) + Send + 'static,
    {
        let mut on_convert = lock_or_recover(&self.on_convert);
        *on_convert = Some(Box::new(callback));
    }

    pub fn set_undo_callback<F>(&self, callback: F)
    where
        F: Fn(String, String) + Send + 'static,
    {
        let mut on_undo = lock_or_recover(&self.on_undo);
        *on_undo = Some(Box::new(callback));
    }

    /// Koing 활성화/비활성화
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Koing 활성화 여부
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
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
        self.realtime_mode.store(enabled, Ordering::Relaxed);
    }

    /// 실시간 모드 활성화 여부
    pub fn is_realtime_mode(&self) -> bool {
        self.realtime_mode.load(Ordering::Relaxed)
    }

    /// 변환 감지 debounce 시간 설정
    pub fn set_debounce_ms(&self, ms: u64) {
        self.debounce_ms.store(ms, Ordering::Relaxed);
    }

    /// 변환 감지 debounce 시간 읽기
    pub fn get_debounce_ms(&self) -> u64 {
        self.debounce_ms.load(Ordering::Relaxed)
    }

    /// 느린 변환 대기 시간 설정
    pub fn set_slow_debounce_ms(&self, ms: u64) {
        self.slow_debounce_ms.store(ms, Ordering::Relaxed);
    }

    /// 느린 변환 대기 시간 읽기
    pub fn get_slow_debounce_ms(&self) -> u64 {
        self.slow_debounce_ms.load(Ordering::Relaxed)
    }

    /// 한글 자판 전환 지연 시간 설정
    pub fn set_switch_delay_ms(&self, ms: u64) {
        self.switch_delay_ms.store(ms, Ordering::Relaxed);
    }

    /// 한글 자판 전환 지연 시간 읽기
    pub fn get_switch_delay_ms(&self) -> u64 {
        self.switch_delay_ms.load(Ordering::Relaxed)
    }

    /// Debounce 타이머에 명령 전송 (Condvar로 즉시 깨움)
    fn send_debounce_command(&self, cmd: DebounceCommand) {
        let (ref mutex, ref cvar) = *self.debounce_cv;
        if let Ok(mut state) = mutex.lock() {
            state.command = Some(cmd);
            cvar.notify_one();
        }
    }

    /// 한글 전환 타이머에 명령 전송 (Condvar로 즉시 깨움)
    pub fn send_switch_command(&self, cmd: SwitchCommand) {
        let (ref mutex, ref cvar) = *self.switch_cv;
        if let Ok(mut state) = mutex.lock() {
            state.command = Some(cmd);
            cvar.notify_one();
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
        self.tap_port.store(port, Ordering::Release);
    }

    /// 재활성화 필요 플래그 설정 (콜백에서 호출 — 빠르게 반환)
    fn request_reenable(&self) {
        self.needs_reenable.store(true, Ordering::Release);
    }

    /// 재시도 + 검증 로직이 포함된 이벤트 탭 재활성화
    /// Sonoma/Sequoia에서는 더 많은 재시도와 딜레이를 사용
    fn reenable_tap_with_retry(&self) {
        use crate::platform::os_version::{is_sequoia_or_later, is_sonoma_or_later};

        let port = self.tap_port.load(Ordering::Acquire);
        if port.is_null() {
            return;
        }

        let (initial_delay_ms, max_retries) = if is_sequoia_or_later() {
            (50, 5)
        } else if is_sonoma_or_later() {
            (50, 3)
        } else {
            // Ventura 이하: 즉시 1회 시도
            (0, 1)
        };

        for attempt in 0..max_retries {
            let delay_ms = if attempt == 0 {
                initial_delay_ms
            } else {
                // 지수 백오프: 50, 100, 200, 400, ...
                initial_delay_ms * (1u64 << attempt)
            };

            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }

            unsafe {
                CGEventTapEnable(port, true);
            }

            // 재활성화 성공 검증 (짧은 딜레이 후 확인)
            thread::sleep(Duration::from_millis(10));
            let enabled = unsafe { CGEventTapIsEnabled(port) };
            if enabled {
                log::warn!(
                    "이벤트 탭 재활성화 성공 (시도 {}/{})",
                    attempt + 1,
                    max_retries
                );
                self.needs_reenable.store(false, Ordering::Release);
                return;
            }

            log::warn!(
                "이벤트 탭 재활성화 실패, 재시도 {}/{}",
                attempt + 1,
                max_retries
            );
        }

        log::error!("이벤트 탭 재활성화 최종 실패 ({}회 시도)", max_retries);
    }

    /// 변환 이력 가져오기 (Undo용)
    pub fn take_conversion_history(&self) -> Option<ConversionHistory> {
        if let Ok(mut history) = self.conversion_history.lock() {
            history.take()
        } else {
            None
        }
    }

    /// 이벤트 탭 스레드의 CFRunLoop 저장
    fn set_run_loop(&self, rl: *mut std::ffi::c_void) {
        self.run_loop.store(rl, Ordering::Release);
    }

    /// 이벤트 탭 종료 — 타이머 스레드 정지 + CFRunLoop 정지
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
        self.send_debounce_command(DebounceCommand::Shutdown);
        self.send_switch_command(SwitchCommand::Shutdown);
        let rl = self.run_loop.load(Ordering::Acquire);
        if !rl.is_null() {
            unsafe { CFRunLoopStop(rl); }
        }
    }
}

/// Debounce 타이머 스레드 시작 (Condvar 기반 — 정확한 타이밍)
fn start_debounce_timer(state: Arc<EventTapState>) {
    let cv = Arc::clone(&state.debounce_cv);
    let state_for_timer = Arc::clone(&state);

    thread::spawn(move || {
        let (ref mutex, ref cvar) = *cv;
        let mut deadline: Option<Instant> = None;
        // 1단계(빠른 변환) 시도 후 실패했는지 추적
        let mut fast_triggered = false;

        loop {
            let mut guard = lock_or_recover(mutex);

            // 대기: 명령이 오거나 deadline까지
            loop {
                // 명령 확인
                if let Some(cmd) = guard.command.take() {
                    match cmd {
                        DebounceCommand::Reset => {
                            deadline = Some(Instant::now());
                            fast_triggered = false;
                        }
                        DebounceCommand::Cancel => {
                            deadline = None;
                            fast_triggered = false;
                        }
                        DebounceCommand::Trigger => {
                            trigger_realtime_conversion(&state_for_timer);
                            deadline = None;
                            fast_triggered = false;
                        }
                        DebounceCommand::Shutdown => {
                            return;
                        }
                    }
                    continue; // 추가 명령이 있을 수 있으므로 재확인
                }

                // deadline 계산
                let remaining = if let Some(reset_time) = deadline {
                    let elapsed = reset_time.elapsed();
                    let target_duration = if fast_triggered {
                        Duration::from_millis(
                            state_for_timer.slow_debounce_ms.load(Ordering::Relaxed),
                        )
                    } else {
                        Duration::from_millis(
                            state_for_timer.debounce_ms.load(Ordering::Relaxed),
                        )
                    };

                    if elapsed >= target_duration {
                        // 타이머 만료
                        break;
                    }
                    target_duration - elapsed
                } else {
                    // deadline 없음 — 무한 대기
                    Duration::from_secs(3600)
                };

                let (new_guard, timeout_result) = cvar.wait_timeout(guard, remaining).unwrap_or_else(|e| {
                    let g = e.into_inner();
                    (g.0, g.1)
                });
                guard = new_guard;

                if timeout_result.timed_out() && deadline.is_some() {
                    break;
                }
            }

            // deadline이 없으면 (Cancel 상태) 루프 재시작
            if deadline.is_none() {
                continue;
            }

            // 타이머 만료 — 변환 시도
            if !fast_triggered {
                // 1단계: 높은 confidence 변환 시도
                if trigger_realtime_conversion(&state_for_timer) {
                    deadline = None;
                    fast_triggered = false;
                } else {
                    fast_triggered = true; // 1단계 실패, 2단계 대기
                    // deadline은 유지 — slow_debounce_ms까지 추가 대기
                }
            } else {
                // 2단계: 유효한 한글 구조이면 변환
                trigger_slow_conversion(&state_for_timer);
                deadline = None;
                fast_triggered = false;
            }
        }
    });
}

/// 한글 전환 타이머 스레드 시작 (Condvar 기반 — 정확한 타이밍)
/// 자동 변환 후 switch_delay_ms간 추가 입력이 없으면 한글 입력 소스로 전환
fn start_switch_timer(state: Arc<EventTapState>) {
    let cv = Arc::clone(&state.switch_cv);
    let state_for_timer = Arc::clone(&state);

    thread::spawn(move || {
        let (ref mutex, ref cvar) = *cv;
        let mut deadline: Option<Instant> = None;
        let mut switch_fired = false;

        loop {
            let mut guard = lock_or_recover(mutex);

            loop {
                // 명령 확인
                if let Some(cmd) = guard.command.take() {
                    match cmd {
                        SwitchCommand::Reset => {
                            deadline = Some(Instant::now());
                            switch_fired = false;
                        }
                        SwitchCommand::Cancel => {
                            deadline = None;
                            switch_fired = false;
                        }
                        SwitchCommand::Shutdown => {
                            return;
                        }
                    }
                    continue;
                }

                let remaining = if let Some(reset_time) = deadline {
                    let switch_delay_ms = state_for_timer.switch_delay_ms.load(Ordering::Relaxed);
                    let target = Duration::from_millis(switch_delay_ms);
                    let elapsed = reset_time.elapsed();
                    if elapsed >= target {
                        break;
                    }
                    target - elapsed
                } else {
                    Duration::from_secs(3600)
                };

                let (new_guard, timeout_result) = cvar.wait_timeout(guard, remaining).unwrap_or_else(|e| {
                    let g = e.into_inner();
                    (g.0, g.1)
                });
                guard = new_guard;

                if timeout_result.timed_out() && deadline.is_some() {
                    break;
                }
            }

            if deadline.is_none() {
                continue;
            }

            // 타이머 만료 — 한글 전환
            if !switch_fired {
                if let Err(e) = switch_to_korean() {
                    log::warn!("[SwitchTimer] 한글 전환 실패: {}", e);
                }
                switch_fired = true;
            }
            deadline = None;
        }
    });
}

/// 실시간 변환 트리거 (1단계: 높은 confidence)
/// 반환값: true이면 변환 성공, false이면 변환 조건 미충족
fn trigger_realtime_conversion(state: &EventTapState) -> bool {
    if !state.is_realtime_mode() {
        return false;
    }

    // 텍스트 교체 중이면 실시간 변환 스킵 (레이스 컨디션 방지)
    if state.is_replacing.load(Ordering::Acquire) {
        return false;
    }

    let should_convert = {
        let buffer = lock_or_recover(&state.buffer);
        if buffer.is_empty() {
            return false;
        }
        let detector = lock_or_recover(&state.auto_detector);
        detector.should_convert_realtime(buffer.get())
    };

    if should_convert {
        let buffer_content = {
            let mut buffer = lock_or_recover(&state.buffer);
            let content = buffer.get().to_string();
            buffer.clear();
            content
        };

        if !buffer_content.is_empty() {
            state
                .conversion_just_triggered
                .store(true, Ordering::Release);
            if let Some(callback) = lock_or_recover(&state.on_convert).as_ref() {
                callback(buffer_content, false); // 실시간 debounce
            }
            return true;
        }
    }
    false
}

/// 느린 변환 트리거 (2단계: 구조적 유효성 검사)
/// N-gram 점수가 낮지만 유효한 한글 구조를 가진 입력을 변환
fn trigger_slow_conversion(state: &EventTapState) -> bool {
    if !state.is_realtime_mode() {
        return false;
    }
    if state.is_replacing.load(Ordering::Acquire) {
        return false;
    }

    let buffer_content = {
        let buffer = lock_or_recover(&state.buffer);
        if buffer.is_empty() {
            return false;
        }
        buffer.get().to_string()
    };

    // 한글로 변환
    let converted = crate::core::converter::convert(&buffer_content);
    if converted == buffer_content {
        return false;
    }

    // 낱자모(미완성 자모) 포함 시 거부
    if crate::detection::validator::has_incomplete_jamo(&converted) {
        return false;
    }

    // 음절 구조 검사
    if !crate::ngram::check_syllable_structure(&converted) {
        return false;
    }

    // 한 글자 변환은 오탐 방지
    if converted.chars().count() <= 1 {
        return false;
    }

    // 최소한의 confidence 확인 (threshold 70)
    let has_min_confidence = {
        let detector = lock_or_recover(&state.auto_detector);
        detector.should_convert(&buffer_content)
    };
    if !has_min_confidence {
        return false;
    }

    // 변환 실행
    let content = {
        let mut buffer = lock_or_recover(&state.buffer);
        let c = buffer.get().to_string();
        buffer.clear();
        c
    };

    if !content.is_empty() {
        state
            .conversion_just_triggered
            .store(true, Ordering::Release);
        if let Some(callback) = lock_or_recover(&state.on_convert).as_ref() {
            callback(content, false);
        }
        return true;
    }
    false
}

/// 재활성화 감시 스레드 시작
/// needs_reenable 플래그를 폴링하여 재활성화 수행
fn start_reenable_watcher(state: Arc<EventTapState>) {
    let state_for_watcher = Arc::clone(&state);
    thread::spawn(move || {
        while state_for_watcher.running.load(Ordering::Acquire) {
            if state_for_watcher.needs_reenable.load(Ordering::Acquire) {
                state_for_watcher.reenable_tap_with_retry();
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
}

/// 이벤트 탭 헬스 모니터링 스레드 시작
/// 60초 이상 이벤트가 없으면 자동 재활성화 시도
fn start_health_monitor(state: Arc<EventTapState>) {
    let state_for_monitor = Arc::clone(&state);
    thread::spawn(move || {
        // 초기 30초 대기 (앱 시작 직후 이벤트 없는 것은 정상)
        thread::sleep(Duration::from_secs(30));

        while state_for_monitor.running.load(Ordering::Acquire) {
            thread::sleep(Duration::from_secs(30));

            let last = state_for_monitor.last_event_time.load(Ordering::Acquire);
            if last == 0 {
                // 아직 이벤트를 한 번도 받지 못함 — 스킵
                continue;
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let elapsed_sec = (now_ms.saturating_sub(last)) / 1000;
            if elapsed_sec >= 60 {
                log::warn!(
                    "헬스 모니터: {}초간 이벤트 없음, 이벤트 탭 재활성화 시도",
                    elapsed_sec
                );
                state_for_monitor.request_reenable();
            }
        }
    });
}

/// 이벤트 탭 시작
/// 반환: 성공 시 EventTapState의 Arc, 실패 시 에러 메시지
pub fn start_event_tap(state: Arc<EventTapState>) -> Result<(), String> {
    // Debounce 타이머 시작
    start_debounce_timer(Arc::clone(&state));
    // 한글 전환 타이머 시작
    start_switch_timer(Arc::clone(&state));
    // 재활성화 감시 스레드 시작
    start_reenable_watcher(Arc::clone(&state));
    // 헬스 모니터링 스레드 시작
    start_health_monitor(Arc::clone(&state));

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

        let current_loop = CFRunLoop::get_current();
        current_loop.add_source(&loop_source, kCFRunLoopCommonModes);

        // CFRunLoop 참조 저장 (stop()에서 종료용)
        use core_foundation::base::TCFType as _;
        state.set_run_loop(current_loop.as_concrete_TypeRef() as *mut std::ffi::c_void);

        tap.enable();

        // 런루프 실행 (stop() 호출 시 종료됨)
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
    if !state.running.load(Ordering::Acquire) {
        return Some(event.clone());
    }

    // Koing 비활성화 상태이면 모든 이벤트를 그대로 통과
    if !state.is_enabled() {
        return Some(event.clone());
    }

    // 마지막 이벤트 수신 시간 업데이트 (헬스 모니터링용)
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    state.last_event_time.store(now_ms, Ordering::Release);

    // macOS가 이벤트 탭을 비활성화했으면 재활성화 요청
    // 콜백에서 직접 재시도하지 않고, 감시 스레드가 처리하도록 플래그만 설정
    if matches!(
        event_type,
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
    ) {
        log::warn!("이벤트 탭 비활성화 감지: {:?}", event_type);
        state.request_reenable();
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
            // 텍스트 교체 중이면 연타 방지
            if keycode == 6 && option_pressed && !state.is_replacing.load(Ordering::Acquire) {
                // 6 = Z key
                if let Some(history) = state.take_conversion_history() {
                    // Undo 콜백 호출 (원본 텍스트로 복원)
                    if let Some(callback) = lock_or_recover(&state.on_undo).as_ref() {
                        callback(history.converted, history.original);
                    }
                    return None;
                }
                return Some(event.clone());
            }

            // 단축키 체크 (Option + Space)
            // 텍스트 교체 중이면 연타 방지
            if keycode == state.hotkey.trigger_keycode
                && state.hotkey.require_option
                && option_pressed
                && !state.is_replacing.load(Ordering::Acquire)
            {
                // Debounce 및 한글 전환 타이머 취소 (수동 전환이므로 즉시 전환됨)
                state.send_debounce_command(DebounceCommand::Cancel);
                state.send_switch_command(SwitchCommand::Cancel);

                // 변환 트리거
                let buffer_content = {
                    let mut buffer = lock_or_recover(&state.buffer);
                    let content = buffer.get().to_string();
                    buffer.clear();
                    content
                };

                if !buffer_content.is_empty() {
                    if let Some(callback) = lock_or_recover(&state.on_convert).as_ref() {
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
                lock_or_recover(&state.buffer).clear();
                state.send_debounce_command(DebounceCommand::Cancel);
                state.send_switch_command(SwitchCommand::Cancel);
                return Some(event.clone());
            }

            // Space 입력 시: 버퍼 초기화 (변환 트리거 없이 통과)
            if keycode == 49 {
                state.send_debounce_command(DebounceCommand::Cancel);
                // debounce가 직전에 버퍼를 소비했다면 Space 소비
                if state
                    .conversion_just_triggered
                    .swap(false, Ordering::AcqRel)
                {
                    lock_or_recover(&state.buffer).clear();
                    return None;
                }
                lock_or_recover(&state.buffer).clear();
                return Some(event.clone());
            }

            // Enter 입력 시 버퍼 초기화 (자동 변환 비활성화)
            if keycode == 36 {
                state.send_debounce_command(DebounceCommand::Cancel);

                // debounce가 직전에 버퍼를 소비했다면 Enter 소비
                if state
                    .conversion_just_triggered
                    .swap(false, Ordering::AcqRel)
                {
                    lock_or_recover(&state.buffer).clear();
                    return None;
                }

                lock_or_recover(&state.buffer).clear();
                return Some(event.clone());
            }

            // 문자 키 처리 - 영문 입력 모드일 때만 버퍼링
            if let Some(c) = keycode_to_char(keycode, shift_pressed) {
                // 현재 입력 소스 확인 (한글 모드면 버퍼링 안함)
                if !is_english_input_source() {
                    // 한글 입력 모드: 버퍼 클리어하고 패스스루
                    lock_or_recover(&state.buffer).clear();
                    state.send_debounce_command(DebounceCommand::Cancel);
                    state.send_switch_command(SwitchCommand::Cancel);
                    return Some(event.clone());
                }

                // 한글 키인지 확인
                let is_hangul = is_hangul_key(c);

                state
                    .conversion_just_triggered
                    .store(false, Ordering::Relaxed);
                lock_or_recover(&state.buffer).push(c);

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
                            let buffer = lock_or_recover(&state.buffer);
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
                                let detector = lock_or_recover(&state.auto_detector);
                                detector.should_convert_realtime(&buffer_before)
                            };

                            if should_convert {
                                // 비한글 키 직전까지 변환
                                {
                                    let mut buffer = lock_or_recover(&state.buffer);
                                    buffer.clear();
                                    buffer.push(c); // 비한글 키는 버퍼에 남김
                                }

                                state
                                    .conversion_just_triggered
                                    .store(true, Ordering::Release);
                                if let Some(callback) = lock_or_recover(&state.on_convert).as_ref() {
                                    callback(buffer_before, false); // 실시간 즉시
                                }
                            }
                        }
                    }
                }
            }

            Some(event.clone())
        }
        CGEventType::FlagsChanged => {
            // 수정키 변경 시 입력 소스 캐시 무효화
            invalidate_input_source_cache();

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
