<script lang="ts">
  import ItemCard from "./ItemCard.svelte";
  import type { ItemDto } from "../types";

  /**
   * 横向卡片流（Paste 式布局）。
   * 固定卡片宽度 + 固定间距，只渲染可视区 + 两侧缓冲，DOM 节点数恒定。
   */
  const CARD_WIDTH = 330;
  const GAP = 14;
  const STEP = CARD_WIDTH + GAP;
  const OVERSCAN = 2;

  let {
    items,
    selectedId,
    onselect,
    onactivate,
    ontogglePin,
    ondelete,
    oncontextmenu,
  }: {
    items: ItemDto[];
    selectedId: number | null;
    onselect: (id: number) => void;
    onactivate: (id: number) => void;
    ontogglePin: (id: number) => void;
    ondelete: (id: number) => void;
    oncontextmenu: (event: MouseEvent, id: number) => void;
  } = $props();

  let viewportEl = $state<HTMLDivElement | null>(null);
  let viewportWidth = $state(800);
  let scrollLeft = $state(0);
  let ticking = false;

  const startIndex = $derived(
    Math.max(0, Math.floor(scrollLeft / STEP) - OVERSCAN),
  );
  const endIndex = $derived(
    Math.min(
      items.length,
      Math.ceil((scrollLeft + viewportWidth) / STEP) + OVERSCAN,
    ),
  );
  const visible = $derived(items.slice(startIndex, endIndex));

  function handleScroll() {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      scrollLeft = viewportEl?.scrollLeft ?? 0;
      ticking = false;
    });
  }

  /** 鼠标滚轮的纵向滚动转成横向，横向卡片流要靠这个才顺手 */
  function handleWheel(event: WheelEvent) {
    const el = viewportEl;
    if (!el) return;
    if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
    el.scrollLeft += event.deltaY;
    event.preventDefault();
  }

  function ensureVisible(index: number) {
    const el = viewportEl;
    if (!el) return;
    const left = index * STEP;
    const right = left + CARD_WIDTH;
    if (left < el.scrollLeft) {
      el.scrollLeft = left;
    } else if (right > el.scrollLeft + el.clientWidth) {
      el.scrollLeft = right - el.clientWidth;
    }
    // 程序化滚动后立刻同步位置，不必等滚动事件在下一帧到达，
    // 否则虚拟列表会先用旧区间渲染一帧，出现空白
    scrollLeft = el.scrollLeft;
  }

  // 键盘左右切换时保持选中卡片可见
  $effect(() => {
    const index = items.findIndex((item) => item.id === selectedId);
    if (index >= 0) ensureVisible(index);
  });

  $effect(() => {
    const el = viewportEl;
    if (!el) return;
    viewportWidth = el.clientWidth;
    const observer = new ResizeObserver(() => {
      viewportWidth = el.clientWidth;
    });
    observer.observe(el);
    return () => observer.disconnect();
  });
</script>

<div
  class="viewport"
  bind:this={viewportEl}
  onscroll={handleScroll}
  onwheel={handleWheel}
>
  <div class="spacer" style="width: {items.length * STEP}px">
    <div
      class="track"
      style="--card-w: {CARD_WIDTH}px; gap: {GAP}px; transform: translateX({startIndex * STEP}px)"
    >
      {#each visible as item, i (item.id)}
        <ItemCard
          {item}
          index={startIndex + i}
          selected={item.id === selectedId}
          {onselect}
          {onactivate}
          {ontogglePin}
          {ondelete}
          {oncontextmenu}
        />
      {/each}
    </div>
  </div>
</div>

<style>
  .viewport {
    flex: 1 1 auto;
    min-height: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* 左侧只留 8px 呼吸位，避免卡片死贴面板边缘；右侧渐隐提示还有内容 */
    padding: 0 0 0 8px;
    contain: strict;
    mask-image: linear-gradient(
      90deg,
      #000 0,
      #000 calc(100% - 24px),
      transparent 100%
    );
  }

  .spacer {
    position: relative;
    height: 100%;
  }

  .track {
    position: absolute;
    top: 0;
    left: 0;
    display: flex;
    align-items: stretch;
    height: 100%;
    will-change: transform;
  }
</style>
