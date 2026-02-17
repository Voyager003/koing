//! 설정 윈도우 (NSWindow)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use crate::config::save_config;
use crate::ui::menubar::{current_config, update_toggle_state};
use cocoa::appkit::{NSApp, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::{Mutex, OnceLock};

use super::menubar::EVENT_STATE;
use super::{
    DEBOUNCE_LABELS, DEBOUNCE_PRESETS,
    SLOW_DEBOUNCE_LABELS, SLOW_DEBOUNCE_PRESETS,
    SWITCH_LABELS, SWITCH_PRESETS,
};

/// 설정 윈도우 참조 (재사용)
struct SendId(id);
unsafe impl Send for SendId {}
unsafe impl Sync for SendId {}

static SETTINGS_WINDOW: Mutex<Option<SendId>> = Mutex::new(None);
/// delegate 참조를 유지하여 해제 방지 (NSControl.target은 unretained)
static SETTINGS_DELEGATE: Mutex<Option<SendId>> = Mutex::new(None);
static SETTINGS_DELEGATE_CLASS: OnceLock<&'static Class> = OnceLock::new();

// --- ObjC 액션 핸들러 ---

extern "C" fn toggle_enabled_action(_: &Object, _: Sel, sender: id) {
    let Some(state) = EVENT_STATE.get() else { return };
    unsafe {
        let checked: cocoa::foundation::NSInteger = msg_send![sender, state];
        let new_enabled = checked == 0; // 0=unchecked→enable, 1=checked→disable
        let new_state: cocoa::foundation::NSInteger = if new_enabled { 1 } else { 0 };
        let _: () = msg_send![sender, setState: new_state];

        state.set_enabled(new_enabled);
        update_toggle_state(new_enabled);

        let config = current_config();
        if let Err(e) = save_config(&config) {
            log::error!("설정 저장 실패: {}", e);
        }
    }
}

extern "C" fn debounce_changed(_: &Object, _: Sel, sender: id) {
    let Some(state) = EVENT_STATE.get() else { return };
    unsafe {
        let index: cocoa::foundation::NSInteger = msg_send![sender, indexOfSelectedItem];
        if (index as usize) < DEBOUNCE_PRESETS.len() {
            let ms = DEBOUNCE_PRESETS[index as usize];
            state.set_debounce_ms(ms);

            let config = current_config();
            if let Err(e) = save_config(&config) {
                log::error!("설정 저장 실패: {}", e);
            }
        }
    }
}

extern "C" fn switch_changed(_: &Object, _: Sel, sender: id) {
    let Some(state) = EVENT_STATE.get() else { return };
    unsafe {
        let index: cocoa::foundation::NSInteger = msg_send![sender, indexOfSelectedItem];
        if (index as usize) < SWITCH_PRESETS.len() {
            let ms = SWITCH_PRESETS[index as usize];
            state.set_switch_delay_ms(ms);

            let config = current_config();
            if let Err(e) = save_config(&config) {
                log::error!("설정 저장 실패: {}", e);
            }
        }
    }
}

extern "C" fn slow_debounce_changed(_: &Object, _: Sel, sender: id) {
    let Some(state) = EVENT_STATE.get() else { return };
    unsafe {
        let index: cocoa::foundation::NSInteger = msg_send![sender, indexOfSelectedItem];
        if (index as usize) < SLOW_DEBOUNCE_PRESETS.len() {
            let ms = SLOW_DEBOUNCE_PRESETS[index as usize];
            state.set_slow_debounce_ms(ms);

            let config = current_config();
            if let Err(e) = save_config(&config) {
                log::error!("설정 저장 실패: {}", e);
            }
        }
    }
}

fn get_delegate_class() -> &'static Class {
    SETTINGS_DELEGATE_CLASS.get_or_init(|| {
        let superclass = class!(NSObject);
        match ClassDecl::new("KoingSettingsDelegate", superclass) {
            Some(mut decl) => {
                type ActionFn = extern "C" fn(&Object, Sel, id);

                unsafe {
                    decl.add_method(sel!(toggleEnabled:), toggle_enabled_action as ActionFn);
                    decl.add_method(sel!(debounceChanged:), debounce_changed as ActionFn);
                    decl.add_method(sel!(switchChanged:), switch_changed as ActionFn);
                    decl.add_method(sel!(slowDebounceChanged:), slow_debounce_changed as ActionFn);
                }

                decl.register()
            }
            None => {
                // 클래스가 이미 등록됨 (재사용)
                Class::get("KoingSettingsDelegate").expect("KoingSettingsDelegate class not found")
            }
        }
    })
}

/// 설정 윈도우 표시 (없으면 생성, 있으면 앞으로 가져오기)
pub fn show_settings_window() {
    // 설정 윈도우를 열 때 대기 중인 변환 타이머를 취소하여
    // 합성 이벤트(backspace+paste)가 설정 윈도우에 전송되는 것을 방지
    if let Some(state) = EVENT_STATE.get() {
        state.cancel_pending_conversion();
    }

    let mut window_guard = SETTINGS_WINDOW.lock().unwrap_or_else(|e| e.into_inner());

    // 기존 윈도우가 있으면 앞으로 가져오기
    if let Some(ref win) = *window_guard {
        unsafe {
            let is_visible: bool = msg_send![win.0, isVisible];
            if is_visible {
                let _: () = msg_send![win.0, makeKeyAndOrderFront: nil];
                let app: id = NSApp();
                let _: () = msg_send![app, activateIgnoringOtherApps: YES];
                return;
            }
            // 닫혀있으면 이전 윈도우 해제 후 새로 생성 (현재 설정 반영)
            let _: () = msg_send![win.0, close];
        }
        *window_guard = None;
    }

    unsafe {
        let config = current_config();

        let delegate_class = get_delegate_class();
        let delegate: id = msg_send![delegate_class, new];

        // delegate 참조를 전역에 저장 (NSControl.target은 unretained — delegate 해제 방지)
        {
            let mut dg = SETTINGS_DELEGATE.lock().unwrap_or_else(|e| e.into_inner());
            *dg = Some(SendId(delegate));
        }

        // 윈도우 생성
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 330.0));
        let style = NSWindowStyleMask::NSTitledWindowMask
            | NSWindowStyleMask::NSClosableWindowMask;
        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            rect,
            style,
            cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );
        let _: () = msg_send![window, center];
        let _: () = msg_send![window, setTitle: NSString::alloc(nil).init_str("Koing 설정")];
        let _: () = msg_send![window, setReleasedWhenClosed: NO];

        let content_view: id = msg_send![window, contentView];

        // --- "Koing 활성화" 체크박스 ---
        let checkbox = create_checkbox(
            "Koing 활성화",
            NSRect::new(NSPoint::new(30.0, 260.0), NSSize::new(200.0, 24.0)),
            config.enabled,
            delegate,
            sel!(toggleEnabled:),
        );
        let _: () = msg_send![content_view, addSubview: checkbox];

        // --- 구분선 ---
        let separator = create_separator(
            NSRect::new(NSPoint::new(20.0, 245.0), NSSize::new(360.0, 1.0)),
        );
        let _: () = msg_send![content_view, addSubview: separator];

        // --- "변환 속도" 라벨 + 팝업 버튼 ---
        let debounce_label = create_label(
            "변환 속도",
            NSRect::new(NSPoint::new(30.0, 205.0), NSSize::new(120.0, 20.0)),
        );
        let _: () = msg_send![content_view, addSubview: debounce_label];

        let debounce_popup = create_popup_button(
            &DEBOUNCE_LABELS,
            NSRect::new(NSPoint::new(160.0, 202.0), NSSize::new(200.0, 26.0)),
            DEBOUNCE_PRESETS.iter().position(|&v| v == config.debounce_ms).unwrap_or(1),
            delegate,
            sel!(debounceChanged:),
        );
        let _: () = msg_send![content_view, addSubview: debounce_popup];

        // --- "느린 변환 속도" 라벨 + 팝업 버튼 ---
        let slow_debounce_label = create_label(
            "느린 변환 속도",
            NSRect::new(NSPoint::new(30.0, 160.0), NSSize::new(120.0, 20.0)),
        );
        let _: () = msg_send![content_view, addSubview: slow_debounce_label];

        let slow_debounce_popup = create_popup_button(
            &SLOW_DEBOUNCE_LABELS,
            NSRect::new(NSPoint::new(160.0, 157.0), NSSize::new(200.0, 26.0)),
            SLOW_DEBOUNCE_PRESETS.iter().position(|&v| v == config.slow_debounce_ms).unwrap_or(1),
            delegate,
            sel!(slowDebounceChanged:),
        );
        let _: () = msg_send![content_view, addSubview: slow_debounce_popup];

        // --- "자판 전환 지연" 라벨 + 팝업 버튼 ---
        let switch_label = create_label(
            "자판 전환 지연",
            NSRect::new(NSPoint::new(30.0, 115.0), NSSize::new(120.0, 20.0)),
        );
        let _: () = msg_send![content_view, addSubview: switch_label];

        let switch_popup = create_popup_button(
            &SWITCH_LABELS,
            NSRect::new(NSPoint::new(160.0, 112.0), NSSize::new(200.0, 26.0)),
            SWITCH_PRESETS.iter().position(|&v| v == config.switch_delay_ms).unwrap_or(0),
            delegate,
            sel!(switchChanged:),
        );
        let _: () = msg_send![content_view, addSubview: switch_popup];

        // --- 단축키 안내 ---
        let hotkey_label = create_label(
            "단축키: ⌥ Space (변환)  ⌥ Z (되돌리기)",
            NSRect::new(NSPoint::new(30.0, 50.0), NSSize::new(340.0, 20.0)),
        );
        let _: () = msg_send![hotkey_label, setTextColor: {
            let color: id = msg_send![class!(NSColor), secondaryLabelColor];
            color
        }];
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 11.0f64];
        let _: () = msg_send![hotkey_label, setFont: font];
        let _: () = msg_send![content_view, addSubview: hotkey_label];

        // 윈도우 표시
        let _: () = msg_send![window, makeKeyAndOrderFront: nil];
        let app: id = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];

        *window_guard = Some(SendId(window));
    }
}

// --- UI 헬퍼 함수들 ---

unsafe fn create_checkbox(title: &str, frame: NSRect, checked: bool, target: id, action: Sel) -> id {
    let button: id = msg_send![class!(NSButton), alloc];
    let button: id = msg_send![button, initWithFrame: frame];
    let _: () = msg_send![button, setButtonType: 3i64]; // NSSwitchButton
    let _: () = msg_send![button, setTitle: NSString::alloc(nil).init_str(title)];
    let state: cocoa::foundation::NSInteger = if checked { 1 } else { 0 };
    let _: () = msg_send![button, setState: state];
    let _: () = msg_send![button, setTarget: target];
    let _: () = msg_send![button, setAction: action];
    button
}

unsafe fn create_label(text: &str, frame: NSRect) -> id {
    let label: id = msg_send![class!(NSTextField), alloc];
    let label: id = msg_send![label, initWithFrame: frame];
    let _: () = msg_send![label, setStringValue: NSString::alloc(nil).init_str(text)];
    let _: () = msg_send![label, setBezeled: NO];
    let _: () = msg_send![label, setDrawsBackground: NO];
    let _: () = msg_send![label, setEditable: NO];
    let _: () = msg_send![label, setSelectable: NO];
    label
}

unsafe fn create_popup_button(
    labels: &[&str],
    frame: NSRect,
    selected_index: usize,
    target: id,
    action: Sel,
) -> id {
    let popup: id = msg_send![class!(NSPopUpButton), alloc];
    let popup: id = msg_send![popup, initWithFrame: frame pullsDown: NO];
    for label in labels {
        let _: () = msg_send![popup, addItemWithTitle: NSString::alloc(nil).init_str(label)];
    }
    let _: () = msg_send![popup, selectItemAtIndex: selected_index as cocoa::foundation::NSInteger];
    let _: () = msg_send![popup, setTarget: target];
    let _: () = msg_send![popup, setAction: action];
    popup
}

unsafe fn create_separator(frame: NSRect) -> id {
    let separator: id = msg_send![class!(NSBox), alloc];
    let separator: id = msg_send![separator, initWithFrame: frame];
    let _: () = msg_send![separator, setBoxType: 2i64]; // NSBoxSeparator
    separator
}
