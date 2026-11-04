/**
 * 内容识别：判断一条文本实际上是什么东西。
 * 剪贴板里最常见的几类是链接、颜色值、代码片段、文件路径、邮箱，
 * 它们的显示方式应该不一样，而不是全部按正文字体铺开。
 */

export type ContentKind = "text" | "url" | "color" | "email" | "code" | "path";

const HEX_COLOR = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const FUNC_COLOR = /^(rgb|hsl)a?\(\s*[\d.,%\s/]+\)$/i;
const URL_ONLY = /^(https?:\/\/|www\.)\S+$/i;
const EMAIL_ONLY = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
const WINDOWS_PATH = /^([a-z]:\\|\\\\)[^<>|?*\n]*$/i;
const UNIX_PATH = /^\/(usr|home|etc|var|opt|tmp|mnt|Users)\/\S*$/;

export function detectContent(raw: string): ContentKind {
  const text = raw.trim();
  if (!text) return "text";

  if (HEX_COLOR.test(text) || FUNC_COLOR.test(text)) return "color";
  if (URL_ONLY.test(text)) return "url";
  if (EMAIL_ONLY.test(text)) return "email";
  if (WINDOWS_PATH.test(text) || UNIX_PATH.test(text)) return "path";
  if (looksLikeCode(text)) return "code";

  return "text";
}

/** 以编程关键字开头，或 ASCII 符号密度高且含语法符号 */
function looksLikeCode(text: string): boolean {
  if (
    /^\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER)\b/i.test(text) ||
    /^\s*(import|export|const|let|var|function|class|def|fn|struct|impl)\b/.test(text) ||
    /^\s*[{[]/.test(text)
  ) {
    return true;
  }

  const ascii = text.replace(/[^\x20-\x7E]/g, "").length;
  const ratio = ascii / text.length;
  return ratio > 0.72 && text.length < 600 && /[;{}()=]/.test(text);
}

/** 判断色值是否会解析成偏亮的颜色，用于决定色块是否需要描边 */
export function isLightColor(hex: string): boolean {
  const value = hex.replace("#", "");
  const full =
    value.length === 3
      ? value
          .split("")
          .map((char) => char + char)
          .join("")
      : value;
  if (full.length < 6) return false;

  const r = parseInt(full.slice(0, 2), 16);
  const g = parseInt(full.slice(2, 4), 16);
  const b = parseInt(full.slice(4, 6), 16);
  // 相对亮度，阈值取人眼舒适区间
  return (r * 299 + g * 587 + b * 114) / 1000 > 200;
}
