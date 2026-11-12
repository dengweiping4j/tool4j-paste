use std::time::Duration;

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW,
    GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows_sys::Win32::UI::Shell::DragQueryFileW;

use crate::db::{self, NewFormat, NewItem, StoredFormat};
use crate::state::AppState;
use crate::util;

/// 标准剪贴板格式号。取值由 Windows SDK 固定（winuser.h），不会变更，
/// 这里直接定义，避免为两个常量额外引入 Win32_System_Ole 模块。
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;

/// 剪贴板轮询间隔。轮询而非 WM_CLIPBOARDUPDATE 消息：
/// 无需自建消息窗口与窗口过程，代码路径更短，250ms 延迟对使用体验无感。
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// 剪贴板被占用时的重试次数。浏览器渲染进程、Office、输入法都会短暂占用。
const OPEN_RETRY: usize = 5;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(20);

const PREVIEW_LIMIT: usize = 200;
const SEARCH_TEXT_LIMIT: usize = 4000;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn register_format(name: &str) -> u32 {
    let buffer = wide(name);
    unsafe { RegisterClipboardFormatW(buffer.as_ptr()) }
}

// ---------------------------------------------------------------------------
// 读取
// ---------------------------------------------------------------------------

struct Captured {
    text: Option<String>,
    html: Option<Vec<u8>>,
    rtf: Option<Vec<u8>>,
    files: Option<Vec<String>>,
    png: Option<Vec<u8>>,
}

fn open_clipboard_with_retry() -> bool {
    for _ in 0..OPEN_RETRY {
        if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
            return true;
        }
        std::thread::sleep(OPEN_RETRY_DELAY);
    }
    false
}

fn format_name(format: u32) -> Option<String> {
    let mut buffer = [0u16; 128];
    let length = unsafe {
        GetClipboardFormatNameW(format, buffer.as_mut_ptr(), buffer.len() as i32)
    };
    if length <= 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

/// 隐私检查：必须在读取任何用户内容之前完成。
/// 任一步骤出错都按「敏感」处理，宁可漏记也不误记。
fn has_private_marker() -> bool {
    let mut format = 0u32;
    loop {
        format = unsafe { EnumClipboardFormats(format) };
        if format == 0 {
            return false;
        }

        let Some(name) = format_name(format) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();

        if lower == "clipboard viewer ignore"
            || lower == "excludeclipboardcontentfrommonitorprocessing"
        {
            return true;
        }

        // 这两个格式携带 DWORD 开关，0 表示禁止进入历史
        if lower == "canincludeinclipboardhistory" || lower == "canuploadtocloudclipboard" {
            if let Some(value) = read_format_dword(format) {
                if value == 0 {
                    return true;
                }
            }
        }
    }
}

fn read_format_dword(format: u32) -> Option<u32> {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let ptr = GlobalLock(handle) as *const u32;
        if ptr.is_null() {
            return None;
        }
        let value = *ptr;
        GlobalUnlock(handle);
        Some(value)
    }
}

fn read_format_bytes(format: u32) -> Option<Vec<u8>> {
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let size = GlobalSize(handle);
        if size == 0 {
            return None;
        }
        let ptr = GlobalLock(handle) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        GlobalUnlock(handle);
        Some(bytes)
    }
}

fn read_text() -> Option<String> {
    let bytes = read_format_bytes(CF_UNICODETEXT)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();
    let end = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
    let text = String::from_utf16_lossy(&units[..end]);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn read_files() -> Option<Vec<String>> {
    unsafe {
        let handle = GetClipboardData(CF_HDROP);
        if handle.is_null() {
            return None;
        }
        // 先取数量（0xFFFFFFFF 是 SDK 约定的「查询总数」参数）
        let count = DragQueryFileW(handle, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
        if count == 0 {
            return None;
        }

        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            let length = DragQueryFileW(handle, index, std::ptr::null_mut(), 0);
            if length == 0 {
                continue;
            }
            let mut buffer = vec![0u16; (length + 1) as usize];
            let written = DragQueryFileW(handle, index, buffer.as_mut_ptr(), buffer.len() as u32);
            if written == 0 {
                continue;
            }
            paths.push(String::from_utf16_lossy(&buffer[..written as usize]));
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }
}

fn read_png() -> Option<Vec<u8>> {
    let format = register_format("PNG");
    if format == 0 {
        return None;
    }
    read_format_bytes(format)
}

fn read_html() -> Option<Vec<u8>> {
    let format = register_format("HTML Format");
    if format == 0 {
        return None;
    }
    read_format_bytes(format)
}

fn read_rtf() -> Option<Vec<u8>> {
    let format = register_format("Rich Text Format");
    if format == 0 {
        return None;
    }
    read_format_bytes(format)
}

fn read_clipboard() -> Option<Captured> {
    if !open_clipboard_with_retry() {
        tracing::warn!("剪贴板被其他程序占用，本次事件已跳过");
        return None;
    }

    let result = {
        if has_private_marker() {
            None
        } else {
            Some(Captured {
                text: read_text(),
                html: read_html(),
                rtf: read_rtf(),
                files: read_files(),
                png: read_png(),
            })
        }
    };

    unsafe { CloseClipboard() };

    result.filter(|captured| {
        captured.text.is_some()
            || captured.html.is_some()
            || captured.rtf.is_some()
            || captured.files.is_some()
            || captured.png.is_some()
    })
}

// ---------------------------------------------------------------------------
// 规范化
// ---------------------------------------------------------------------------

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

fn content_hash(primary_type: &str, payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(primary_type.as_bytes());
    hasher.update([0u8]);
    hasher.update(payload);
    format!("{:x}", hasher.finalize())
}

fn one_line(text: &str, limit: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > limit {
        let truncated: String = flat.chars().take(limit).collect();
        format!("{truncated}…")
    } else {
        flat
    }
}

/// 从 CF_HTML 里粗略取出正文，仅用于生成预览与搜索文本
fn html_to_text(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let body = match (text.find("<body"), text.rfind("</body>")) {
        (Some(start), Some(end)) if end > start => &text[start..end],
        _ => &text[..],
    };

    let mut result = String::with_capacity(body.len());
    let mut in_tag = false;
    for ch in body.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

/// 常见位图扩展名，命中才为文件条目生成缩略图
fn is_image_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// 缩略图文件名键：路径 + 修改时间。同一路径内容变化后缩略图自动重生成
fn file_thumb_key(path: &str) -> String {
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(mtime.to_string().as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

/// 生成缩略图。宽 ≤512px：卡片内容区约 330 CSS 像素，按 1.5 倍 DPI 余量取 512，
/// 缩略图与显示尺寸 1:1 才不会糊。文件过大或图片异常时静默跳过，前端回退加载原图。
fn generate_thumbnail(source: &str, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
    const MAX_SOURCE_BYTES: u64 = 10 * 1024 * 1024;
    const THUMB_WIDTH: u32 = 512;
    const THUMB_HEIGHT_LIMIT: u32 = 512;

    if std::fs::metadata(source)?.len() > MAX_SOURCE_BYTES {
        return Ok(());
    }

    let image = image::open(source)?;
    let (width, height) = (image.width(), image.height());
    // 像素总量异常的图不解码，防御畸形 PNG 声称的超大尺寸
    if width == 0 || height == 0 || width.saturating_mul(height) > 4096 * 4096 {
        return Ok(());
    }

    let thumb = image.thumbnail(THUMB_WIDTH, THUMB_HEIGHT_LIMIT);
    thumb.save(dest)?;
    Ok(())
}

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

struct BuiltItem {
    item: NewItem,
    hash: String,
}

fn build_item(
    captured: Captured,
    source_app: Option<String>,
    data_dir: &std::path::Path,
) -> Option<BuiltItem> {
    let (primary_type, preview, search_text, formats, hash) = if let Some(files) = captured.files {
        let payload = files.join("\n");
        let preview = if files.len() == 1 {
            file_name(&files[0]).to_string()
        } else {
            format!("{} 个文件 · {}", files.len(), file_name(&files[0]))
        };
        let hash = content_hash("files", payload.as_bytes());
        let content = serde_json::to_vec(&files).ok()?;
        let byte_size = content.len() as i64;

        let mut formats = vec![NewFormat {
            format_name: "files/paths".into(),
            win32_format: Some(CF_HDROP),
            storage_type: "inline".into(),
            content: Some(content),
            file_path: None,
            byte_size,
        }];

        // 复制的是图片文件时生成缩略图，让卡片直接预览内容而非只显示文件名
        if let Some(first) = files.first() {
            if is_image_path(first) {
                let thumb = data_dir
                    .join("images")
                    .join(format!("f{}.thumb.png", file_thumb_key(first)));
                let source = first.clone();
                let dest = thumb.to_string_lossy().to_string();
                std::thread::spawn(move || {
                    if let Err(error) = generate_thumbnail(&source, &dest) {
                        tracing::warn!("文件缩略图生成失败：{error}");
                    }
                });
                formats.push(NewFormat {
                    // 这个名字不会参与写回剪贴板，仅用于列表查询时定位缩略图文件
                    format_name: "image/thumbnail".into(),
                    win32_format: None,
                    storage_type: "file".into(),
                    content: None,
                    file_path: Some(thumb.to_string_lossy().to_string()),
                    byte_size: 0,
                });
            }
        }

        ("files", preview, payload, formats, hash)
    } else if let Some(png) = captured.png {
        let preview = match png_dimensions(&png) {
            Some((width, height)) => format!("图片 {width} × {height}"),
            None => "图片".to_string(),
        };
        let hash = content_hash("image", &png);

        let file_path = data_dir.join("images").join(format!("{hash}.png"));
        std::fs::write(&file_path, &png).ok()?;

        // 缩略图后台生成：解码大图可能上百毫秒，不能卡住监听线程。
        // 生成完成前前端回退到加载原图。
        let thumb_path = file_path.with_extension("thumb.png");
        if let Some(source) = file_path.to_str().map(str::to_string) {
            let dest = thumb_path.to_string_lossy().to_string();
            std::thread::spawn(move || {
                if let Err(error) = generate_thumbnail(&source, &dest) {
                    tracing::warn!("缩略图生成失败：{error}");
                }
            });
        }

        (
            "image",
            preview,
            String::new(),
            vec![NewFormat {
                format_name: "image/png".into(),
                win32_format: None,
                storage_type: "file".into(),
                content: None,
                file_path: Some(file_path.to_string_lossy().to_string()),
                byte_size: png.len() as i64,
            }],
            hash,
        )
    } else {
        let plain = captured
            .text
            .as_deref()
            .map(normalize)
            .filter(|text| !text.is_empty());
        let html_text = captured
            .html
            .as_deref()
            .map(html_to_text)
            .map(|text| normalize(&text))
            .filter(|text| !text.is_empty());

        let display = plain
            .clone()
            .or_else(|| html_text.clone())
            .unwrap_or_default();
        if display.is_empty() && captured.rtf.is_none() {
            return None;
        }

        let primary_type = if captured.html.is_some() {
            "html"
        } else if captured.rtf.is_some() {
            "rtf"
        } else {
            "text"
        };

        let hash = content_hash(primary_type, display.as_bytes());
        let mut formats = Vec::new();

        if let Some(text) = &plain {
            formats.push(NewFormat {
                format_name: "text/plain".into(),
                win32_format: Some(CF_UNICODETEXT),
                storage_type: "inline".into(),
                content: Some(text.as_bytes().to_vec()),
                file_path: None,
                byte_size: text.len() as i64,
            });
        }
        if let Some(html) = captured.html {
            formats.push(NewFormat {
                format_name: "text/html".into(),
                win32_format: None,
                storage_type: "inline".into(),
                byte_size: html.len() as i64,
                content: Some(html),
                file_path: None,
            });
        }
        if let Some(rtf) = captured.rtf {
            formats.push(NewFormat {
                format_name: "application/rtf".into(),
                win32_format: None,
                storage_type: "inline".into(),
                byte_size: rtf.len() as i64,
                content: Some(rtf),
                file_path: None,
            });
        }

        let preview = one_line(&display, PREVIEW_LIMIT);
        let search_text: String = display.chars().take(SEARCH_TEXT_LIMIT).collect();
        (primary_type, preview, search_text, formats, hash)
    };

    Some(BuiltItem {
        item: NewItem {
            content_hash: hash.clone(),
            primary_type: primary_type.to_string(),
            preview,
            search_text,
            source_app,
            formats,
        },
        hash,
    })
}

// ---------------------------------------------------------------------------
// 写入（粘贴时把历史条目写回剪贴板）
// ---------------------------------------------------------------------------

fn set_clipboard_bytes(format: u32, bytes: &[u8]) -> bool {
    unsafe {
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
        if hglobal.is_null() {
            return false;
        }
        let ptr = GlobalLock(hglobal) as *mut u8;
        if ptr.is_null() {
            GlobalFree(hglobal);
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        GlobalUnlock(hglobal);

        if SetClipboardData(format, hglobal).is_null() {
            GlobalFree(hglobal);
            return false;
        }
        // 成功后内存所有权归系统，不能再释放
        true
    }
}

fn set_clipboard_text(format: u32, text: &str) -> bool {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    let bytes: Vec<u8> = units.iter().flat_map(|unit| unit.to_ne_bytes()).collect();
    set_clipboard_bytes(format, &bytes)
}

fn set_clipboard_files(paths: &[String]) -> bool {
    // DROPFILES 结构：20 字节头部 + UTF-16 路径列表 + 双 null 结尾
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&20u32.to_ne_bytes()); // pFiles：文件列表相对偏移
    data.extend_from_slice(&[0u8; 8]); // pt (POINT)
    data.extend_from_slice(&0u32.to_ne_bytes()); // fNC
    data.extend_from_slice(&1u32.to_ne_bytes()); // fWide：使用 UTF-16
    for path in paths {
        for unit in path.encode_utf16() {
            data.extend_from_slice(&unit.to_ne_bytes());
        }
        data.extend_from_slice(&0u16.to_ne_bytes());
    }
    data.extend_from_slice(&0u16.to_ne_bytes());
    set_clipboard_bytes(CF_HDROP, &data)
}

pub fn write_formats(formats: &[StoredFormat]) -> Result<(), String> {
    if !open_clipboard_with_retry() {
        return Err("剪贴板正被其他程序占用".into());
    }

    let result = (|| -> Result<(), String> {
        if unsafe { EmptyClipboard() } == 0 {
            return Err("清空剪贴板失败".into());
        }

        for format in formats {
            match format.format_name.as_str() {
                "text/plain" => {
                    if let Some(bytes) = &format.content {
                        let text = String::from_utf8_lossy(bytes);
                        set_clipboard_text(CF_UNICODETEXT, &text);
                    }
                }
                "text/html" => {
                    if let Some(bytes) = &format.content {
                        let id = register_format("HTML Format");
                        if id != 0 {
                            set_clipboard_bytes(id, bytes);
                        }
                    }
                }
                "application/rtf" => {
                    if let Some(bytes) = &format.content {
                        let id = register_format("Rich Text Format");
                        if id != 0 {
                            set_clipboard_bytes(id, bytes);
                        }
                    }
                }
                "image/png" => {
                    let bytes = match (&format.content, &format.file_path) {
                        (Some(content), _) => Some(content.clone()),
                        (None, Some(path)) => std::fs::read(path).ok(),
                        _ => None,
                    };
                    if let Some(bytes) = bytes {
                        let id = register_format("PNG");
                        if id != 0 {
                            set_clipboard_bytes(id, &bytes);
                        }
                    }
                }
                "files/paths" => {
                    if let Some(bytes) = &format.content {
                        if let Ok(paths) = serde_json::from_slice::<Vec<String>>(bytes) {
                            set_clipboard_files(&paths);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })();

    unsafe { CloseClipboard() };
    result
}

// ---------------------------------------------------------------------------
// 监听
// ---------------------------------------------------------------------------

pub fn spawn_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("clipboard-watcher".into())
        .spawn(move || {
            let mut last_sequence = unsafe { GetClipboardSequenceNumber() };
            tracing::info!("剪贴板监听已启动");

            loop {
                std::thread::sleep(POLL_INTERVAL);

                let sequence = unsafe { GetClipboardSequenceNumber() };
                if sequence == 0 || sequence == last_sequence {
                    continue;
                }
                last_sequence = sequence;

                let source_hwnd = util::foreground_window();
                let Some(captured) = read_clipboard() else {
                    continue;
                };

                let state = app.state::<AppState>();
                let source_app = util::process_name(source_hwnd);
                let Some(built) = build_item(captured, source_app, &state.data_dir) else {
                    continue;
                };

                // 自己写回剪贴板触发的变化不记录
                if state.consume_self_write(&built.hash) {
                    continue;
                }

                let settings = state.settings();
                let inserted = {
                    let mut conn = state.db.lock().unwrap();
                    let result = db::insert(&mut conn, built.item);
                    // 容量清理失败不影响本次记录
                    let _ = db::enforce_capacity(&mut conn, settings.max_unpinned_items);
                    result
                };

                match inserted {
                    // 新增卡片、或把同内容的旧卡片提到最前，都要让界面重取一次列表
                    Ok(_) => {
                        let _ = app.emit("history-changed", ());
                    }
                    Err(error) => tracing::error!("写入历史失败：{error}"),
                }
            }
        })
        .ok();
}
