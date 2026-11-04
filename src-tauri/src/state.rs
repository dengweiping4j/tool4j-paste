use std::path::PathBuf;
use std::sync::atomic::AtomicIsize;
use std::sync::Mutex;
use std::time::Instant;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::db;

/// 自身写入剪贴板后的抑制窗口：只有内容哈希一致才认定为「自己写的」，
/// 因此用户在此期间复制了新内容仍会被正常记录。
pub struct ExpectedWrite {
    pub hash: String,
    pub expires_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
    pub auto_paste: bool,
    pub max_unpinned_items: i64,
    pub launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+V".into(),
            auto_paste: true,
            max_unpinned_items: 100,
            launch_at_login: true,
        }
    }
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub data_dir: PathBuf,
    /// 呼出面板前的前台窗口，粘贴时作为目标
    pub last_foreground: AtomicIsize,
    pub expected_write: Mutex<Option<ExpectedWrite>>,
    pub settings: Mutex<Settings>,
}

impl AppState {
    fn new(conn: Connection, data_dir: PathBuf, settings: Settings) -> Self {
        Self {
            db: Mutex::new(conn),
            data_dir,
            last_foreground: AtomicIsize::new(0),
            expected_write: Mutex::new(None),
            settings: Mutex::new(settings),
        }
    }

    pub fn begin_self_write(&self, hash: String) {
        let mut guard = self.expected_write.lock().unwrap();
        *guard = Some(ExpectedWrite {
            hash,
            expires_at: Instant::now() + std::time::Duration::from_millis(1500),
        });
    }

    /// 返回 true 表示这次剪贴板变化确认由本程序写入，应当跳过记录。
    pub fn consume_self_write(&self, hash: &str) -> bool {
        let mut guard = self.expected_write.lock().unwrap();
        match guard.as_ref() {
            Some(expected) if expected.expires_at > Instant::now() => {
                let matched = expected.hash == hash;
                // 无论是否匹配都清除：不匹配说明用户复制了新内容
                *guard = None;
                matched
            }
            Some(_) => {
                *guard = None;
                false
            }
            None => false,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}

/// 供其他模块（如 hotkey）直接从 AppHandle 读取设置
pub fn app_settings(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings()
}

/// 初始化状态，返回是否为首次运行（数据库里还没有保存过设置）
pub fn init(app: &AppHandle) -> Result<bool, Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(data_dir.join("images"))?;

    let conn = db::open(&data_dir)?;
    let stored = db::load_settings(&conn);
    let first_run = stored.is_none();
    let settings = stored.unwrap_or_default();

    app.manage(AppState::new(conn, data_dir, settings));
    Ok(first_run)
}
