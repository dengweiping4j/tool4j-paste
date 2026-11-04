use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

/// 窗口句柄在窗口间以整数形式传递，避免在状态里持有裸指针类型
pub fn hwnd_from_raw(raw: isize) -> HWND {
    raw as HWND
}

/// UTF-16 + null 结尾，用于 Win32 宽字符 API
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn foreground_window() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

pub fn cursor_position() -> Option<POINT> {
    let mut point = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut point) } == 0 {
        None
    } else {
        Some(point)
    }
}

/// 鼠标所在显示器的工作区（已排除任务栏）
pub fn monitor_work_area(point: POINT) -> Option<RECT> {
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            None
        } else {
            Some(info.rcWork)
        }
    }
}

/// 把窗口拉到前台。直接调用常被前台锁定策略拒绝，
/// 先试一次，不行再临时附加输入队列重试。
pub fn activate_window(raw: isize) -> bool {
    if raw == 0 {
        return false;
    }
    let hwnd = hwnd_from_raw(raw);

    unsafe {
        if SetForegroundWindow(hwnd) != 0 && GetForegroundWindow() == hwnd {
            return true;
        }

        let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let current_thread = GetCurrentThreadId();
        if target_thread != 0 && target_thread != current_thread {
            if AttachThreadInput(current_thread, target_thread, 1) != 0 {
                let ok = SetForegroundWindow(hwnd) != 0;
                AttachThreadInput(current_thread, target_thread, 0);
                if ok {
                    return true;
                }
            }
        }

        SetForegroundWindow(hwnd) != 0
    }
}

/// 目标窗口的进程名（不含路径），用于显示来源应用
pub fn process_name(raw: isize) -> Option<String> {
    if raw == 0 {
        return None;
    }
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd_from_raw(raw), &mut pid);
        if pid == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }

        let mut buffer = [0u16; 512];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);

        if ok == 0 || size == 0 {
            return None;
        }
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        full_path.rsplit('\\').next().map(str::to_string)
    }
}

fn modifier_down() -> bool {
    const KEYS: [VIRTUAL_KEY; 5] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN];
    KEYS.iter().any(|key| unsafe {
        // 最高位为 1 表示当前处于按下状态
        (GetAsyncKeyState(*key as i32) as u16 & 0x8000) != 0
    })
}

/// 呼出面板的快捷键在按下瞬间触发，此时 Ctrl/Alt 往往还按着。
/// 直接注入 Ctrl+V 会变成别的组合键，必须等修饰键松开。
pub fn wait_modifiers_released(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !modifier_down() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    !modifier_down()
}

fn key_event(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    let mut input: INPUT = unsafe { std::mem::zeroed() };
    input.r#type = INPUT_KEYBOARD;
    // 写入 union 字段是安全操作，只有读取才需要 unsafe
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
        time: 0,
        dwExtraInfo: 0,
    };
    input
}

/// 向当前前台窗口发送一次 Ctrl+V
pub fn send_ctrl_v() -> bool {
    let inputs = [
        key_event(VK_CONTROL, false),
        key_event(VK_V, false),
        key_event(VK_V, true),
        key_event(VK_CONTROL, true),
    ];
    let injected = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    injected == inputs.len() as u32
}
