//! Koing - macOS 한영 자동변환 프로그램

use koing::config::load_config;
use koing::ngram::KoreanValidator;
use koing::platform::{
    event_tap::{start_event_tap, EventTapState, HotkeyConfig},
    input_source::switch_to_korean,
    permissions::request_accessibility_permission,
    text_replacer::{replace_text, undo_replace_text},
};
use std::sync::atomic::Ordering as AtomicOrdering;
use koing::ui::menubar::MenuBarApp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

/// 워커 스레드가 처리할 작업 항목
enum WorkItem {
    /// 영문→한글 변환 (버퍼 내용)
    Convert(String),
    /// Undo (한글 텍스트, 원본 영문)
    Undo(String, String),
}

fn main() {
    // 로깅 초기화 (error/warn만 출력)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // Accessibility 권한 확인

    if !request_accessibility_permission(true) {
        eprintln!();
        eprintln!("⚠️  Accessibility 권한이 필요합니다.");
        eprintln!("   시스템 설정 > 개인 정보 보호 및 보안 > 손쉬운 사용");
        eprintln!("   에서 이 앱을 허용해주세요.");
        eprintln!();
        eprintln!("권한을 허용한 후 앱을 다시 실행해주세요.");

    }

    // 설정 로드
    let config = load_config();

    // 앱 실행 상태
    let running = Arc::new(AtomicBool::new(true));

    // 이벤트 탭 상태
    let event_state = Arc::new(EventTapState::new(HotkeyConfig::default()));
    event_state.set_debounce_ms(config.debounce_ms);
    event_state.set_switch_delay_ms(config.switch_delay_ms);

    // 워커 스레드 채널 — 변환/Undo 작업을 단일 스레드에서 직렬 처리
    let (work_tx, work_rx) = mpsc::channel::<WorkItem>();

    let event_state_for_worker = Arc::clone(&event_state);
    thread::spawn(move || {
        let validator = KoreanValidator::new();

        while let Ok(item) = work_rx.recv() {
            match item {
                WorkItem::Convert(buffer) => {
                    let result = validator.analyze(&buffer);

                    if !result.should_convert {
                        continue;
                    }
                    let hangul = result.converted;

                    // 한 글자 변환은 오탐 가능성이 높으므로 차단 (ex: rk→가, fn→루)
                    if hangul.chars().count() <= 1 {
                        continue;
                    }

                    // 텍스트 교체 중 플래그 설정 (실시간 변환 레이스 방지)
                    event_state_for_worker
                        .is_replacing
                        .store(true, AtomicOrdering::Release);

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
                    let replace_result = replace_text(backspace_count, &hangul);

                    event_state_for_worker
                        .is_replacing
                        .store(false, AtomicOrdering::Release);

                    if let Err(e) = replace_result {
                        log::error!("텍스트 교체 실패: {}", e);
                        continue;
                    }

                    // 변환 이력 저장 (Undo용)
                    event_state_for_worker.save_conversion_history(buffer, hangul);
                }
                WorkItem::Undo(hangul, original) => {
                    // 텍스트 교체 중 플래그 설정 (실시간 변환 레이스 방지)
                    event_state_for_worker
                        .is_replacing
                        .store(true, AtomicOrdering::Release);

                    let result = undo_replace_text(&hangul, &original);

                    event_state_for_worker
                        .is_replacing
                        .store(false, AtomicOrdering::Release);

                    if let Err(e) = result {
                        log::error!("Undo 텍스트 교체 실패: {}", e);
                    }
                }
            }
        }
    });

    // 변환 콜백 설정
    // 콜백은 이벤트 탭 스레드에서 호출되므로, 워커에 전송만 하여
    // 이벤트 탭이 macOS에 의해 비활성화되지 않도록 함
    let convert_tx = work_tx.clone();
    event_state.set_convert_callback(move |buffer: String, _is_manual: bool| {
        let _ = convert_tx.send(WorkItem::Convert(buffer));
    });

    // Undo 콜백 설정
    let undo_tx = work_tx;
    event_state.set_undo_callback(move |hangul: String, original: String| {
        let _ = undo_tx.send(WorkItem::Undo(hangul, original));
    });

    // 이벤트 탭 스레드 시작
    let event_state_for_thread = Arc::clone(&event_state);
    let running_for_thread = Arc::clone(&running);
    thread::spawn(move || {
        if let Err(e) = start_event_tap(event_state_for_thread) {
            log::error!("Event tap 시작 실패: {}", e);
        }
        running_for_thread.store(false, Ordering::Release);
    });

    // 메뉴바 앱 실행 (메인 스레드에서)
    let app = MenuBarApp::new(Arc::clone(&running), Arc::clone(&event_state));
    app.run();
}
