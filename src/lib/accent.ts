import type { PrimaryType } from "./types";

/**
 * 卡片配色：来源应用决定主色（顶部色条 + 来源图标），内容类型决定辅助色（类型标签）。
 * 同一来源在任何时候都得到同一个颜色，便于凭颜色识别来源。
 */

/** 调色板：中等饱和度，深色与浅色主题下都能辨认 */
const PALETTE = [
  "#4C9AFF", // 蓝
  "#34C759", // 绿
  "#FF9F0A", // 橙
  "#BF5AF2", // 紫
  "#FF6482", // 粉
  "#32ADE6", // 青
  "#E5A800", // 琥珀
  "#FF453A", // 红
  "#5E5CE6", // 靛
  "#2AA198", // 蓝绿
];

/** 常见应用的品牌色，比哈希取色更像"它自己的颜色" */
const KNOWN_SOURCES: Record<string, string> = {
  "chrome.exe": "#4C9AFF",
  "msedge.exe": "#0FA3D9",
  "firefox.exe": "#FF7139",
  "code.exe": "#3B9EFF",
  "wechat.exe": "#34C759",
  "weixin.exe": "#34C759",
  "qq.exe": "#32ADE6",
  "dingtalk.exe": "#3B9EFF",
  "windowsterminal.exe": "#8E8E93",
  "cmd.exe": "#8E8E93",
  "powershell.exe": "#32ADE6",
  "explorer.exe": "#E5A800",
  "winword.exe": "#2B579A",
  "excel.exe": "#217346",
  "powerpnt.exe": "#D24726",
  "outlook.exe": "#0F6CBD",
  "notepad.exe": "#8E8E93",
  "snippingtool.exe": "#FF6482",
  "photoshop.exe": "#31A8FF",
  "figma.exe": "#BF5AF2",
};

/** 来源应用的展示名 */
const DISPLAY_NAMES: Record<string, string> = {
  "chrome.exe": "Chrome",
  "msedge.exe": "Edge",
  "firefox.exe": "Firefox",
  "code.exe": "VS Code",
  "wechat.exe": "微信",
  "weixin.exe": "微信",
  "qq.exe": "QQ",
  "dingtalk.exe": "钉钉",
  "windowsterminal.exe": "终端",
  "cmd.exe": "命令提示符",
  "powershell.exe": "PowerShell",
  "explorer.exe": "资源管理器",
  "winword.exe": "Word",
  "excel.exe": "Excel",
  "powerpnt.exe": "PowerPoint",
  "outlook.exe": "Outlook",
  "notepad.exe": "记事本",
  "snippingtool.exe": "截图工具",
  "photoshop.exe": "Photoshop",
  "figma.exe": "Figma",
};

function hashString(input: string): number {
  let hash = 5381;
  for (let index = 0; index < input.length; index += 1) {
    hash = ((hash << 5) + hash + input.charCodeAt(index)) >>> 0;
  }
  return hash;
}

/** 来源主色：已知应用用品牌色，其余按进程名稳定哈希到调色板 */
export function sourceColor(app: string | null): string {
  if (!app) return "#8E8E93";
  const key = app.toLowerCase();
  const known = KNOWN_SOURCES[key];
  if (known) return known;
  return PALETTE[hashString(key) % PALETTE.length];
}

/** 内容类型辅助色 */
export function typeColor(type: PrimaryType): string {
  switch (type) {
    case "html":
    case "rtf":
      return "#FF9F0A";
    case "image":
      return "#34C759";
    case "files":
      return "#E5A800";
    default:
      return "#8E8E93";
  }
}

/** 进程名转用户看得懂的来源名称 */
export function displayAppName(app: string | null): string {
  if (!app) return "未知来源";
  const key = app.toLowerCase();
  const known = DISPLAY_NAMES[key];
  if (known) return known;
  return app.replace(/\.exe$/i, "");
}

/** 生成带透明度的颜色，用于图标底色这类需要淡色的场合 */
export function withAlpha(hex: string, alpha: number): string {
  const value = hex.replace("#", "");
  const r = parseInt(value.slice(0, 2), 16);
  const g = parseInt(value.slice(2, 4), 16);
  const b = parseInt(value.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
