//! macOS 메뉴바 앱 (NSStatusBar)
#![allow(deprecated)] // cocoa 크레이트 deprecated API 사용

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

// 메뉴 액션 핸들러를 위한 전역 플래그
static mut SHOULD_QUIT: bool = false;

/// 종료 메뉴 클릭 핸들러
extern "C" fn quit_action(_this: &Object, _cmd: Sel, _sender: id) {
    unsafe {
        SHOULD_QUIT = true;
        let app: id = NSApp();
        let _: () = msg_send![app, terminate: nil];
    }
}

/// AppDelegate 클래스 생성
fn create_app_delegate_class() -> &'static Class {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("KoingAppDelegate", superclass).unwrap();

    unsafe {
        decl.add_method(
            sel!(quitApp:),
            quit_action as extern "C" fn(&Object, Sel, id),
        );
    }

    decl.register()
}

impl MenuBarApp {
    /// 새 메뉴바 앱 생성
    pub fn new(running: Arc<AtomicBool>) -> Self {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // NSApplication 초기화
            let app = NSApp();
            app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

            // 상태바 아이템 생성
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

            // 아이콘/타이틀 설정 (한글 "코" 사용)
            let title = NSString::alloc(nil).init_str("코");
            let _: () = msg_send![status_item, setTitle: title];

            // 메뉴 생성
            let menu = NSMenu::new(nil).autorelease();

            // AppDelegate 인스턴스 생성
            let delegate_class = create_app_delegate_class();
            let delegate: id = msg_send![delegate_class, new];

            // "Koing v0.1" 메뉴 항목 (비활성)
            let version_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("Koing v0.1"),
                selector(""),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![version_item, setEnabled: NO];
            menu.addItem_(version_item);

            // 구분선
            let separator = NSMenuItem::separatorItem(nil);
            menu.addItem_(separator);

            // "단축키: ⌥ Space" 메뉴 항목 (비활성)
            let hotkey_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("단축키: ⌥ Space"),
                selector(""),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![hotkey_item, setEnabled: NO];
            menu.addItem_(hotkey_item);

            // 구분선
            let separator2 = NSMenuItem::separatorItem(nil);
            menu.addItem_(separator2);

            // "종료" 메뉴 항목
            let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str("종료"),
                sel!(quitApp:),
                NSString::alloc(nil).init_str("q"),
            );
            let _: () = msg_send![quit_item, setTarget: delegate];
            menu.addItem_(quit_item);

            // 메뉴 설정
            status_item.setMenu_(menu);

            Self {
                status_item,
                running,
            }
        }
    }

    /// 상태 텍스트 업데이트
    pub fn set_title(&self, title: &str) {
        unsafe {
            let ns_title = NSString::alloc(nil).init_str(title);
            let _: () = msg_send![self.status_item, setTitle: ns_title];
        }
    }

    /// 앱 실행 (블로킹)
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
    // 메뉴바 테스트는 GUI 환경에서만 가능
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
