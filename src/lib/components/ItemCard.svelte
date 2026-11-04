<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { ItemDto } from "../types";
  import { shortRelativeTime } from "../format";
  import { cardVisual } from "../card";
  import { displayAppName } from "../accent";
  import { detectContent } from "../content";

  let {
    item,
    index,
    selected = false,
    onselect,
    onactivate,
    ontogglePin,
    ondelete,
    oncontextmenu,
  }: {
    item: ItemDto;
    index: number;
    selected?: boolean;
    onselect: (id: number) => void;
    onactivate: (id: number) => void;
    ontogglePin: (id: number) => void;
    ondelete: (id: number) => void;
    oncontextmenu: (event: MouseEvent, id: number) => void;
  } = $props();

  const visual = $derived(cardVisual(item));
  const initial = $derived(
    item.source_app ? displayAppName(item.source_app).charAt(0).toUpperCase() : "·",
  );
  /** 文本卡片里再细分：链接上链接色，代码用等宽 */
  const contentKind = $derived(
    item.primary_type === "text" ? detectContent(item.preview) : "text",
  );

  /** 位图或图片文件生成了缩略图时，内容区直接铺图 */
  const showImage = $derived(item.thumbnail_path !== null);

  function handlePin(event: MouseEvent) {
    event.stopPropagation();
    ontogglePin(item.id);
  }

  function handleDelete(event: MouseEvent) {
    event.stopPropagation();
    ondelete(item.id);
  }
</script>

<div class="slot">
  <!-- 键盘操作在面板层统一处理，这里只挂鼠标交互 -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="card"
    class:selected
    class:dark-ink={visual.darkInk}
    role="option"
    aria-selected={selected}
    tabindex="-1"
    style="--band: {visual.band}"
    onclick={() => onactivate(item.id)}
    onmouseenter={() => onselect(item.id)}
    onfocus={() => onselect(item.id)}
    oncontextmenu={(event) => {
      event.preventDefault();
      oncontextmenu(event, item.id);
    }}
  >
    <!-- 顶部色带：类型名 + 时间 + 来源 -->
    <div class="band">
      <span class="band-label">{visual.label}</span>
      <span class="band-time">{shortRelativeTime(item.last_used_at)}</span>
      <span class="band-source" title={visual.subtitle || "未知来源"}>{initial}</span>
    </div>

    <div class="body" class:is-media={showImage || visual.kind === "color"}>
      {#if showImage}
        <img src={item.thumbnail_path} alt="" draggable="false" />
      {:else if visual.kind === "image"}
        <div class="image-empty"><Icon name="image" size={26} /></div>
      {:else if visual.kind === "color"}
        <div class="swatch" style="background: {item.preview.trim()}"></div>
      {:else}
        <p
          class="text"
          class:mono={contentKind === "code" || contentKind === "path"}
          class:link={contentKind === "url" || contentKind === "email"}
        >
          {item.preview}
        </p>
      {/if}
    </div>

    <div class="foot">
      <div class="foot-text">
        <span class="foot-title">{visual.title}</span>
        {#if visual.subtitle}
          <span class="foot-sub">{visual.subtitle}</span>
        {/if}
      </div>

      <div class="actions">
        <button
          class="action"
          class:active={item.is_pinned}
          title={item.is_pinned ? "取消钉住" : "钉住"}
          onclick={handlePin}
        >
          <Icon name={item.is_pinned ? "pin-filled" : "pin"} size={12} />
        </button>
        <button class="action" title="删除" onclick={handleDelete}>
          <Icon name="trash" size={12} />
        </button>
      </div>
    </div>

    {#if index < 9}
      <span class="index">{index + 1}</span>
    {/if}
  </div>
</div>

<style>
  .slot {
    flex: 0 0 var(--card-w, 350px);
    width: var(--card-w, 350px);
    height: 100%;
    padding: 12px 0;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    border-radius: 14px;
    background: var(--card-bg);
    box-shadow: var(--card-shadow);
    overflow: hidden;
    cursor: pointer;
  }

  /* 悬停不加任何视觉变化，仅指针变手型（cursor 在 .card 上）。
     selected 供键盘 ↑↓ 导航使用，只做背景提亮 */
  .card.selected {
    background: var(--card-bg-hover);
  }

  /* ---- 顶部色带 ---- */
  .band {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    flex: 0 0 38px;
    background: var(--band);
    color: #fff;
  }

  .card.dark-ink .band {
    color: rgba(0, 0, 0, 0.78);
  }

  .band-label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }

  .band-time {
    flex: 1 1 auto;
    font-size: 11px;
    opacity: 0.82;
    font-variant-numeric: tabular-nums;
  }

  .band-source {
    display: grid;
    place-items: center;
    width: 21px;
    height: 21px;
    border-radius: 6px;
    font-size: 10.5px;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.26);
    color: inherit;
  }

  .card.dark-ink .band-source {
    background: rgba(0, 0, 0, 0.14);
  }

  /* ---- 内容 ---- */
  .body {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    padding: 13px 12px;
    overflow: hidden;
  }

  .body.is-media {
    padding: 0;
  }

  .text {
    margin: 0;
    font-size: 13px;
    line-height: 1.62;
    color: var(--text-primary);
    word-break: break-word;
  }

  .text.mono {
    font-family: "Cascadia Code", "Consolas", "SF Mono", monospace;
    font-size: 12px;
    line-height: 1.66;
    color: var(--text-secondary);
  }

  .text.link {
    color: var(--link);
  }

  .body.is-media img,
  .swatch {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .image-empty {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--band);
    background: var(--surface-1);
  }

  /* ---- 底部信息 ---- */
  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 46px;
    padding: 0 12px;
  }

  .foot-text {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .foot-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .foot-sub {
    font-size: 11px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 1px;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .card:hover .actions,
  .card.selected .actions {
    opacity: 1;
  }

  .action {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    color: var(--text-tertiary);
  }

  .action:hover {
    background: var(--surface-3);
    color: var(--text-primary);
  }

  .action.active {
    color: var(--accent);
  }

  /* 快速粘贴序号 */
  .index {
    position: absolute;
    right: 10px;
    bottom: 52px;
    display: grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 5px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    background: var(--surface-1);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .card:hover .index {
    opacity: 1;
  }
</style>
