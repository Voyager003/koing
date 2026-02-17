//! macOS 메뉴바 앱 (NSStatusBar)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

use crate::config::{load_config, save_config, KoingConfig};
use crate::platform::event_tap::EventTapState;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSMenuItem, NSStatusBar,
    NSStatusItem, NSVariableStatusItemLength,
};
use cocoa::base::{id, nil, selector, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// 메뉴바 아이콘 데이터 (컴파일 타임 임베딩)
const MENUBAR_ICON_DATA: &[u8] = include_bytes!("../../resources/menubar_icon.png");

/// 메뉴바 앱 상태
pub struct MenuBarApp {
    status_item: id,
    running: Arc<AtomicBool>,
}

/// ObjC id wrapper for Send/Sync (all access is on the main thread via ObjC callbacks)
#[derive(Clone, Copy)]
struct SendId(id);
unsafe impl Send for SendId {}
unsafe impl Sync for SendId {}

impl SendId {
    const NULL: Self = SendId(0 as id);
}

// 전역 상태 (ObjC 콜백에서 접근용) — static mut 제거
static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
pub static EVENT_STATE: OnceLock<Arc<EventTapState>> = OnceLock::new();
/// 메뉴바 status_item (아이콘 알파 변경용)
static STATUS_ITEM: Mutex<SendId> = Mutex::new(SendId::NULL);
/// "Koing 활성화" 토글 메뉴 아이템
static TOGGLE_MENU_ITEM: Mutex<SendId> = Mutex::new(SendId::NULL);
static DEBOUNCE_MENU_ITEMS: Mutex<[SendId; 4]> = Mutex::new([SendId::NULL; 4]);
static SWITCH_MENU_ITEMS: Mutex<[SendId; 4]> = Mutex::new([SendId::NULL; 4]);
static SLOW_DEBOUNCE_MENU_ITEMS: Mutex<[SendId; 4]> = Mutex::new([SendId::NULL; 4]);

use super::{
    DEBOUNCE_LABELS, DEBOUNCE_PRESETS,
    SLOW_DEBOUNCE_LABELS, SLOW_DEBOUNCE_PRESETS,
    SWITCH_LABELS, SWITCH_PRESETS,
};

/// 현재 설정 읽어서 KoingConfig 구성
pub fn current_config() -> KoingConfig {
    match EVENT_STATE.get() {
        Some(state) => {
            let mut config = load_config();
            config.enabled = state.is_enabled();
            config.debounce_ms = state.get_debounce_ms();
            config.switch_delay_ms = state.get_switch_delay_ms();
            config.slow_debounce_ms = state.get_slow_debounce_ms();
            config
        }
        None => KoingConfig::default(),
    }
}

fn update_checkmarks(menu_items: &Mutex<[SendId; 4]>, presets: &[u64; 4], selected: u64) {
    let items = menu_items.lock().unwrap_or_else(|e| e.into_inner());
    for (i, &preset) in presets.iter().enumerate() {
        let item = items[i].0;
        if !item.is_null() {
            let s: cocoa::foundation::NSInteger = if preset == selected { 1 } else { 0 };
            unsafe { let _: () = msg_send![item, setState: s]; }
        }
    }
}

fn set_debounce(ms: u64) {
    let Some(state) = EVENT_STATE.get() else { return };
    state.set_debounce_ms(ms);
    update_checkmarks(&DEBOUNCE_MENU_ITEMS, &DEBOUNCE_PRESETS, ms);

    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
}

fn set_switch(ms: u64) {
    let Some(state) = EVENT_STATE.get() else { return };
    state.set_switch_delay_ms(ms);
    update_checkmarks(&SWITCH_MENU_ITEMS, &SWITCH_PRESETS, ms);

    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
}

fn set_slow_debounce(ms: u64) {
    let Some(state) = EVENT_STATE.get() else { return };
    state.set_slow_debounce_ms(ms);
    update_checkmarks(&SLOW_DEBOUNCE_MENU_ITEMS, &SLOW_DEBOUNCE_PRESETS, ms);

    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
}

// --- ObjC 액션 핸들러 ---

extern "C" fn quit_action(_this: &Object, _cmd: Sel, _sender: id) {
    SHOULD_QUIT.store(true, Ordering::Release);
    // 이벤트 탭 CFRunLoop 정지
    if let Some(state) = EVENT_STATE.get() {
        state.stop();
    }
    unsafe {
        let app: id = NSApp();
        let _: () = msg_send![app, terminate: nil];
    }
}

extern "C" fn set_debounce_200(_: &Object, _: Sel, _: id) { set_debounce(200); }
extern "C" fn set_debounce_300(_: &Object, _: Sel, _: id) { set_debounce(300); }
extern "C" fn set_debounce_500(_: &Object, _: Sel, _: id) { set_debounce(500); }
extern "C" fn set_debounce_800(_: &Object, _: Sel, _: id) { set_debounce(800); }

extern "C" fn set_switch_0(_: &Object, _: Sel, _: id)    { set_switch(0); }
extern "C" fn set_switch_10(_: &Object, _: Sel, _: id)   { set_switch(10); }
extern "C" fn set_switch_30(_: &Object, _: Sel, _: id)   { set_switch(30); }
extern "C" fn set_switch_50(_: &Object, _: Sel, _: id)   { set_switch(50); }

extern "C" fn set_slow_debounce_1000(_: &Object, _: Sel, _: id) { set_slow_debounce(1000); }
extern "C" fn set_slow_debounce_1500(_: &Object, _: Sel, _: id) { set_slow_debounce(1500); }
extern "C" fn set_slow_debounce_2000(_: &Object, _: Sel, _: id) { set_slow_debounce(2000); }
extern "C" fn set_slow_debounce_3000(_: &Object, _: Sel, _: id) { set_slow_debounce(3000); }

extern "C" fn toggle_enabled(_: &Object, _: Sel, _: id) {
    let Some(state) = EVENT_STATE.get() else { return };
    let new_enabled = !state.is_enabled();
    state.set_enabled(new_enabled);

    // 토글 메뉴 아이템 체크마크 업데이트
    let toggle_item = TOGGLE_MENU_ITEM.lock().unwrap_or_else(|e| e.into_inner());
    if !toggle_item.0.is_null() {
        let check: cocoa::foundation::NSInteger = if new_enabled { 1 } else { 0 };
        unsafe { let _: () = msg_send![toggle_item.0, setState: check]; }
    }

    // 메뉴바 아이콘 알파값 변경 (비활성화 시 흐리게)
    let status_item = STATUS_ITEM.lock().unwrap_or_else(|e| e.into_inner());
    if !status_item.0.is_null() {
        unsafe {
            let button: id = msg_send![status_item.0, button];
            if !button.is_null() {
                let alpha: f64 = if new_enabled { 1.0 } else { 0.3 };
                let _: () = msg_send![button, setAlphaValue: alpha];
            }
        }
    }

    // 설정 저장
    let config = current_config();
    if let Err(e) = save_config(&config) {
        log::error!("설정 저장 실패: {}", e);
    }
}

extern "C" fn open_settings(_: &Object, _: Sel, _: id) {
    crate::ui::settings::show_settings_window();
}

/// 외부에서 토글 상태를 업데이트할 때 사용 (설정 윈도우에서 호출)
pub fn update_toggle_state(enabled: bool) {
    let toggle_item = TOGGLE_MENU_ITEM.lock().unwrap_or_else(|e| e.into_inner());
    if !toggle_item.0.is_null() {
        let check: cocoa::foundation::NSInteger = if enabled { 1 } else { 0 };
        unsafe { let _: () = msg_send![toggle_item.0, setState: check]; }
    }

    let status_item = STATUS_ITEM.lock().unwrap_or_else(|e| e.into_inner());
    if !status_item.0.is_null() {
        unsafe {
            let button: id = msg_send![status_item.0, button];
            if !button.is_null() {
                let alpha: f64 = if enabled { 1.0 } else { 0.3 };
                let _: () = msg_send![button, setAlphaValue: alpha];
            }
        }
    }
}

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
        decl.add_method(sel!(setSlowDebounce1000:), set_slow_debounce_1000 as ActionFn);
        decl.add_method(sel!(setSlowDebounce1500:), set_slow_debounce_1500 as ActionFn);
        decl.add_method(sel!(setSlowDebounce2000:), set_slow_debounce_2000 as ActionFn);
        decl.add_method(sel!(setSlowDebounce3000:), set_slow_debounce_3000 as ActionFn);
        decl.add_method(sel!(toggleEnabled:), toggle_enabled as ActionFn);
        decl.add_method(sel!(openSettings:), open_settings as ActionFn);
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
    items_out: &Mutex<[SendId; 4]>,
    delegate: id,
) -> id {
    let menu_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
        NSString::alloc(nil).init_str(title),
        selector(""),
        NSString::alloc(nil).init_str(""),
    );
    let submenu = NSMenu::new(nil).autorelease();
    let _: () = msg_send![submenu, setTitle: NSString::alloc(nil).init_str(title)];

    let mut items_guard = items_out.lock().unwrap_or_else(|e| e.into_inner());
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
        items_guard[i] = SendId(item);
        submenu.addItem_(item);
    }

    let _: () = msg_send![menu_item, setSubmenu: submenu];
    menu_item
}

impl MenuBarApp {
    pub fn new(running: Arc<AtomicBool>, event_state: Arc<EventTapState>) -> Self {
        let _ = EVENT_STATE.set(Arc::clone(&event_state));

        let cur_enabled = event_state.is_enabled();
        let cur_debounce = event_state.get_debounce_ms();
        let cur_switch = event_state.get_switch_delay_ms();
        let cur_slow_debounce = event_state.get_slow_debounce_ms();

        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let app = NSApp();
            app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

            // 메뉴바 아이콘 설정
            // 1. NSBundle에서 로드 시도
            // 2. 실패 시 임베딩된 PNG 데이터에서 로드
            // 3. 최종 실패 시 "코" 텍스트 폴백
            let icon_loaded = (|| -> bool {
                // 방법 1: NSBundle에서 리소스 로드
                let bundle: id = msg_send![class!(NSBundle), mainBundle];
                let res_name = NSString::alloc(nil).init_str("menubar_icon");
                let res_type = NSString::alloc(nil).init_str("png");
                let path: id = msg_send![bundle, pathForResource: res_name ofType: res_type];
                let image: id = if path != nil {
                    let img: id = msg_send![class!(NSImage), alloc];
                    msg_send![img, initWithContentsOfFile: path]
                } else {
                    nil
                };

                // 방법 2: 임베딩된 데이터에서 로드
                let image: id = if image != nil {
                    image
                } else {
                    let data: id = msg_send![class!(NSData), dataWithBytes: MENUBAR_ICON_DATA.as_ptr()
                                                               length: MENUBAR_ICON_DATA.len()];
                    if data == nil {
                        return false;
                    }
                    let img: id = msg_send![class!(NSImage), alloc];
                    msg_send![img, initWithData: data]
                };

                if image == nil {
                    return false;
                }
                let size = NSSize::new(18.0, 18.0);
                let _: () = msg_send![image, setSize: size];
                let _: () = msg_send![image, setTemplate: YES];
                let button: id = msg_send![status_item, button];
                let _: () = msg_send![button, setImage: image];
                true
            })();
            if !icon_loaded {
                let title = NSString::alloc(nil).init_str("코");
                let _: () = msg_send![status_item, setTitle: title];
            }

            // status_item을 전역 상태에 저장 (아이콘 알파 변경용)
            {
                let mut si = STATUS_ITEM.lock().unwrap_or_else(|e| e.into_inner());
                *si = SendId(status_item);
            }

            // 비활성화 상태면 아이콘 흐리게 표시
            if !cur_enabled {
                let button: id = msg_send![status_item, button];
                if !button.is_null() {
                    let _: () = msg_send![button, setAlphaValue: 0.3f64];
                }
            }

            let menu = NSMenu::new(nil).autorelease();

            let delegate_class = create_app_delegate_class();
            let delegate: id = msg_send![delegate_class, new];

            // Koing v0.2 (비활성)
            let version_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str(concat!("Koing v", env!("CARGO_PKG_VERSION"))),
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

            // "Koing 활성화" 토글 메뉴 아이템
            let toggle_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("Koing 활성화"),
                sel!(toggleEnabled:),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![toggle_item, setTarget: delegate];
            if cur_enabled {
                let _: () = msg_send![toggle_item, setState: 1i64];
            }
            {
                let mut ti = TOGGLE_MENU_ITEM.lock().unwrap_or_else(|e| e.into_inner());
                *ti = SendId(toggle_item);
            }
            menu.addItem_(toggle_item);

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
                &DEBOUNCE_MENU_ITEMS,
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
                &SWITCH_MENU_ITEMS,
                delegate,
            );
            menu.addItem_(switch_item);

            // 느린 변환 서브메뉴
            let slow_debounce_item = build_submenu(
                "느린 변환",
                &SLOW_DEBOUNCE_LABELS,
                [
                    sel!(setSlowDebounce1000:),
                    sel!(setSlowDebounce1500:),
                    sel!(setSlowDebounce2000:),
                    sel!(setSlowDebounce3000:),
                ],
                &SLOW_DEBOUNCE_PRESETS,
                cur_slow_debounce,
                &SLOW_DEBOUNCE_MENU_ITEMS,
                delegate,
            );
            menu.addItem_(slow_debounce_item);

            menu.addItem_(NSMenuItem::separatorItem(nil));

            // 설정...
            let settings_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("설정..."),
                sel!(openSettings:),
                NSString::alloc(nil).init_str(","),
            );
            let _: () = msg_send![settings_item, setTarget: delegate];
            menu.addItem_(settings_item);

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

    pub fn run(&self) {
        unsafe {
            let app = NSApp();
            app.run();
        }
        self.running.store(false, Ordering::Release);
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

