use tauri::{AppHandle, State};

use crate::db::{self, ItemDto, PinboardDto};
use crate::hotkey;
use crate::paste;
use crate::state::{AppState, Settings};
use crate::util;

const MAX_PAGE_SIZE: u32 = 200;
const MAX_PINBOARD_NAME: usize = 20;

#[tauri::command]
pub fn list_items(
    state: State<'_, AppState>,
    pinboard_id: Option<i64>,
    offset: u32,
    limit: u32,
) -> Result<Vec<ItemDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::list(&conn, pinboard_id, offset, limit.min(MAX_PAGE_SIZE))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_items(
    state: State<'_, AppState>,
    query: String,
    pinboard_id: Option<i64>,
    offset: u32,
    limit: u32,
) -> Result<Vec<ItemDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    if query.trim().is_empty() {
        return db::list(&conn, pinboard_id, offset, limit.min(MAX_PAGE_SIZE))
            .map_err(|error| error.to_string());
    }
    db::search(&conn, &query, pinboard_id, offset, limit.min(MAX_PAGE_SIZE))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn paste_item(app: AppHandle, id: i64) -> Result<(), String> {
    paste::paste(&app, id)
}

#[tauri::command]
pub fn toggle_pin(state: State<'_, AppState>, id: i64) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let current: bool = conn
        .query_row("SELECT is_pinned FROM items WHERE id = ?1", [id], |row| {
            row.get::<_, i64>(0)
        })
        .map(|value| value != 0)
        .map_err(|error| error.to_string())?;

    let next = !current;
    db::set_pinned(&conn, id, next).map_err(|error| error.to_string())?;
    Ok(next)
}

#[tauri::command]
pub fn delete_item(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    db::delete(&mut conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_unpinned(state: State<'_, AppState>) -> Result<u64, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    db::clear_unpinned(&mut conn).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let old = state.settings();

    // 快捷键变更：先注册新键，失败则整体回滚（旧键仍可用）
    if old.hotkey != settings.hotkey {
        hotkey::apply_hotkey(&app, &settings.hotkey)?;
    }

    // 开机自启变更
    if old.launch_at_login != settings.launch_at_login {
        use tauri_plugin_autostart::ManagerExt;
        let result = if settings.launch_at_login {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        result.map_err(|error| format!("设置开机自启失败：{error}"))?;
    }

    {
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        db::save_settings(&conn, &settings).map_err(|error| error.to_string())?;
    }
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

// ---------------------------------------------------------------------------
// Pinboard 分组
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_pinboards(state: State<'_, AppState>) -> Result<Vec<PinboardDto>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::list_pinboards(&conn).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_pinboard(state: State<'_, AppState>, name: String) -> Result<PinboardDto, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }
    if name.chars().count() > MAX_PINBOARD_NAME {
        return Err(format!("分组名称不能超过 {MAX_PINBOARD_NAME} 个字符"));
    }

    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    db::create_pinboard(&mut conn, name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rename_pinboard(state: State<'_, AppState>, id: i64, name: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::rename_pinboard(&conn, id, name).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_pinboard(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::delete_pinboard(&conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn assign_pinboard(
    state: State<'_, AppState>,
    item_id: i64,
    pinboard_id: Option<i64>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    db::assign_pinboard(&conn, item_id, pinboard_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_about_page() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = util::to_wide("open");
    let target = util::to_wide("https://www.tool4j.com");

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    // ShellExecuteW 返回值 > 32 表示成功
    if (result as isize) > 32 {
        Ok(())
    } else {
        Err("打开浏览器失败".into())
    }
}
