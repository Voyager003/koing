//! 입력 소스 인디케이터 오버레이 윈도우
//! 한영 전환 시 커서 옆에 "한" / "A" 표시

#![allow(deprecated)]

use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Mutex;
use std::time::Instant;

/// ObjC id wrapper for Send/Sync (all access on main thread)
#[derive(Clone, Copy)]
struct SendId(id);
unsafe impl Send for SendId {}
unsafe impl Sync for SendId {}

/// 인디케이터 윈도우의 전역 상태
struct IndicatorState {
    /// NSWindow 참조 (재사용)
    window: SendId,
    /// 텍스트 NSTextField 참조
    label: SendId,
    /// 마지막 표시 시간 (자동 숨기기용)
    last_shown: Option<Instant>,
    /// 현재 실행 중인 fade-out 타이머의 generation
    timer_generation: u64,
}

// IndicatorState contains SendId which is Send+Sync
unsafe impl Send for IndicatorState {}

static INDICATOR: Mutex<Option<IndicatorState>> = Mutex::new(None);
/// 타이머 generation (fade-out 취소용)
static TIMER_GEN: Mutex<u64> = Mutex::new(0);

const INDICATOR_SIZE: f64 = 28.0;
const FADE_DELAY_SECS: f64 = 1.5;
const OFFSET_X: f64 = 8.0;
const OFFSET_Y: f64 = 4.0;

/// 인디케이터 윈도우를 생성하거나 재사용하여 표시합니다.
/// **반드시 메인 스레드에서 호출해야 합니다.**
pub fn show_indicator(text: &str, x: f64, y: f64) {
    let mut guard = INDICATOR.lock().unwrap_or_else(|e| e.into_inner());

    if guard.is_none() {
        let (window, label) = create_indicator_window();
        *guard = Some(IndicatorState {
            window: SendId(window),
            label: SendId(label),
            last_shown: None,
            timer_generation: 0,
        });
    }

    let state = guard.as_mut().unwrap();

    unsafe {
        // 텍스트 업데이트
        let ns_text = NSString::alloc(nil).init_str(text);
        let _: () = msg_send![state.label.0, setStringValue: ns_text];

        // 위치 업데이트 (커서 오른쪽 아래)
        let frame = NSRect::new(
            NSPoint::new(x + OFFSET_X, screen_flip_y(y + OFFSET_Y)),
            NSSize::new(INDICATOR_SIZE, INDICATOR_SIZE),
        );
        let _: () = msg_send![state.window.0, setFrame: frame display: NO];

        // 완전 불투명으로 표시
        let _: () = msg_send![state.window.0, setAlphaValue: 1.0f64];
        let _: () = msg_send![state.window.0, orderFrontRegardless];
    }

    // 타이머 generation 증가 (이전 fade-out 타이머 무효화)
    state.timer_generation += 1;
    let gen = state.timer_generation;
    state.last_shown = Some(Instant::now());

    {
        let mut tg = TIMER_GEN.lock().unwrap_or_else(|e| e.into_inner());
        *tg = gen;
    }

    // 자동 숨기기 타이머 예약
    schedule_fade_out(gen);
}

/// 인디케이터를 즉시 숨깁니다.
/// **반드시 메인 스레드에서 호출해야 합니다.**
pub fn hide_indicator() {
    let mut guard = INDICATOR.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = guard.as_mut() {
        // 타이머 무효화
        state.timer_generation += 1;
        {
            let mut tg = TIMER_GEN.lock().unwrap_or_else(|e| e.into_inner());
            *tg = state.timer_generation;
        }

        unsafe {
            let _: () = msg_send![state.window.0, orderOut: nil];
        }
    }
}

/// macOS 좌표계 변환 (상→하 기준 y를 하→상 기준으로)
fn screen_flip_y(y: f64) -> f64 {
    unsafe {
        let main_screen: id = msg_send![class!(NSScreen), mainScreen];
        let frame: NSRect = msg_send![main_screen, frame];
        frame.size.height - y - INDICATOR_SIZE
    }
}

/// 인디케이터 윈도우를 생성합니다.
fn create_indicator_window() -> (id, id) {
    unsafe {
        // NSWindow 레벨 상수
        // kCGStatusWindowLevel = 25 (CGWindowLevelKey)
        let status_window_level: i64 = 25;

        // NSPanel 생성 (borderless)
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(INDICATOR_SIZE, INDICATOR_SIZE),
        );

        // NSWindowStyleMaskBorderless = 0
        let window: id = msg_send![class!(NSPanel), alloc];
        let window: id = msg_send![window,
            initWithContentRect: frame
            styleMask: 0u64
            backing: 2u64  // NSBackingStoreBuffered
            defer: NO
        ];

        // 윈도우 속성 설정
        let _: () = msg_send![window, setLevel: status_window_level];
        let _: () = msg_send![window, setOpaque: NO];
        let _: () = msg_send![window, setHasShadow: NO];
        let _: () = msg_send![window, setIgnoresMouseEvents: cocoa::base::YES];
        let _: () = msg_send![window, setCollectionBehavior: 1u64 << 4]; // canJoinAllSpaces
        // 포커스를 뺏지 않음
        // NSPanel의 becomesKeyOnlyIfNeeded를 설정하는 대신,
        // 단순히 key/main window가 되지 않게 함
        let _: () = msg_send![window, setHidesOnDeactivate: NO];

        // 배경 뷰: 반투명 검정 + cornerRadius
        let content_view: id = msg_send![window, contentView];
        let _: () = msg_send![content_view, setWantsLayer: cocoa::base::YES];
        let layer: id = msg_send![content_view, layer];
        let bg_color: id = msg_send![class!(NSColor),
            colorWithRed: 0.0f64
            green: 0.0f64
            blue: 0.0f64
            alpha: 0.7f64
        ];
        let cg_color: *mut std::ffi::c_void = msg_send![bg_color, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: cg_color];
        let _: () = msg_send![layer, setCornerRadius: 6.0f64];

        // 텍스트 라벨
        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, initWithFrame: frame];
        let _: () = msg_send![label, setBezeled: NO];
        let _: () = msg_send![label, setDrawsBackground: NO];
        let _: () = msg_send![label, setEditable: NO];
        let _: () = msg_send![label, setSelectable: NO];
        let _: () = msg_send![label, setAlignment: 2u64]; // NSTextAlignmentCenter

        // 흰색 텍스트
        let white: id = msg_send![class!(NSColor), whiteColor];
        let _: () = msg_send![label, setTextColor: white];

        // 시스템 폰트 14pt
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 14.0f64];
        let _: () = msg_send![label, setFont: font];

        let _: () = msg_send![content_view, addSubview: label];

        (window, label)
    }
}

/// 일정 시간 후 fade-out을 예약합니다.
fn schedule_fade_out(generation: u64) {
    // GCD dispatch_after를 사용
    extern "C" {
        static _dispatch_main_q: std::ffi::c_void;
        fn dispatch_after_f(
            when: u64,
            queue: *const std::ffi::c_void,
            context: *mut std::ffi::c_void,
            work: extern "C" fn(*mut std::ffi::c_void),
        );
        fn dispatch_time(when: u64, delta: i64) -> u64;
    }

    extern "C" fn fade_out_callback(context: *mut std::ffi::c_void) {
        let gen = context as u64;

        // generation이 일치하지 않으면 이미 새 표시/숨기기가 발생한 것
        let current_gen = {
            let tg = TIMER_GEN.lock().unwrap_or_else(|e| e.into_inner());
            *tg
        };
        if gen != current_gen {
            return;
        }

        // 윈도우 숨기기
        let mut guard = INDICATOR.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.as_mut() {
            unsafe {
                let _: () = msg_send![state.window.0, orderOut: nil];
            }
        }
    }

    unsafe {
        let delay_ns = (FADE_DELAY_SECS * 1_000_000_000.0) as i64;
        // DISPATCH_TIME_NOW = 0
        let when = dispatch_time(0, delay_ns);
        let queue = &_dispatch_main_q as *const std::ffi::c_void;
        dispatch_after_f(
            when,
            queue,
            generation as *mut std::ffi::c_void,
            fade_out_callback,
        );
    }
}
