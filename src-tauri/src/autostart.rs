use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::{db, state::AppState};

/// 启动时对齐开机自启状态。
///
/// 开关的期望值存在数据库里，真实状态存在系统注册表里，两者可能不一致：
/// - 首次运行：数据库还没有用户设置，按默认值把注册表写一遍，让默认的「开启」真正生效；
/// - 之后每次启动：系统是唯一事实来源，只把真实状态回写设置，让界面显示与实际一致。
///
/// 对齐后的值会存进数据库，因此「首次运行」只会发生一次。
pub fn sync(app: &AppHandle, first_run: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let actual = autolaunch
        .is_enabled()
        .map_err(|error| format!("读取开机自启状态失败：{error}"))?;

    let current = app.state::<AppState>().settings();

    let aligned = if first_run {
        let desired = current.launch_at_login;
        if desired != actual {
            let result = if desired {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };
            result.map_err(|error| format!("设置开机自启失败：{error}"))?;
        }
        desired
    } else {
        actual
    };

    // 首次运行即使状态恰好一致也要落盘，用来标记「已初始化」
    if !first_run && aligned == current.launch_at_login {
        return Ok(());
    }

    let mut next = current;
    next.launch_at_login = aligned;
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().map_err(|error| error.to_string())?;
        db::save_settings(&conn, &next).map_err(|error| error.to_string())?;
    }
    *state.settings.lock().unwrap() = next;
    Ok(())
}
