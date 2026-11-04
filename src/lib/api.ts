import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ItemDto, PinboardDto, SettingsDto } from "./types";
import { DEFAULT_SETTINGS } from "./types";
import { MOCK_ITEMS } from "./mock";

/**
 * 唯一的后端调用入口。组件不直接使用 invoke，
 * 这样在浏览器里 npm run dev 也能完整看到界面（走示例数据）。
 */
export const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let mockStore: ItemDto[] = [...MOCK_ITEMS];
let mockSettings: SettingsDto = { ...DEFAULT_SETTINGS };

/** 示例数据模拟后端的排序规则：钉住区在前，同一区段内最近用过的靠前 */
function sortMock() {
  mockStore = [...mockStore].sort((a, b) => {
    if (a.is_pinned !== b.is_pinned) return a.is_pinned ? -1 : 1;
    return b.last_used_at - a.last_used_at;
  });
}

sortMock();

/** 模拟网络/数据库延迟，让加载态在浏览器里也能被看到 */
function delay(ms = 60): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function page(items: ItemDto[], offset: number, limit: number): ItemDto[] {
  return items.slice(offset, offset + limit);
}

/** 后端返回的是磁盘路径，需要转成 webview 可加载的 asset 地址 */
function toAssetUrl(item: ItemDto): ItemDto {
  if (!item.thumbnail_path) return item;
  return { ...item, thumbnail_path: convertFileSrc(item.thumbnail_path) };
}

export async function listItems(
  pinboardId: number | null,
  offset = 0,
  limit = 50,
): Promise<ItemDto[]> {
  if (!IS_TAURI) return page(mockStore, offset, limit);
  const rows = await invoke<ItemDto[]>("list_items", { pinboardId, offset, limit });
  return rows.map(toAssetUrl);
}

export async function searchItems(
  query: string,
  pinboardId: number | null,
  offset = 0,
  limit = 50,
): Promise<ItemDto[]> {
  if (!IS_TAURI) {
    await delay(30);
    const needle = query.trim().toLowerCase();
    if (!needle) return page(mockStore, offset, limit);
    const hits = mockStore.filter((item) =>
      item.preview.toLowerCase().includes(needle),
    );
    return page(hits, offset, limit);
  }
  const rows = await invoke<ItemDto[]>("search_items", {
    query,
    pinboardId,
    offset,
    limit,
  });
  return rows.map(toAssetUrl);
}

/** 示例数据：用过的条目刷新使用时间，浮到所属区段最前面 */
export async function pasteItem(id: number): Promise<void> {
  if (!IS_TAURI) {
    const target = mockStore.find((item) => item.id === id);
    if (target) {
      target.last_used_at = Date.now();
      sortMock();
    }
    return;
  }
  return invoke("paste_item", { id });
}

export async function togglePin(id: number): Promise<boolean> {
  if (!IS_TAURI) {
    const target = mockStore.find((item) => item.id === id);
    if (!target) return false;
    target.is_pinned = !target.is_pinned;
    sortMock();
    return target.is_pinned;
  }
  return invoke<boolean>("toggle_pin", { id });
}

export async function deleteItem(id: number): Promise<void> {
  if (!IS_TAURI) {
    mockStore = mockStore.filter((item) => item.id !== id);
    return;
  }
  return invoke("delete_item", { id });
}

/** 清空未钉住的记录，返回实际删除条数；钉住的条目不受影响 */
export async function clearUnpinned(): Promise<number> {
  if (!IS_TAURI) {
    const before = mockStore.length;
    mockStore = mockStore.filter((item) => item.is_pinned);
    return before - mockStore.length;
  }
  return invoke<number>("clear_unpinned");
}

export async function getSettings(): Promise<SettingsDto> {
  if (!IS_TAURI) return { ...mockSettings };
  return invoke<SettingsDto>("get_settings");
}

export async function updateSettings(settings: SettingsDto): Promise<void> {
  if (!IS_TAURI) {
    mockSettings = { ...settings };
    return;
  }
  return invoke("update_settings", { settings });
}

/** 隐藏面板（不回退到托盘退出） */
export async function hidePanel(): Promise<void> {
  if (!IS_TAURI) {
    console.info("[mock] hide panel");
    return;
  }
  await getCurrentWindow().hide();
}

// ---------------------------------------------------------------------------
// Pinboard 分组
// ---------------------------------------------------------------------------

export async function listPinboards(): Promise<PinboardDto[]> {
  if (!IS_TAURI) return [];
  return invoke<PinboardDto[]>("list_pinboards");
}

export async function createPinboard(name: string): Promise<PinboardDto> {
  return invoke<PinboardDto>("create_pinboard", { name });
}

export async function renamePinboard(id: number, name: string): Promise<void> {
  return invoke("rename_pinboard", { id, name });
}

export async function deletePinboard(id: number): Promise<void> {
  return invoke("delete_pinboard", { id });
}

export async function assignPinboard(
  itemId: number,
  pinboardId: number | null,
): Promise<void> {
  return invoke("assign_pinboard", { itemId, pinboardId });
}

export async function openAboutPage(): Promise<void> {
  if (!IS_TAURI) {
    console.info("[mock] open https://www.tool4j.com");
    return;
  }
  return invoke("open_about_page");
}
