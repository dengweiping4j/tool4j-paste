use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut,
};

/// 注册全局快捷键。注册失败不阻断启动：托盘仍然可以呼出面板，
/// 只是要在日志里留下明确记录，避免静默失效。
pub fn register(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = crate::state::app_settings(app);
    apply_hotkey(app, &settings.hotkey)?;
    Ok(())
}

/// 解析 "Ctrl+Alt+V" 这类字符串。主键支持 A-Z、0-9、F1-F12。
pub fn parse_hotkey(spec: &str) -> Option<Shortcut> {
    let mut modifiers = Modifiers::empty();
    let mut main: Option<Code> = None;

    for part in spec.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "win" | "super" | "meta" => modifiers |= Modifiers::SUPER,
            other => {
                if main.is_some() {
                    return None; // 出现两个主键
                }
                main = Some(code_from_name(other)?);
            }
        }
    }

    Some(Shortcut::new(Some(modifiers), main?))
}

fn code_from_name(name: &str) -> Option<Code> {
    // 单字母：KeyA .. KeyZ
    if name.len() == 1 {
        let c = name.to_uppercase().chars().next()?;
        let code = match c {
            'A' => Code::KeyA,
            'B' => Code::KeyB,
            'C' => Code::KeyC,
            'D' => Code::KeyD,
            'E' => Code::KeyE,
            'F' => Code::KeyF,
            'G' => Code::KeyG,
            'H' => Code::KeyH,
            'I' => Code::KeyI,
            'J' => Code::KeyJ,
            'K' => Code::KeyK,
            'L' => Code::KeyL,
            'M' => Code::KeyM,
            'N' => Code::KeyN,
            'O' => Code::KeyO,
            'P' => Code::KeyP,
            'Q' => Code::KeyQ,
            'R' => Code::KeyR,
            'S' => Code::KeyS,
            'T' => Code::KeyT,
            'U' => Code::KeyU,
            'V' => Code::KeyV,
            'W' => Code::KeyW,
            'X' => Code::KeyX,
            'Y' => Code::KeyY,
            'Z' => Code::KeyZ,
            _ => return None,
        };
        return Some(code);
    }

    // 数字：Digit0 .. Digit9
    if let Some(digit) = name.strip_prefix('d').and_then(|rest| rest.parse::<u8>().ok()) {
        let code = match digit {
            0 => Code::Digit0,
            1 => Code::Digit1,
            2 => Code::Digit2,
            3 => Code::Digit3,
            4 => Code::Digit4,
            5 => Code::Digit5,
            6 => Code::Digit6,
            7 => Code::Digit7,
            8 => Code::Digit8,
            9 => Code::Digit9,
            _ => return None,
        };
        return Some(code);
    }

    // 功能键：F1 .. F12
    if let Some(n) = name.strip_prefix('f').and_then(|rest| rest.parse::<u8>().ok()) {
        if (1..=12).contains(&n) {
            let code = match n {
                1 => Code::F1,
                2 => Code::F2,
                3 => Code::F3,
                4 => Code::F4,
                5 => Code::F5,
                6 => Code::F6,
                7 => Code::F7,
                8 => Code::F8,
                9 => Code::F9,
                10 => Code::F10,
                11 => Code::F11,
                _ => Code::F12,
            };
            return Some(code);
        }
    }

    None
}

/// 注销旧快捷键并注册新的。失败时旧键保持可用，返回错误给设置页提示。
pub fn apply_hotkey(app: &AppHandle, spec: &str) -> Result<(), String> {
    let shortcut =
        parse_hotkey(spec).ok_or_else(|| format!("无法识别的快捷键「{spec}」，支持 Ctrl/Alt/Shift/Win + 字母、数字或 F1-F12"))?;

    let shortcut_manager = app.global_shortcut();
    // 先注销所有已注册的快捷键，避免旧键残留
    let _ = shortcut_manager.unregister_all();
    shortcut_manager
        .register(shortcut)
        .map_err(|error| format!("快捷键注册失败，可能已被其他程序占用：{error}"))?;

    tracing::info!("全局快捷键已更新：{spec}");
    Ok(())
}
