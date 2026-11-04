/**
 * 与 Rust 后端 IPC 共享的数据结构。
 * 字段名保持 snake_case，与后端 serde 默认序列化一致，避免前端反复转换。
 */

export type PrimaryType = "text" | "html" | "rtf" | "image" | "files";

export interface ItemDto {
  id: number;
  primary_type: PrimaryType;
  /** 列表展示用的一行预览，最多 200 字符 */
  preview: string;
  /** 图片类型返回缩略图本地路径，其余为 null */
  thumbnail_path: string | null;
  /** 该条目保存了哪些格式，例如 ["text/plain", "text/html"] */
  formats: string[];
  source_app: string | null;
  is_pinned: boolean;
  pinboard_id: number | null;
  /** Unix 毫秒，内容第一次进入历史的时刻 */
  created_at: number;
  /** Unix 毫秒，最近一次被使用（粘贴或复制）的时刻；列表排序和卡片时间都以它为准 */
  last_used_at: number;
}

export interface PinboardDto {
  id: number;
  name: string;
  color: string | null;
  sort_order: number;
}

export type FilterKind = "all" | "text" | "image" | "files" | "pinned";

export interface SettingsDto {
  hotkey: string;
  auto_paste: boolean;
  max_unpinned_items: number;
  launch_at_login: boolean;
}

export const DEFAULT_SETTINGS: SettingsDto = {
  hotkey: "Ctrl+Alt+V",
  auto_paste: true,
  max_unpinned_items: 10000,
  launch_at_login: true,
};
