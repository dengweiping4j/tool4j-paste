import type { ItemDto, PrimaryType } from "./types";
import { detectContent, isLightColor } from "./content";
import { displayAppName } from "./accent";

/**
 * 卡片视觉定义：类型决定色带颜色与顶部标签，内容决定底部标题。
 * 配色与结构参照 macOS Paste：色带在卡片顶部整块铺开，标签与时间压在色带上。
 */
export type CardKind = "text" | "link" | "color" | "image" | "files" | "rich";

export interface CardVisual {
  kind: CardKind;
  /** 色带上的类型名 */
  label: string;
  /** 色带底色 */
  band: string;
  /** 色带上的文字用深色还是浅色 */
  darkInk: boolean;
  /** 底部主标题 */
  title: string;
  /** 底部次标题 */
  subtitle: string;
}

const BAND_COLORS: Record<CardKind, string> = {
  text: "#3B82F6",
  link: "#0EA5A4",
  color: "#3B82F6",
  image: "#F59E0B",
  files: "#64748B",
  rich: "#8B5CF6",
};

const LABELS: Record<CardKind, string> = {
  text: "文本",
  link: "链接",
  color: "颜色",
  image: "图片",
  files: "文件",
  rich: "富文本",
};

function baseKind(type: PrimaryType): CardKind {
  switch (type) {
    case "image":
      return "image";
    case "files":
      return "files";
    case "html":
    case "rtf":
      return "rich";
    default:
      return "text";
  }
}

function hostOf(url: string): string {
  try {
    const parsed = new URL(url.startsWith("http") ? url : `https://${url}`);
    return parsed.hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function charLabel(text: string): string {
  const length = text.trim().length;
  return length >= 200 ? "200+ 字符" : `${length} 字符`;
}

export function cardVisual(item: ItemDto): CardVisual {
  const kind = baseKind(item.primary_type);
  const source = item.source_app ? displayAppName(item.source_app) : "";

  // 图片与文件不再细分内容，纯文本则按链接 / 色值重新归类
  if (kind === "text") {
    const detected = detectContent(item.preview);

    if (detected === "url") {
      const url = item.preview.trim();
      return {
        kind: "link",
        label: LABELS.link,
        band: BAND_COLORS.link,
        darkInk: false,
        title: hostOf(url),
        subtitle: source || url.replace(/^https?:\/\//, "").slice(0, 40),
      };
    }

    if (detected === "color") {
      const value = item.preview.trim();
      const light = isLightColor(value);
      return {
        kind: "color",
        label: LABELS.color,
        band: value,
        darkInk: light,
        title: value,
        subtitle: source,
      };
    }

    return {
      kind: "text",
      label: LABELS.text,
      band: BAND_COLORS.text,
      darkInk: false,
      title: charLabel(item.preview),
      subtitle: source,
    };
  }

  const band = BAND_COLORS[kind];
  let title = charLabel(item.preview);

  if (kind === "image") {
    title = item.preview.replace(/^图片\s*/, "") || "图片";
  } else if (kind === "files") {
    title = item.preview;
  }

  return {
    kind,
    label: LABELS[kind],
    band,
    darkInk: false,
    title,
    subtitle: source,
  };
}
