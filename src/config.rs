//! 설정 파일 로드/저장 (JSON)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Koing 설정
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KoingConfig {
    /// 타이핑 멈춘 후 자동 변환까지 대기 시간 (ms)
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// 자동 변환 후 한글 자판 전환까지 대기 시간 (ms)
    #[serde(default = "default_switch_delay_ms")]
    pub switch_delay_ms: u64,
    /// 붙여넣기 후 클립보드 복원까지 대기 시간 (ms)
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
}

fn default_debounce_ms() -> u64 {
    300
}

fn default_switch_delay_ms() -> u64 {
    0
}

fn default_paste_delay_ms() -> u64 {
    500
}

impl Default for KoingConfig {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(),
            switch_delay_ms: default_switch_delay_ms(),
            paste_delay_ms: default_paste_delay_ms(),
        }
    }
}

/// 설정 파일 경로: ~/Library/Application Support/koing/config.json
pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.is_dir())
        .unwrap_or_else(|| {
            // HOME 미설정이거나 유효하지 않으면 /var/tmp 폴백 (쓰기 가능, /tmp보다 안전)
            PathBuf::from("/var/tmp")
        });
    home.join("Library")
        .join("Application Support")
        .join("koing")
        .join("config.json")
}

/// 설정 파일 로드 (파일 없거나 파싱 실패 시 기본값)
pub fn load_config() -> KoingConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| {
            KoingConfig::default()
        }),
        Err(_) => KoingConfig::default(),
    }
}

/// 설정 파일 저장
pub fn save_config(config: &KoingConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("설정 디렉토리 생성 실패: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("직렬화 실패: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("설정 파일 저장 실패: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KoingConfig::default();
        assert_eq!(config.debounce_ms, 300);
        assert_eq!(config.switch_delay_ms, 0);
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = KoingConfig {
            debounce_ms: 150,
            switch_delay_ms: 50,
            paste_delay_ms: 500,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: KoingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.debounce_ms, 150);
        assert_eq!(parsed.switch_delay_ms, 50);
    }

    #[test]
    fn test_backward_compat_missing_field() {
        // 이전 설정 파일에 debounce_ms가 없는 경우 기본값 사용
        let json = r#"{"switch_delay_ms": 300}"#;
        let config: KoingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.debounce_ms, 300);
        assert_eq!(config.switch_delay_ms, 300);
    }
}
