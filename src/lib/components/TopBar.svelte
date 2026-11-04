<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { FilterKind, PinboardDto } from "../types";

  let {
    value = $bindable(""),
    filter,
    inputEl = $bindable(null),
    pinboards = [],
    activePinboard = null,
    onfilter,
    onpinboard,
    oncreatepinboard,
    ondeletepinboard,
    onsettings,
    onclear,
  }: {
    value?: string;
    filter: FilterKind;
    inputEl?: HTMLInputElement | null;
    pinboards?: PinboardDto[];
    activePinboard?: number | null;
    onfilter: (kind: FilterKind) => void;
    onpinboard: (id: number | null) => void;
    oncreatepinboard: (name: string) => void;
    ondeletepinboard: (id: number) => void;
    onsettings?: () => void;
    onclear?: () => void;
  } = $props();

  /** 顶部标签页：类型筛选 + Pinboard 分组 */
  const tabs: { kind: FilterKind; label: string; dot: string }[] = [
    { kind: "all", label: "全部", dot: "#8E8E93" },
    { kind: "text", label: "文本", dot: "#3B82F6" },
    { kind: "image", label: "图片", dot: "#F59E0B" },
    { kind: "files", label: "文件", dot: "#64748B" },
    { kind: "pinned", label: "钉住", dot: "#EC4899" },
  ];

  let creating = $state(false);
  let newName = $state("");
  let groupInput = $state<HTMLInputElement | null>(null);

  function submitNew() {
    const name = newName.trim();
    if (name) oncreatepinboard(name);
    newName = "";
    creating = false;
  }

  $effect(() => {
    if (creating) setTimeout(() => groupInput?.focus(), 0);
  });

  function clear() {
    value = "";
    inputEl?.focus();
  }
</script>

<header data-tauri-drag-region>
  <div class="search">
    <span class="search-icon"><Icon name="search" size={14} /></span>
    <input
      bind:this={inputEl}
      bind:value
      placeholder="搜索剪贴板"
      spellcheck="false"
      autocomplete="off"
    />
    {#if value}
      <button class="clear" title="清除" onclick={clear}>
        <Icon name="close" size={11} />
      </button>
    {/if}
  </div>

  <nav data-tauri-drag-region>
    {#each tabs as tab (tab.kind)}
      <button
        class="tab"
        class:active={filter === tab.kind && activePinboard === null}
        onclick={() => {
          onpinboard(null);
          onfilter(tab.kind);
        }}
      >
        <i class="dot" style="background: {tab.dot}"></i>
        {tab.label}
      </button>
    {/each}

    <span class="divider"></span>

    {#each pinboards as pb (pb.id)}
      <button
        class="tab"
        class:active={activePinboard === pb.id}
        title="分组 · {pb.name}"
        onclick={() => onpinboard(activePinboard === pb.id ? null : pb.id)}
      >
        <i class="dot" style="background: {pb.color ?? '#8E8E93'}"></i>
        {pb.name}
        <span
          class="pb-del"
          title="删除分组（条目会保留）"
          role="button"
          tabindex="-1"
          aria-label="删除分组 {pb.name}"
          onclick={(event) => {
            event.stopPropagation();
            ondeletepinboard(pb.id);
          }}
          onkeydown={(event) => event.key === "Enter" && ondeletepinboard(pb.id)}
        >
          <Icon name="close" size={9} />
        </span>
      </button>
    {/each}

    {#if creating}
      <input
        bind:this={groupInput}
        bind:value={newName}
        class="pb-input"
        placeholder="分组名，回车确认"
        maxlength="20"
        onkeydown={(event) => {
          if (event.key === "Enter") submitNew();
          if (event.key === "Escape") {
            creating = false;
            newName = "";
          }
        }}
        onblur={() => (creating = false)}
      />
    {:else}
      <button class="add" title="新建分组" onclick={() => (creating = true)}>
        <Icon name="plus" size={13} />
      </button>
    {/if}
  </nav>

  <div class="tools">
    {#if onclear}
      <button class="tool" title="清空历史（钉住的内容会保留）" onclick={onclear}>
        <Icon name="trash" size={15} />
      </button>
    {/if}

    {#if onsettings}
      <button class="tool" title="设置" onclick={onsettings}>
        <Icon name="settings" size={15} />
      </button>
    {/if}
  </div>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    flex: 0 0 56px;
    padding: 0 16px 0 8px;
    border-bottom: 1px solid var(--line);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 220px;
    height: 30px;
    padding: 0 8px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--duration-fast) var(--ease-out);
    flex: 0 0 220px;
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 1px var(--accent-line);
    background: var(--surface-2);
  }

  .search-icon {
    display: grid;
    place-items: center;
    color: var(--text-tertiary);
  }

  input {
    flex: 1 1 auto;
    min-width: 0;
    height: 100%;
    font-size: 12.5px;
    color: var(--text-primary);
    user-select: text;
    cursor: text;
  }

  input::placeholder {
    color: var(--text-tertiary);
  }

  .clear {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    color: var(--text-tertiary);
  }

  .clear:hover {
    background: var(--surface-3);
    color: var(--text-primary);
  }

  nav {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--text-secondary);
    white-space: nowrap;
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .tab:hover {
    background: var(--surface-1);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--surface-3);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: 0 0 6px;
  }

  .pb-del {
    display: none;
    place-items: center;
    width: 14px;
    height: 14px;
    margin-right: -3px;
    border-radius: 4px;
    color: var(--text-tertiary);
  }

  .pb-del:hover {
    background: var(--surface-3);
    color: var(--danger);
  }

  .tab:hover .pb-del {
    display: grid;
  }

  .divider {
    width: 1px;
    height: 16px;
    margin: 0 6px;
    background: var(--line);
    flex: 0 0 1px;
  }

  .pb-input {
    width: 130px;
    height: 28px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--accent-line);
    font-size: 12px;
    color: var(--text-primary);
    user-select: text;
  }

  .pb-input::placeholder {
    color: var(--text-tertiary);
  }

  .add {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--text-tertiary);
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .add:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: 0 0 auto;
  }

  .tool {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex: 0 0 28px;
    border-radius: 8px;
    color: var(--text-secondary);
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .tool:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }
</style>
