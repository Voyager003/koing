pub mod menubar;
pub mod settings;

// --- 공유 프리셋 상수 (menubar.rs, settings.rs에서 사용) ---

/// 변환 속도 프리셋 (ms)
pub const DEBOUNCE_PRESETS: [u64; 4] = [200, 300, 500, 800];
pub const DEBOUNCE_LABELS: [&str; 4] = [
    "빠름 (200ms)",
    "보통 (300ms)",
    "느림 (500ms)",
    "여유 (800ms)",
];

/// 자판 전환 프리셋 (ms)
pub const SWITCH_PRESETS: [u64; 4] = [0, 10, 30, 50];
pub const SWITCH_LABELS: [&str; 4] = [
    "즉시 (0ms)",
    "빠름 (10ms)",
    "보통 (30ms)",
    "느림 (50ms)",
];

/// 느린 변환 프리셋 (ms)
pub const SLOW_DEBOUNCE_PRESETS: [u64; 4] = [1000, 1500, 2000, 3000];
pub const SLOW_DEBOUNCE_LABELS: [&str; 4] = [
    "빠름 (1초)",
    "보통 (1.5초)",
    "느림 (2초)",
    "여유 (3초)",
];
