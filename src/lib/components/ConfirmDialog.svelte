<script lang="ts">
  /**
   * 通用确认框：用于「清空」这类不可撤销的操作。
   * 默认焦点放在「取消」上，避免误按回车造成破坏。
   */
  import { onMount } from "svelte";

  let {
    title,
    description,
    confirmLabel = "确认",
    cancelLabel = "取消",
    danger = true,
    onconfirm,
    oncancel,
  }: {
    title: string;
    description: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let cancelButton = $state<HTMLButtonElement | null>(null);

  onMount(() => cancelButton?.focus());

  // 捕获阶段抢先处理按键：Esc 取消、回车确认，且不再冒泡给面板的快捷键
  $effect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        oncancel();
      } else if (event.key === "Enter") {
        event.preventDefault();
        event.stopPropagation();
        onconfirm();
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  });
</script>

<div class="layer" role="presentation" onclick={oncancel}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    onclick={(event) => event.stopPropagation()}
  >
    <div class="title">{title}</div>
    <p class="desc">{description}</p>
    <div class="actions">
      <button class="btn" bind:this={cancelButton} onclick={oncancel}>
        {cancelLabel}
      </button>
      <button class="btn primary" class:danger onclick={onconfirm}>
        {confirmLabel}
      </button>
    </div>
  </div>
</div>

<style>
  .layer {
    position: absolute;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.34);
    animation: fade var(--duration-base) var(--ease-out);
  }

  .dialog {
    width: 296px;
    padding: 16px 16px 12px;
    border-radius: 14px;
    background: var(--panel-bg);
    backdrop-filter: blur(30px) saturate(1.6);
    box-shadow:
      inset 0 0 0 1px var(--line-strong),
      0 18px 44px rgba(0, 0, 0, 0.45);
    animation: rise var(--duration-base) var(--ease-out);
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .desc {
    margin: 6px 0 16px;
    font-size: 11.5px;
    line-height: 1.55;
    color: var(--text-secondary);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    height: 28px;
    padding: 0 14px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--text-secondary);
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--line);
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      filter var(--duration-fast) var(--ease-out);
  }

  .btn:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }

  .btn.primary {
    font-weight: 500;
    color: #fff;
    background: var(--accent);
    box-shadow: none;
  }

  .btn.primary.danger {
    background: var(--danger);
  }

  .btn.primary:hover {
    filter: brightness(1.1);
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(4px);
    }
  }
</style>
