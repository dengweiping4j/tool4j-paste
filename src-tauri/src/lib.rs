mod autostart;
mod clipboard;
mod commands;
mod db;
mod hotkey;
mod paste;
mod state;
mod tray;
mod util;
mod window;

use tauri_plugin_global_shortcut::ShortcutState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    tauri::Builder::default()
        // 单实例插件必须最先注册，否则第二次启动会重复创建托盘图标
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // 只在按下时响应，避免松开时又收起面板
                    if event.state() == ShortcutState::Pressed {
                        window::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            let first_run = state::init(&handle)?;
            tray::create(&handle)?;
            hotkey::register(&handle)?;

            // 对齐开机自启：失败只记录日志，不能因此阻止程序启动
            if let Err(error) = autostart::sync(&handle, first_run) {
                tracing::warn!("{error}");
            }

            // 剪贴板监听独立线程，不占用异步运行时
            clipboard::spawn_watcher(handle.clone());

            tracing::info!("Tool4J Paste 已启动");
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 关闭窗口不退出进程，托盘继续常驻
                api.prevent_close();
                let _ = window.hide();
            }
            // 点击面板以外的地方即收起，符合悬浮工具的直觉
            tauri::WindowEvent::Focused(false) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_items,
            commands::search_items,
            commands::paste_item,
            commands::toggle_pin,
            commands::delete_item,
            commands::clear_unpinned,
            commands::get_settings,
            commands::update_settings,
            commands::list_pinboards,
            commands::create_pinboard,
            commands::rename_pinboard,
            commands::delete_pinboard,
            commands::assign_pinboard,
            commands::open_about_page,
        ])
        .run(tauri::generate_context!())
        .expect("Tool4J Paste 启动失败");
}
