<script lang="ts">
  import TopBar from "./lib/components/TopBar.svelte";
  import HorizontalList from "./lib/components/HorizontalList.svelte";
  import ContextMenu from "./lib/components/ContextMenu.svelte";
  import SettingsPanel from "./lib/components/SettingsPanel.svelte";
  import ConfirmDialog from "./lib/components/ConfirmDialog.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import type { FilterKind, ItemDto, PinboardDto } from "./lib/types";
  import { listen } from "@tauri-apps/api/event";
  import {
    IS_TAURI,
    assignPinboard,
    clearUnpinned,
    createPinboard,
    deleteItem,
    deletePinboard,
    hidePanel,
    listItems,
    listPinboards,
    pasteItem,
    searchItems,
    togglePin,
  } from "./lib/api";

  const PAGE_SIZE = 100;

  let query = $state("");
  let filter = $state<FilterKind>("all");
  let pinboards = $state<PinboardDto[]>([]);
  let activePinboard = $state<number | null>(null);
  let items = $state<ItemDto[]>([]);
  let selectedId = $state<number | null>(null);
  let loading = $state(true);
  let searchInput = $state<HTMLInputElement | null>(null);

  let showSettings = $state(false);
  let confirmingClear = $state(false);
  let contextMenu = $state<{ x: number; y: number; item: ItemDto } | null>(null);

  function focusSearchInput() {
    searchInput?.focus();
    searchInput?.select();
  }

  /** 类型筛选在前端完成：一页数据量有限，避免为筛选单独增加后端接口 */
  function applyFilter(list: ItemDto[], kind: FilterKind): ItemDto[] {
    switch (kind) {
      case "text":
        return list.filter(
          (item) =>
            item.primary_type === "text" ||
            item.primary_type === "html" ||
            item.primary_type === "rtf",
        );
      case "image":
        return list.filter((item) => item.primary_type === "image");
      case "files":
        return list.filter((item) => item.primary_type === "files");
      case "pinned":
        return list.filter((item) => item.is_pinned);
      default:
        return list;
    }
  }

  async function loadPinboards() {
    if (!IS_TAURI) return;
    pinboards = await listPinboards();
    // 当前选中的分组被删除后回到「全部」
    if (activePinboard !== null && !pinboards.some((pb) => pb.id === activePinboard)) {
      activePinboard = null;
    }
  }

  async function loadData(keyword: string, kind: FilterKind, pinboard: number | null) {
    const trimmed = keyword.trim();
    try {
      const raw = trimmed
        ? await searchItems(trimmed, pinboard, 0, PAGE_SIZE)
        : await listItems(pinboard, 0, PAGE_SIZE);
      const next = applyFilter(raw, kind);
      items = next;
      if (!next.some((item) => item.id === selectedId)) {
        selectedId = next[0]?.id ?? null;
      }
    } finally {
      loading = false;
    }
  }

  function reload() {
    void loadData(query, filter, activePinboard);
  }

  // 关键词、筛选或分组变化后重新查询；输入 150ms 防抖
  $effect(() => {
    const keyword = query;
    const kind = filter;
    const pinboard = activePinboard;
    const timer = setTimeout(
      () => {
        void loadData(keyword, kind, pinboard);
      },
      keyword ? 150 : 0,
    );
    return () => clearTimeout(timer);
  });

  // 跟随系统明暗主题；调试时可用 ?theme=dark / ?theme=light 强制指定
  $effect(() => {
    const forced = new URLSearchParams(location.search).get("theme");
    if (forced === "dark" || forced === "light") {
      document.documentElement.dataset.theme = forced;
      return;
    }
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme = media.matches ? "dark" : "light";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  });

  /** 面板重新获得焦点：刷新列表并聚焦搜索框，符合反复呼出的使用节奏 */
  $effect(() => {
    const onFocus = () => {
      // 收起期间在别处复制的内容会插到最前面，上次选中的条目已经被挤到后面；
      // 清掉选中让列表重新从第一张开始，视野也跟着回到最左边（列表自身会保证选中项可见）
      selectedId = null;
      reload();
      focusSearchInput();
    };
    window.addEventListener("focus", onFocus);
    focusSearchInput();
    return () => window.removeEventListener("focus", onFocus);
  });

  // 后端记录到新内容时刷新当前列表
  $effect(() => {
    if (!IS_TAURI) return;
    let disposed = false;
    const unlisten = listen("item-created", () => {
      if (!disposed) reload();
    });
    return () => {
      disposed = true;
      void unlisten.then((off) => off());
    };
  });

  // 启动时加载分组列表
  $effect(() => {
    void loadPinboards();
  });

  function moveSelection(delta: number) {
    if (items.length === 0) return;
    const current = items.findIndex((item) => item.id === selectedId);
    const next = Math.min(
      items.length - 1,
      Math.max(0, (current < 0 ? 0 : current) + delta),
    );
    selectedId = items[next].id;
  }

  /**
   * 用过的条目立刻浮到所属区段的最前面（钉住区仍在最上层）。
   * 正常粘贴时面板马上就收起了，用户看不到过程；但粘贴失败回退、面板留在屏幕上时，
   * 顺序必须当场就是对的，否则会以为点击没生效。
   */
  function moveUsedItemToFront(id: number) {
    const index = items.findIndex((item) => item.id === id);
    if (index < 0) return;
    const used = { ...items[index], last_used_at: Date.now() };
    const rest = items.filter((item) => item.id !== id);
    const insertAt = used.is_pinned ? 0 : rest.findIndex((item) => !item.is_pinned);
    rest.splice(insertAt < 0 ? rest.length : insertAt, 0, used);
    items = rest;
  }

  async function activate(id: number) {
    moveUsedItemToFront(id);
    await pasteItem(id);
  }

  async function handleTogglePin(id: number) {
    const pinned = await togglePin(id);
    items = items.map((item) =>
      item.id === id ? { ...item, is_pinned: pinned } : item,
    );
    if (filter === "pinned" && !pinned) {
      items = items.filter((item) => item.id !== id);
    }
  }

  async function handleDelete(id: number) {
    await deleteItem(id);
    const index = items.findIndex((item) => item.id === id);
    items = items.filter((item) => item.id !== id);
    if (selectedId === id) {
      selectedId = items[Math.min(index, items.length - 1)]?.id ?? null;
    }
  }

  /** 一键清空：只删除未钉住的记录，钉住的内容保留为常用素材 */
  async function handleClearAll() {
    confirmingClear = false;
    try {
      const removed = await clearUnpinned();
      if (removed > 0) {
        selectedId = null;
        reload();
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function handleCreatePinboard(name: string) {
    try {
      await createPinboard(name);
      await loadPinboards();
    } catch (e) {
      console.error(e);
    }
  }

  async function handleDeletePinboard(id: number) {
    await deletePinboard(id);
    await loadPinboards();
    if (activePinboard === id) activePinboard = null;
    reload();
  }

  async function handleAssign(itemId: number, pinboard: number | null) {
    await assignPinboard(itemId, pinboard);
    reload();
  }

  function isTypingInSearch(): boolean {
    return document.activeElement?.tagName === "INPUT";
  }

  function handleKeydown(event: KeyboardEvent) {
    // 确认框自己处理 Esc 与回车，避免误触面板的其它快捷键
    if (confirmingClear) return;

    // 设置面板打开时只响应 Esc
    if (showSettings) {
      if (event.key === "Escape") {
        event.preventDefault();
        showSettings = false;
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      void hidePanel();
      return;
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      moveSelection(1);
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      moveSelection(-1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if (selectedId !== null) void activate(selectedId);
      return;
    }

    if (event.key === "Delete" && selectedId !== null && !isTypingInSearch()) {
      event.preventDefault();
      void handleDelete(selectedId);
      return;
    }

    // 面板内 1-9 快捷粘贴；搜索框聚焦时数字键用于输入
    if (/^[1-9]$/.test(event.key) && !isTypingInSearch()) {
      const target = items[Number(event.key) - 1];
      if (target) {
        event.preventDefault();
        void activate(target.id);
      }
    }
  }

  function buildContextMenu(item: ItemDto) {
    type Entry = { label: string; danger?: boolean; action: () => void };
    const entries: Entry[] = [];

    entries.push({
      label: item.is_pinned ? "取消钉住" : "钉住",
      action: () => void handleTogglePin(item.id),
    });

    if (pinboards.length > 0) {
      if (item.pinboard_id !== null) {
        entries.push({
          label: "移出分组",
          action: () => void handleAssign(item.id, null),
        });
      }
      for (const pb of pinboards) {
        if (pb.id !== item.pinboard_id) {
          entries.push({
            label: `移入「${pb.name}」`,
            action: () => void handleAssign(item.id, pb.id),
          });
        }
      }
    }

    entries.push({ label: "删除", danger: true, action: () => void handleDelete(item.id) });
    return entries;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="panel">
  <TopBar
    bind:value={query}
    bind:inputEl={searchInput}
    {filter}
    {pinboards}
    {activePinboard}
    onfilter={(kind) => (filter = kind)}
    onpinboard={(id) => (activePinboard = id)}
    oncreatepinboard={handleCreatePinboard}
    ondeletepinboard={handleDeletePinboard}
    onsettings={() => (showSettings = true)}
    onclear={() => (confirmingClear = true)}
  />

  <main>
    {#if items.length > 0}
      <HorizontalList
        {items}
        {selectedId}
        onselect={(id) => (selectedId = id)}
        onactivate={activate}
        ontogglePin={handleTogglePin}
        ondelete={handleDelete}
        oncontextmenu={(event, id) => {
          const item = items.find((entry) => entry.id === id);
          if (item) contextMenu = { x: event.clientX, y: event.clientY, item };
        }}
      />
    {:else if !loading}
      <div class="empty">
        <div class="empty-icon">
          <Icon name={query || activePinboard !== null ? "search" : "text"} size={20} />
        </div>
        <p class="empty-title">
          {query
            ? "没有匹配的内容"
            : activePinboard !== null
              ? "该分组还没有条目"
              : "还没有剪贴板记录"}
        </p>
        <p class="empty-hint">
          {query
            ? "换个关键词试试"
            : activePinboard !== null
              ? "右键点击任意卡片，选择「移入分组」"
              : "复制任意内容后会自动出现在这里"}
        </p>
      </div>
    {/if}
  </main>

  {#if contextMenu}
    <ContextMenu
      x={contextMenu.x}
      y={contextMenu.y}
      items={buildContextMenu(contextMenu.item)}
      onclose={() => (contextMenu = null)}
    />
  {/if}

  {#if showSettings}
    <SettingsPanel onclose={() => (showSettings = false)} />
  {/if}

  {#if confirmingClear}
    <ConfirmDialog
      title="清空剪贴板历史？"
      description="将删除全部未钉住的记录，钉住的内容会保留。此操作无法撤销。"
      confirmLabel="清空"
      onconfirm={handleClearAll}
      oncancel={() => (confirmingClear = false)}
    />
  {/if}
</div>

<style>
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    /* 面板贴屏幕底部横向铺开，直角避免与屏幕边缘脱开 */
    background: var(--panel-bg);
    box-shadow: inset 0 0 0 1px var(--panel-border);
    backdrop-filter: blur(30px) saturate(1.6);
    overflow: hidden;
  }

  main {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .empty {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding-bottom: 16px;
  }

  .empty-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin-bottom: 6px;
    border-radius: 11px;
    background: var(--surface-1);
    color: var(--text-tertiary);
  }

  .empty-title {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-secondary);
  }

  .empty-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
