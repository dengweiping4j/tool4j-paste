import type { PrimaryType } from "./types";

/**
 * 相对时间：刚刚 / N 分钟前 / N 小时前 / 昨天 / MM月DD日
 * 在前端计算，列表刷新时才会随时间变化，避免后端反复计算。
 */
export function relativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (diff < minute) return "刚刚";
  if (diff < hour) return `${Math.floor(diff / minute)} 分钟前`;
  if (diff < 6 * hour) return `${Math.floor(diff / hour)} 小时前`;

  const date = new Date(timestamp);
  const now = new Date();
  const isToday =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (isToday) return `${pad(date.getHours())}:${pad(date.getMinutes())}`;

  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
  if (
    date.getFullYear() === yesterday.getFullYear() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getDate() === yesterday.getDate()
  ) {
    return "昨天";
  }

  if (date.getFullYear() === now.getFullYear()) {
    return `${date.getMonth() + 1}月${date.getDate()}日`;
  }
  return `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`;
}

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** 卡片色带上的时间：刚刚 / 3 分钟前 / 2 小时前 / 2 天前 / 4月8日 */
export function shortRelativeTime(timestamp: number): string {
  const diff = Math.max(0, Date.now() - timestamp);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (diff < minute) return "刚刚";
  if (diff < hour) return `${Math.floor(diff / minute)} 分钟前`;
  if (diff < day) return `${Math.floor(diff / hour)} 小时前`;
  // 超过一个月改用具体日期，「300 天前」这类说法对用户没有意义
  if (diff < 30 * day) return `${Math.floor(diff / day)} 天前`;

  const date = new Date(timestamp);
  const monthAndDay = `${date.getMonth() + 1}月${date.getDate()}日`;
  if (date.getFullYear() === new Date().getFullYear()) return monthAndDay;
  return `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`;
}

/** 类型的中文名称，用于筛选栏与提示 */
export function typeLabel(type: PrimaryType | "files"): string {
  switch (type) {
    case "text":
      return "文本";
    case "html":
      return "富文本";
    case "rtf":
      return "富文本";
    case "image":
      return "图片";
    case "files":
      return "文件";
  }
}

/** 把多行文本压成一行预览 */
export function toSingleLine(text: string, limit = 200): string {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > limit ? `${flat.slice(0, limit)}…` : flat;
}

/** 从文件路径数组里取文件名 */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}
