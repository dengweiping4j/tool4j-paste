use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::clipboard;
use crate::db;
use crate::state::AppState;
use crate::util;

const MODIFIER_WAIT: Duration = Duration::from_millis(400);

pub fn paste(app: &AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();

    let formats = {
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        db::formats_of(&conn, id).map_err(|error| error.to_string())?
    };
    if formats.is_empty() {
        return Err("该条目的内容已不存在".into());
    }

    let hash = {
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        db::content_hash_of(&conn, id)
            .map_err(|error| error.to_string())?
            .unwrap_or_default()
    };

    // 先登记预期哈希再写入，监听线程看到这次变化时才能认出是自己写的
    state.begin_self_write(hash);
    clipboard::write_formats(&formats)?;

    // 内容已经进了剪贴板，这次使用就算数：后面自动粘贴失败（回退成手动 Ctrl+V）
    // 也照样刷新使用时间，让这条记录浮到列表前面。
    if let Ok(conn) = state.db.lock() {
        let _ = db::touch(&conn, id);
    }

    let target = state.last_foreground.load(Ordering::SeqCst);

    // 先把目标窗口拉回前台并确认成功，之后才收起面板。
    // 顺序反了的话，粘贴失败时面板已经消失，用户既没贴上也不知道发生了什么。
    let restored = target != 0 && util::activate_window(target);

    // 呼出面板的快捷键通常还按着，此时注入 Ctrl+V 会变成 Ctrl+Alt+V
    if !util::wait_modifiers_released(MODIFIER_WAIT) {
        return fallback(app, id, "modifier");
    }

    if !restored {
        return fallback(app, id, "foreground");
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    if state.settings().auto_paste {
        std::thread::sleep(Duration::from_millis(20));
        if !util::send_ctrl_v() {
            return fallback(app, id, "send_input");
        }
    }

    Ok(())
}

/// 内容已经写入剪贴板，只是无法自动粘贴，提示用户手动 Ctrl+V。
/// 这里不能静默失败，否则用户会以为点击没生效。
fn fallback(app: &AppHandle, id: i64, reason: &str) -> Result<(), String> {
    let _ = app.emit(
        "paste-fallback",
        serde_json::json!({ "id": id, "reason": reason }),
    );
    Ok(())
}
