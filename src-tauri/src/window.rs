use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::state::AppState;
use crate::util;

/// 面板高度固定，宽度铺满显示器
const PANEL_HEIGHT: i32 = 520;
const EDGE_MARGIN: i32 = 0;
const PANEL_MIN_WIDTH: i32 = 600;
const BOTTOM_MARGIN: i32 = 0;

/// 快捷键 / 托盘触发：可见则收起，不可见则展开
pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    // 目标窗口必须在面板获得焦点之前捕获，否则会记成本程序自己
    let foreground = util::foreground_window();
    app.state::<AppState>()
        .last_foreground
        .store(foreground, Ordering::SeqCst);

    remove_resize_border(&window);
    apply_layout(&window);
    apply_square_corners(&window);

    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("panel-shown", ());
}

/// 去掉窗口的 WS_THICKFRAME 样式。面板固定尺寸、不可缩放，留着这个样式
/// 只会让 Windows 在窗口外沿预留一圈可拖拽的粗边框。
fn remove_resize_border(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_THICKFRAME,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = util::hwnd_from_raw(hwnd.0 as isize);

    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        if style & WS_THICKFRAME == 0 {
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !WS_THICKFRAME) as isize);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}

/// 关掉 Windows 11 给无边框窗口强加的系统圆角。
/// 只改 CSS 不够：DWM 会在窗口外层再裁一次圆角，必须显式声明不要。
fn apply_square_corners(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };

    let preference: i32 = DWMWCP_DONOTROUND;
    unsafe {
        DwmSetWindowAttribute(
            util::hwnd_from_raw(hwnd.0 as isize),
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &preference as *const i32 as *const core::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

/// 贴着鼠标所在显示器的底部铺开，宽度随屏幕自适应
fn apply_layout(window: &tauri::WebviewWindow) {
    let Some(cursor) = util::cursor_position() else {
        return;
    };
    let Some(work) = util::monitor_work_area(cursor) else {
        return;
    };

    let screen_width = work.right - work.left;
    let width = (screen_width - EDGE_MARGIN).max(PANEL_MIN_WIDTH);

    let x = work.left + (screen_width - width) / 2;
    let y = work.bottom - PANEL_HEIGHT - BOTTOM_MARGIN;

    let _ = window.set_size(PhysicalSize::new(width as u32, PANEL_HEIGHT as u32));
    let _ = window.set_position(PhysicalPosition::new(x, y.max(work.top)));

    // 无边框窗口的内容区应当与窗口外框完全重合。两者一旦不一致，说明窗口
    // 又被套上了一圈隐藏边框，面板就会离开屏幕边缘，这里留一条日志便于发现。
    if let (Ok(inner), Ok(outer)) = (window.inner_size(), window.outer_size()) {
        if inner != outer {
            tracing::warn!(
                "面板内容区与窗口外框不一致：内容 {}x{}，外框 {}x{}",
                inner.width,
                inner.height,
                outer.width,
                outer.height
            );
        }
    }
}
