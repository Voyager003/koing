//! Koing - macOS 한영 자동변환 프로그램

use koing::config::load_config;
use koing::ngram::KoreanValidator;
use koing::platform::{
    event_tap::{start_event_tap, EventTapState, HotkeyConfig},
    input_source::switch_to_korean,
    permissions::{permission_status_string, request_accessibility_permission},
    text_replacer::{replace_text, undo_replace_text},
};
use std::sync::atomic::Ordering as AtomicOrdering;
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

    // 설정 로드
    let config = load_config();
    log::info!(
        "설정 로드: debounce={}ms, switch={}ms",
        config.debounce_ms, config.switch_delay_ms
    );

    println!();
    println!("단축키:");
    println!("  - ⌥ Option + Space: 수동 한글 변환");
    println!("  - ⌥ Option + Z: 마지막 변환 되돌리기 (Undo)");
    println!();
    println!(
        "실시간 모드: 타이핑 후 {}ms 대기 시 자동 변환, 변환 후 {}ms 뒤 한글 자판 전환",
        config.debounce_ms, config.switch_delay_ms
    );
    println!();

    // 앱 실행 상태
    let running = Arc::new(AtomicBool::new(true));

    // 이벤트 탭 상태
    let event_state = Arc::new(EventTapState::new(HotkeyConfig::default()));
    event_state.set_debounce_ms(config.debounce_ms);
    event_state.set_switch_delay_ms(config.switch_delay_ms);

    // 변환 이력 저장용 Arc 클론
    let event_state_for_callback = Arc::clone(&event_state);

    // 변환 콜백 설정
    // 콜백은 이벤트 탭 스레드에서 호출될 수 있으므로,
    // 블로킹 작업(replace_text)은 별도 스레드에서 실행하여
    // 이벤트 탭이 macOS에 의해 비활성화되지 않도록 함
    event_state.set_convert_callback(move |buffer: String, _is_manual: bool| {
        let event_state = Arc::clone(&event_state_for_callback);
        thread::spawn(move || {
            log::info!("변환 요청: '{}'", buffer);

            let validator = KoreanValidator::new();
            let result = validator.analyze(&buffer);
            log::info!("변환 결과: '{}' -> '{}'", buffer, result.converted);

            if !result.should_convert {
                log::info!(
                    "변환 취소: '{}' → '{}' (jamo={}, unnatural={})",
                    buffer, result.converted,
                    result.has_incomplete_jamo, result.has_unnatural_syllables
                );
                return;
            }
            let hangul = result.converted;

            // 텍스트 교체 중 플래그 설정 (실시간 변환 레이스 방지)
            event_state
                .is_replacing
                .store(true, AtomicOrdering::SeqCst);

            // 한글 전환을 텍스트 교체와 동시에 시작
            // replace_text()는 백스페이스/붙여넣기를 keycode로 시뮬레이션하므로
            // 입력 소스와 무관하게 동작함
            thread::spawn(|| {
                if let Err(e) = switch_to_korean() {
                    log::warn!("한글 전환 실패: {}", e);
                }
            });

            // 텍스트 교체 (이 동안 한글 전환도 진행됨)
            let backspace_count = buffer.chars().count();
            let result = replace_text(backspace_count, &hangul);

            event_state
                .is_replacing
                .store(false, AtomicOrdering::SeqCst);

            if let Err(e) = result {
                log::error!("텍스트 교체 실패: {}", e);
                return;
            }

            // 변환 이력 저장 (Undo용)
            event_state.save_conversion_history(buffer, hangul);
        });
    });

    // Undo 콜백 설정
    let event_state_for_undo = Arc::clone(&event_state);
    event_state.set_undo_callback(move |hangul: String, original: String| {
        let event_state = Arc::clone(&event_state_for_undo);
        thread::spawn(move || {
            log::info!("Undo 요청: '{}' -> '{}'", hangul, original);

            // 텍스트 교체 중 플래그 설정 (실시간 변환 레이스 방지)
            event_state
                .is_replacing
                .store(true, AtomicOrdering::SeqCst);

            let result = undo_replace_text(&hangul, &original);

            event_state
                .is_replacing
                .store(false, AtomicOrdering::SeqCst);

            if let Err(e) = result {
                log::error!("Undo 텍스트 교체 실패: {}", e);
            }
        });
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
    let app = MenuBarApp::new(Arc::clone(&running), Arc::clone(&event_state));
    app.run();

    println!("Koing 종료");
}
