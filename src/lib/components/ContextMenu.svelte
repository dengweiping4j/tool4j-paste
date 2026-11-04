<script lang="ts">
  /**
   * 卡片右键菜单：钉住 / 分组指派 / 删除。
   * 定位用 fixed（clientX/Y），点击菜单外或按 Esc 关闭。
   */
  let {
    x,
    y,
    items,
    onclose,
  }: {
    x: number;
    y: number;
    items: { label: string; danger?: boolean; action: () => void }[];
    onclose: () => void;
  } = $props();

  // 屏幕边缘防溢出
  const left = $derived(Math.min(x, window.innerWidth - 180));
  const top = $derived(Math.min(y, window.innerHeight - items.length * 32 - 16));

  function pick(action: () => void) {
    action();
    onclose();
  }

  $effect(() => {
    const close = () => onclose();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    const timer = setTimeout(() => {
      window.addEventListener("click", close, { once: true });
    }, 0);
    window.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="menu" style="left: {left}px; top: {top}px">
  {#each items as item, i (i)}
    <button
      class="item"
      class:danger={item.danger}
      onclick={() => pick(item.action)}
    >
      {item.label}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 60;
    min-width: 160px;
    padding: 4px;
    border-radius: 10px;
    background: var(--panel-bg);
    backdrop-filter: blur(24px) saturate(1.5);
    box-shadow:
      inset 0 0 0 1px var(--line-strong),
      0 12px 32px rgba(0, 0, 0, 0.35);
  }

  .item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    border-radius: 6px;
    font-size: 12.5px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item:hover {
    background: var(--surface-2);
  }

  .item.danger {
    color: var(--danger);
  }
</style>
