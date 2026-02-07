//! macOS 메뉴바 앱 (NSStatusBar)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use crate::config::{save_config, KoingConfig};
use crate::platform::event_tap::EventTapState;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSMenuItem, NSStatusBar,
    NSStatusItem, NSVariableStatusItemLength,
};
use cocoa::base::{id, nil, selector, NO};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 메뉴바 앱 상태
pub struct MenuBarApp {
    status_item: id,
    running: Arc<AtomicBool>,
}

// 전역 상태 포인터 (ObjC 콜백에서 접근용)
static mut SHOULD_QUIT: bool = false;
static mut EVENT_STATE_PTR: *const EventTapState = std::ptr::null();

// --- 변환 속도 (debounce) ---
static mut DEBOUNCE_MENU_ITEMS: [id; 4] = [0 as id; 4];
const DEBOUNCE_PRESETS: [u64; 4] = [200, 300, 500, 800];
const DEBOUNCE_LABELS: [&str; 4] = [
    "빠름 (200ms)",
    "보통 (300ms)",
    "느림 (500ms)",
    "여유 (800ms)",
];

// --- 자판 전환 (switch) ---
static mut SWITCH_MENU_ITEMS: [id; 4] = [0 as id; 4];
const SWITCH_PRESETS: [u64; 4] = [0, 10, 30, 50];
const SWITCH_LABELS: [&str; 4] = [
    "즉시 (0ms)",
    "빠름 (10ms)",
    "보통 (30ms)",
    "느림 (50ms)",
];

/// 현재 설정 읽어서 KoingConfig 구성
unsafe fn current_config() -> KoingConfig {
    if EVENT_STATE_PTR.is_null() {
        return KoingConfig::default();
    }
    let state = &*EVENT_STATE_PTR;
    KoingConfig {
        debounce_ms: state.get_debounce_ms(),
        switch_delay_ms: state.get_switch_delay_ms(),
    }
}

unsafe fn update_checkmarks(items: *const [id; 4], presets: &[u64; 4], selected: u64) {
    let items = &*items;
    for (i, &preset) in presets.iter().enumerate() {
        let item = items[i];
        if !item.is_null() {
            let s: cocoa::foundation::NSInteger = if preset == selected { 1 } else { 0 };
            let _: () = msg_send![item, setState: s];
        }
    }
}

unsafe fn set_debounce(ms: u64) {
    if EVENT_STATE_PTR.is_null() {
        return;
    }
    let state = &*EVENT_STATE_PTR;
    state.set_debounce_ms(ms);
    update_checkmarks(&raw const DEBOUNCE_MENU_ITEMS, &DEBOUNCE_PRESETS, ms);

    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
    log::info!("변환 속도 변경: {}ms", ms);
}

unsafe fn set_switch(ms: u64) {
    if EVENT_STATE_PTR.is_null() {
        return;
    }
    let state = &*EVENT_STATE_PTR;
    state.set_switch_delay_ms(ms);
    update_checkmarks(&raw const SWITCH_MENU_ITEMS, &SWITCH_PRESETS, ms);

    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
    log::info!("자판 전환 변경: {}ms", ms);
}

// --- ObjC 액션 핸들러 ---

extern "C" fn quit_action(_this: &Object, _cmd: Sel, _sender: id) {
    unsafe {
        SHOULD_QUIT = true;
        let app: id = NSApp();
        let _: () = msg_send![app, terminate: nil];
    }
}

extern "C" fn set_debounce_200(_: &Object, _: Sel, _: id) { unsafe { set_debounce(200) }; }
extern "C" fn set_debounce_300(_: &Object, _: Sel, _: id) { unsafe { set_debounce(300) }; }
extern "C" fn set_debounce_500(_: &Object, _: Sel, _: id) { unsafe { set_debounce(500) }; }
extern "C" fn set_debounce_800(_: &Object, _: Sel, _: id) { unsafe { set_debounce(800) }; }

extern "C" fn set_switch_0(_: &Object, _: Sel, _: id)    { unsafe { set_switch(0) }; }
extern "C" fn set_switch_10(_: &Object, _: Sel, _: id)   { unsafe { set_switch(10) }; }
extern "C" fn set_switch_30(_: &Object, _: Sel, _: id)   { unsafe { set_switch(30) }; }
extern "C" fn set_switch_50(_: &Object, _: Sel, _: id)   { unsafe { set_switch(50) }; }

fn create_app_delegate_class() -> &'static Class {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("KoingAppDelegate", superclass).unwrap();

    type ActionFn = extern "C" fn(&Object, Sel, id);

    unsafe {
        decl.add_method(sel!(quitApp:), quit_action as ActionFn);
        decl.add_method(sel!(setDebounce200:), set_debounce_200 as ActionFn);
        decl.add_method(sel!(setDebounce300:), set_debounce_300 as ActionFn);
        decl.add_method(sel!(setDebounce500:), set_debounce_500 as ActionFn);
        decl.add_method(sel!(setDebounce800:), set_debounce_800 as ActionFn);
        decl.add_method(sel!(setSwitch0:), set_switch_0 as ActionFn);
        decl.add_method(sel!(setSwitch10:), set_switch_10 as ActionFn);
        decl.add_method(sel!(setSwitch30:), set_switch_30 as ActionFn);
        decl.add_method(sel!(setSwitch50:), set_switch_50 as ActionFn);
    }

    decl.register()
}

/// 서브메뉴 생성 헬퍼
unsafe fn build_submenu(
    title: &str,
    labels: &[&str; 4],
    selectors: [Sel; 4],
    presets: &[u64; 4],
    current: u64,
    items_out: *mut [id; 4],
    delegate: id,
) -> id {
    let menu_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
        NSString::alloc(nil).init_str(title),
        selector(""),
        NSString::alloc(nil).init_str(""),
    );
    let submenu = NSMenu::new(nil).autorelease();
    let _: () = msg_send![submenu, setTitle: NSString::alloc(nil).init_str(title)];

    for (i, (&label, &sel)) in labels.iter().zip(selectors.iter()).enumerate() {
        let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            NSString::alloc(nil).init_str(label),
            sel,
            NSString::alloc(nil).init_str(""),
        );
        let _: () = msg_send![item, setTarget: delegate];
        if presets[i] == current {
            let _: () = msg_send![item, setState: 1i64];
        }
        (*items_out)[i] = item;
        submenu.addItem_(item);
    }

    let _: () = msg_send![menu_item, setSubmenu: submenu];
    menu_item
}

impl MenuBarApp {
    pub fn new(running: Arc<AtomicBool>, event_state: Arc<EventTapState>) -> Self {
        unsafe {
            EVENT_STATE_PTR = Arc::as_ptr(&event_state);
        }

        let cur_debounce = event_state.get_debounce_ms();
        let cur_switch = event_state.get_switch_delay_ms();

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let app = NSApp();
            app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

            let title = NSString::alloc(nil).init_str("코");
            let _: () = msg_send![status_item, setTitle: title];

            let menu = NSMenu::new(nil).autorelease();

            let delegate_class = create_app_delegate_class();
            let delegate: id = msg_send![delegate_class, new];

            // Koing v0.1 (비활성)
            let version_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("Koing v0.1"),
                selector(""),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![version_item, setEnabled: NO];
            menu.addItem_(version_item);

            // 단축키 안내 (비활성)
            let hotkey_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("단축키: ⌥ Space"),
                selector(""),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![hotkey_item, setEnabled: NO];
            menu.addItem_(hotkey_item);

            menu.addItem_(NSMenuItem::separatorItem(nil));

            // 변환 속도 서브메뉴
            let debounce_item = build_submenu(
                "변환 속도",
                &DEBOUNCE_LABELS,
                [
                    sel!(setDebounce200:),
                    sel!(setDebounce300:),
                    sel!(setDebounce500:),
                    sel!(setDebounce800:),
                ],
                &DEBOUNCE_PRESETS,
                cur_debounce,
                &raw mut DEBOUNCE_MENU_ITEMS,
                delegate,
            );
            menu.addItem_(debounce_item);

            // 자판 전환 서브메뉴
            let switch_item = build_submenu(
                "자판 전환",
                &SWITCH_LABELS,
                [
                    sel!(setSwitch0:),
                    sel!(setSwitch10:),
                    sel!(setSwitch30:),
                    sel!(setSwitch50:),
                ],
                &SWITCH_PRESETS,
                cur_switch,
                &raw mut SWITCH_MENU_ITEMS,
                delegate,
            );
            menu.addItem_(switch_item);

            menu.addItem_(NSMenuItem::separatorItem(nil));

            // 종료
            let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("종료"),
                sel!(quitApp:),
                NSString::alloc(nil).init_str("q"),
            );
            let _: () = msg_send![quit_item, setTarget: delegate];
            menu.addItem_(quit_item);

            status_item.setMenu_(menu);

            Self {
                status_item,
                running,
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let ns_title = NSString::alloc(nil).init_str(title);
            let _: () = msg_send![self.status_item, setTitle: ns_title];
        }
    }

    pub fn run(&self) {
        unsafe {
            let app = NSApp();
            app.run();
        }
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for MenuBarApp {
    fn drop(&mut self) {
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let _: () = msg_send![status_bar, removeStatusItem: self.status_item];
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
