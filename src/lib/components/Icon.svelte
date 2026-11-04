<script lang="ts">
  /**
   * 统一的线性图标，风格贴近 SF Symbols：
   * 1.6 描边、圆角端点、24 网格，全部继承 currentColor。
   */
  let {
    name,
    size = 16,
    strokeWidth = 1.6,
  }: { name: string; size?: number; strokeWidth?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "search"}
    <circle cx="10.5" cy="10.5" r="6.5" />
    <path d="M15.4 15.4L20 20" />
  {:else if name === "pin"}
    <path d="M9.5 3.5h5v5l2 3.2v1.3h-9v-1.3l2-3.2z" />
    <path d="M12 13v7" />
  {:else if name === "pin-filled"}
    <path
      d="M9.5 3.5h5v5l2 3.2v1.3h-9v-1.3l2-3.2z"
      fill="currentColor"
    />
    <path d="M12 13v7" />
  {:else if name === "settings"}
    <path d="M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1" />
    <circle cx="15" cy="7" r="2" />
    <circle cx="9" cy="12" r="2" />
    <circle cx="17" cy="17" r="2" />
  {:else if name === "text"}
    <path d="M5 6.2V5h14v1.2" />
    <path d="M12 5v14M9 19h6" />
  {:else if name === "image"}
    <rect x="3" y="4.5" width="18" height="15" rx="2.6" />
    <circle cx="8.6" cy="10" r="1.5" />
    <path d="M4.2 17.2l4-4a1.6 1.6 0 0 1 2.3 0l4.3 4.3" />
    <path d="M13.6 15.4l1.9-1.9a1.6 1.6 0 0 1 2.3 0l2 2" />
  {:else if name === "file"}
    <path d="M6.5 3.5h6.8l4.2 4.2v12.3a.5.5 0 0 1-.5.5H6.5a.5.5 0 0 1-.5-.5v-16a.5.5 0 0 1 .5-.5z" />
    <path d="M13 3.6v4.6h4.4" />
  {:else if name === "rich"}
    <path d="M8.5 6.5L3.5 12l5 5.5" />
    <path d="M15.5 6.5l5 5.5-5 5.5" />
  {:else if name === "trash"}
    <path d="M4.5 6.8h15" />
    <path d="M9.5 6.8V4.9h5v1.9" />
    <path d="M6.5 6.8l.9 12.4h9.2l.9-12.4" />
    <path d="M10.4 10.6v5.6M13.6 10.6v5.6" />
  {:else if name === "close"}
    <path d="M6.6 6.6l10.8 10.8M17.4 6.6L6.6 17.4" />
  {:else if name === "check"}
    <path d="M5 12.8l4.4 4.4L19 7.6" />
  {:else if name === "chevron-down"}
    <path d="M7 10l5 5 5-5" />
  {:else if name === "arrow-return"}
    <path d="M19 6v6.5a2.5 2.5 0 0 1-2.5 2.5H6" />
    <path d="M9.5 11L5.5 15l4 4" />
  {:else if name === "plus"}
    <path d="M12 5.5v13M5.5 12h13" />
  {/if}
</svg>
