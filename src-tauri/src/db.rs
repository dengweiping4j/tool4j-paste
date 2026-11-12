use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::state::Settings;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS items (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content_hash  TEXT    NOT NULL,
    primary_type  TEXT    NOT NULL,
    preview       TEXT    NOT NULL,
    search_text   TEXT    NOT NULL DEFAULT '',
    source_app    TEXT,
    is_pinned     INTEGER NOT NULL DEFAULT 0,
    pinboard_id   INTEGER,
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER NOT NULL,
    use_count     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_items_created ON items(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_items_hash    ON items(content_hash);
-- 列表按「钉住优先、最近使用靠前」排序，索引跟着走，老库也重建一次
DROP INDEX IF EXISTS idx_items_pinned;
CREATE INDEX IF NOT EXISTS idx_items_order ON items(is_pinned DESC, last_used_at DESC);

CREATE TABLE IF NOT EXISTS item_formats (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id         INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    format_name     TEXT    NOT NULL,
    win32_format    INTEGER,
    storage_type    TEXT    NOT NULL,
    content         BLOB,
    file_path       TEXT,
    byte_size       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_formats_item ON item_formats(item_id);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pinboards (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    color      TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);
"#;

/// 列表查询统一返回这个结构，字段名与前端 ItemDto 保持一致。
#[derive(Debug, Serialize)]
pub struct ItemDto {
    pub id: i64,
    pub primary_type: String,
    pub preview: String,
    pub thumbnail_path: Option<String>,
    pub formats: Vec<String>,
    pub source_app: Option<String>,
    pub is_pinned: bool,
    pub pinboard_id: Option<i64>,
    pub created_at: i64,
    /// 最近一次被使用（粘贴或复制）的时间，列表排序与卡片时间都以它为准
    pub last_used_at: i64,
}

pub struct NewFormat {
    pub format_name: String,
    pub win32_format: Option<u32>,
    pub storage_type: String,
    pub content: Option<Vec<u8>>,
    pub file_path: Option<String>,
    pub byte_size: i64,
}

pub struct NewItem {
    pub content_hash: String,
    pub primary_type: String,
    pub preview: String,
    pub search_text: String,
    pub source_app: Option<String>,
    pub formats: Vec<NewFormat>,
}

const SELECT_COLUMNS: &str = r#"
SELECT
    i.id,
    i.primary_type,
    i.preview,
    i.source_app,
    i.is_pinned,
    i.pinboard_id,
    i.created_at,
    i.last_used_at,
    (SELECT f.file_path FROM item_formats f
      WHERE f.item_id = i.id AND f.storage_type = 'file' LIMIT 1) AS thumbnail_path,
    (SELECT group_concat(f.format_name, '|') FROM item_formats f
      WHERE f.item_id = i.id) AS formats
FROM items i
"#;

pub fn open(dir: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(dir.join("tool4j_paste.db"))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn row_to_dto(row: &rusqlite::Row) -> rusqlite::Result<ItemDto> {
    let formats: Option<String> = row.get("formats")?;
    // 优先用缩略图；生成是异步的，文件还没落盘时返回 None，前端显示类型占位
    let thumbnail_path = row
        .get::<_, Option<String>>("thumbnail_path")?
        .and_then(|path| {
            let thumb = path.replace(".png", ".thumb.png");
            let candidate = if std::path::Path::new(&thumb).exists() {
                thumb
            } else {
                path
            };
            if std::path::Path::new(&candidate).exists() {
                Some(candidate)
            } else {
                None
            }
        });

    Ok(ItemDto {
        id: row.get("id")?,
        primary_type: row.get("primary_type")?,
        preview: row.get("preview")?,
        thumbnail_path,
        formats: formats
            .map(|raw| raw.split('|').map(str::to_string).collect())
            .unwrap_or_default(),
        source_app: row.get("source_app")?,
        is_pinned: row.get::<_, i64>("is_pinned")? != 0,
        pinboard_id: row.get("pinboard_id")?,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
    })
}

/// 列表排序：钉住的内容始终在前，同一区段内最近用过的靠前。
/// 用过的条目会刷新 last_used_at，所以点一次它就浮到所属区段的最前面。
const ORDER_BY: &str = "ORDER BY i.is_pinned DESC, i.last_used_at DESC, i.id DESC";

pub fn list(
    conn: &Connection,
    pinboard_id: Option<i64>,
    offset: u32,
    limit: u32,
) -> rusqlite::Result<Vec<ItemDto>> {
    let (sql, values): (String, Vec<rusqlite::types::Value>) = match pinboard_id {
        Some(id) => (
            format!("{SELECT_COLUMNS} WHERE i.pinboard_id = ?1 {ORDER_BY} LIMIT ?2 OFFSET ?3"),
            vec![id.into(), (limit as i64).into(), (offset as i64).into()],
        ),
        None => (
            format!("{SELECT_COLUMNS} {ORDER_BY} LIMIT ?1 OFFSET ?2"),
            vec![(limit as i64).into(), (offset as i64).into()],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values), row_to_dto)?;
    rows.collect()
}

pub fn search(
    conn: &Connection,
    keyword: &str,
    pinboard_id: Option<i64>,
    offset: u32,
    limit: u32,
) -> rusqlite::Result<Vec<ItemDto>> {
    let pattern = format!("%{}%", escape_like(keyword.trim()));

    let (sql, values): (String, Vec<rusqlite::types::Value>) = match pinboard_id {
        Some(id) => (
            format!(
                "{SELECT_COLUMNS}
                 WHERE i.search_text LIKE ?1 ESCAPE '\\' AND i.pinboard_id = ?2
                 {ORDER_BY}
                 LIMIT ?3 OFFSET ?4"
            ),
            vec![
                pattern.into(),
                id.into(),
                (limit as i64).into(),
                (offset as i64).into(),
            ],
        ),
        None => (
            format!(
                "{SELECT_COLUMNS}
                 WHERE i.search_text LIKE ?1 ESCAPE '\\'
                 {ORDER_BY}
                 LIMIT ?2 OFFSET ?3"
            ),
            vec![
                pattern.into(),
                (limit as i64).into(),
                (offset as i64).into(),
            ],
        ),
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values), row_to_dto)?;
    rows.collect()
}

/// LIKE 通配符需要转义，否则用户搜索 `%` 或 `_` 会命中全部记录
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 返回 Some(id) 表示新插入；None 表示内容已在历史中，仅把它刷新到列表最前。
pub fn insert(conn: &mut Connection, item: NewItem) -> rusqlite::Result<Option<i64>> {
    let now = now_ms();

    // 在全部历史里按内容指纹查找，而不是只比对上一条。
    // 命中就刷新使用时间，列表按 last_used_at 排序，卡片自然回到最前面。
    // 旧库里若已有多张同内容卡片，固定命中最早那张，行为稳定可预期。
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM items WHERE content_hash = ?1 ORDER BY id ASC LIMIT 1",
            params![item.content_hash],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE items SET last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        return Ok(None);
    }

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO items
            (content_hash, primary_type, preview, search_text, source_app, created_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            item.content_hash,
            item.primary_type,
            item.preview,
            item.search_text,
            item.source_app,
            now
        ],
    )?;
    let id = tx.last_insert_rowid();

    for format in &item.formats {
        tx.execute(
            "INSERT INTO item_formats
                (item_id, format_name, win32_format, storage_type, content, file_path, byte_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                format.format_name,
                format.win32_format,
                format.storage_type,
                format.content,
                format.file_path,
                format.byte_size
            ],
        )?;
    }

    tx.commit()?;
    Ok(Some(id))
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE items SET is_pinned = ?1 WHERE id = ?2",
        params![if pinned { 1 } else { 0 }, id],
    )?;
    Ok(())
}

pub fn touch(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE items SET last_used_at = ?1, use_count = use_count + 1 WHERE id = ?2",
        params![now_ms(), id],
    )?;
    Ok(())
}

/// 取条目的内容哈希，写回剪贴板时用于自记录抑制
pub fn content_hash_of(conn: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT content_hash FROM items WHERE id = ?1", params![id], |row| {
        row.get(0)
    })
    .optional()
}

fn file_paths_of(conn: &Connection, id: i64) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT file_path FROM item_formats WHERE item_id = ?1 AND file_path IS NOT NULL")?;
    let rows = stmt.query_map(params![id], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn delete(conn: &mut Connection, id: i64) -> rusqlite::Result<()> {
    let files = file_paths_of(conn, id)?;
    conn.execute("DELETE FROM items WHERE id = ?1", params![id])?;
    for path in files {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

pub fn clear_unpinned(conn: &mut Connection) -> rusqlite::Result<u64> {
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM items WHERE is_pinned = 0")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<i64>>>()?
    };
    let total = ids.len() as u64;
    for id in ids {
        delete(conn, id)?;
    }
    Ok(total)
}

/// 超过上限时删除最旧的未钉住记录，回落到上限的 90%
pub fn enforce_capacity(conn: &mut Connection, max: i64) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE is_pinned = 0",
        [],
        |row| row.get(0),
    )?;
    if count <= max {
        return Ok(());
    }

    let target = (max * 9) / 10;
    let surplus = count - target;
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT id FROM items WHERE is_pinned = 0 ORDER BY created_at ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![surplus], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<i64>>>()?
    };
    for id in ids {
        delete(conn, id)?;
    }
    Ok(())
}

pub fn load_settings(conn: &Connection) -> Option<Settings> {
    let raw: String = conn
        .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
            row.get(0)
        })
        .ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> rusqlite::Result<()> {
    let raw = serde_json::to_string(settings).unwrap_or_else(|_| "{}".into());
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('app', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![raw],
    )?;
    Ok(())
}

/// 取回条目在写回剪贴板时需要的全部格式内容
pub struct StoredFormat {
    pub format_name: String,
    pub content: Option<Vec<u8>>,
    pub file_path: Option<String>,
}

pub fn formats_of(conn: &Connection, id: i64) -> rusqlite::Result<Vec<StoredFormat>> {
    let mut stmt = conn.prepare(
        "SELECT format_name, content, file_path
         FROM item_formats WHERE item_id = ?1",
    )?;
    let rows = stmt.query_map(params![id], |row| {
        Ok(StoredFormat {
            format_name: row.get(0)?,
            content: row.get(1)?,
            file_path: row.get(2)?,
        })
    })?;
    rows.collect()
}

// ---------------------------------------------------------------------------
// Pinboard 分组
// ---------------------------------------------------------------------------

pub const PINBOARD_COLORS: [&str; 8] = [
    "#3B82F6", "#F59E0B", "#0EA5A4", "#EC4899", "#8B5CF6", "#34C759", "#FF453A", "#64748B",
];

#[derive(Debug, Serialize, Clone)]
pub struct PinboardDto {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
}

pub fn list_pinboards(conn: &Connection) -> rusqlite::Result<Vec<PinboardDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color FROM pinboards ORDER BY sort_order, id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PinboardDto {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn create_pinboard(
    conn: &mut Connection,
    name: &str,
) -> rusqlite::Result<PinboardDto> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pinboards", [], |r| r.get(0))?;
    let color = PINBOARD_COLORS[(count as usize) % PINBOARD_COLORS.len()];

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO pinboards (name, color, sort_order, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![name.trim(), color, count, now_ms()],
    )?;
    let id = tx.last_insert_rowid();
    tx.commit()?;

    Ok(PinboardDto {
        id,
        name: name.trim().to_string(),
        color: Some(color.to_string()),
    })
}

pub fn rename_pinboard(conn: &Connection, id: i64, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE pinboards SET name = ?1 WHERE id = ?2",
        params![name.trim(), id],
    )?;
    Ok(())
}

/// 删除分组：条目本身保留，只解除归属
pub fn delete_pinboard(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE items SET pinboard_id = NULL WHERE pinboard_id = ?1", params![id])?;
    conn.execute("DELETE FROM pinboards WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn assign_pinboard(
    conn: &Connection,
    item_id: i64,
    pinboard_id: Option<i64>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE items SET pinboard_id = ?1 WHERE id = ?2",
        params![pinboard_id, item_id],
    )?;
    Ok(())
}
