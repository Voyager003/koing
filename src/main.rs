//! Koing - macOS 한영 자동변환 프로그램

use koing::convert;
use koing::platform::{
    event_tap::{start_event_tap, EventTapState, HotkeyConfig},
    permissions::{permission_status_string, request_accessibility_permission},
    text_replacer::replace_text,
};
use koing::ui::menubar::MenuBarApp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    // 로깅 초기화
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("Koing v0.1 - macOS 한영 자동변환 프로그램");
    println!();

    // Accessibility 권한 확인
    println!("{}", permission_status_string());

    if !request_accessibility_permission(true) {
        eprintln!();
        eprintln!("⚠️  Accessibility 권한이 필요합니다.");
        eprintln!("   시스템 설정 > 개인 정보 보호 및 보안 > 손쉬운 사용");
        eprintln!("   에서 이 앱을 허용해주세요.");
        eprintln!();
        eprintln!("권한을 허용한 후 앱을 다시 실행해주세요.");

        // 권한 없이도 메뉴바 앱은 실행 (테스트용)
        println!();
        println!("메뉴바 앱만 실행합니다 (변환 기능 비활성화)...");
    }

    println!();
    println!("단축키: ⌥ Option + Space");
    println!("영문을 입력한 후 단축키를 누르면 한글로 변환됩니다.");
    println!();

    // 앱 실행 상태
    let running = Arc::new(AtomicBool::new(true));

    // 이벤트 탭 상태
    let event_state = Arc::new(EventTapState::new(HotkeyConfig::default()));

    // 변환 콜백 설정
    event_state.set_convert_callback(move |buffer: String| {
        log::info!("변환 요청: '{}'", buffer);

        let hangul = convert(&buffer);
        log::info!("변환 결과: '{}' -> '{}'", buffer, hangul);

        // 텍스트 교체
        let backspace_count = buffer.chars().count();
        if let Err(e) = replace_text(backspace_count, &hangul) {
            log::error!("텍스트 교체 실패: {}", e);
        }
    });

    // 이벤트 탭 스레드 시작
    let event_state_for_thread = Arc::clone(&event_state);
    let running_for_thread = Arc::clone(&running);
    thread::spawn(move || {
        if let Err(e) = start_event_tap(event_state_for_thread) {
            log::error!("Event tap 시작 실패: {}", e);
        }
        running_for_thread.store(false, Ordering::SeqCst);
    });

    // 메뉴바 앱 실행 (메인 스레드에서)
    let app = MenuBarApp::new(Arc::clone(&running));
    app.run();

    println!("Koing 종료");
}
