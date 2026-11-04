<script lang="ts">
  import Icon from "./Icon.svelte";
  import { getSettings, updateSettings, openAboutPage } from "../api";
  import { DEFAULT_SETTINGS, type SettingsDto } from "../types";

  let { onclose }: { onclose: () => void } = $props();

  /** 容量是数量级选择而非精确值，提供档位而不是自由输入 */
  const CAPACITY_OPTIONS = [50, 100, 500, 1000, 5000, 10000];

  let settings = $state<SettingsDto>({ ...DEFAULT_SETTINGS });
  let recording = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let savedTip = $state(false);

  $effect(() => {
    void getSettings().then((value) => (settings = value));
  });

  async function save() {
    saving = true;
    error = null;
    try {
      await updateSettings(settings);
      savedTip = true;
      setTimeout(() => (savedTip = false), 1800);
    } catch (e) {
      error = String(e);
      // 设置保存失败时回读当前生效值（例如快捷键被占用，后端未采纳）
      settings = await getSettings();
    } finally {
      saving = false;
    }
  }

  // 录制快捷键：捕获模式拦下所有按键，Esc 取消
  $effect(() => {
    if (!recording) return;
    const handler = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        recording = false;
        return;
      }

      const parts: string[] = [];
      if (event.ctrlKey) parts.push("Ctrl");
      if (event.altKey) parts.push("Alt");
      if (event.shiftKey) parts.push("Shift");
      if (event.metaKey) parts.push("Win");

      if (event.code.startsWith("Key")) {
        parts.push(event.code.slice(3));
      } else if (event.code.startsWith("Digit")) {
        parts.push(event.code.slice(5));
      } else if (/^F\d{1,2}$/.test(event.code)) {
        parts.push(event.code);
      } else {
        return; // 只按了修饰键，等待主键
      }

      if (parts.length < 2) return; // 必须带修饰键
      settings = { ...settings, hotkey: parts.join("+") };
      recording = false;
      void save();
    };
    window.addEventListener("keydown", handler, { capture: true });
    return () => window.removeEventListener("keydown", handler, { capture: true });
  });
</script>

<div class="layer">
  <header>
    <span class="title">设置</span>
    <button class="close" title="关闭" onclick={onclose}>
      <Icon name="close" size={14} />
    </button>
  </header>

  <div class="content">
    <div class="row">
      <div class="text">
        <div class="row-title">全局快捷键</div>
        <div class="row-desc">呼出或收起剪贴板面板，支持 Ctrl / Alt / Shift / Win 组合</div>
      </div>
      <button
        class="hotkey"
        class:recording
        onclick={() => (recording = true)}
      >
        {recording ? "按下组合键…" : settings.hotkey}
      </button>
    </div>

    <div class="row">
      <div class="text">
        <div class="row-title">自动粘贴</div>
        <div class="row-desc">选择条目后直接粘贴到之前的窗口，关闭则仅写入剪贴板</div>
      </div>
      <button
        class="toggle"
        class:on={settings.auto_paste}
        aria-label="自动粘贴开关"
        onclick={() => {
          settings = { ...settings, auto_paste: !settings.auto_paste };
          void save();
        }}
      >
        <span class="knob"></span>
      </button>
    </div>

    <div class="row">
      <div class="text">
        <div class="row-title">开机自启</div>
        <div class="row-desc">登录 Windows 后在后台自动运行</div>
      </div>
      <button
        class="toggle"
        class:on={settings.launch_at_login}
        aria-label="开机自启开关"
        onclick={() => {
          settings = { ...settings, launch_at_login: !settings.launch_at_login };
          void save();
        }}
      >
        <span class="knob"></span>
      </button>
    </div>

    <div class="row">
      <div class="text">
        <div class="row-title">历史容量</div>
        <div class="row-desc">超过上限时自动清理最旧的未钉住记录，钉住的不受影响</div>
      </div>
      <div class="capacity">
        {#each CAPACITY_OPTIONS as option (option)}
          <button
            class="cap"
            class:on={settings.max_unpinned_items === option}
            onclick={() => {
              if (settings.max_unpinned_items === option) return;
              settings = { ...settings, max_unpinned_items: option };
              void save();
            }}
          >
            {option >= 1000 ? `${option / 1000}k` : option}
          </button>
        {/each}
      </div>
    </div>

    <div class="about">
      <div class="about-line">
        <span class="about-name">Tool4J Paste</span>
        <span class="about-ver">v0.1.0</span>
      </div>
      <div class="about-desc">轻量级 Windows 剪贴板历史管理器，数据全部保存在本机。</div>
      <button class="about-link" onclick={() => void openAboutPage()}>
        访问 tool4j.com
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4">
          <path d="M2 8L8 2M8 2H3.5M8 2v4.5" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    </div>
  </div>

  {#if error}
    <div class="notice error">{error}</div>
  {:else if savedTip}
    <div class="notice saved">已保存</div>
  {/if}
</div>

<style>
  .layer {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--panel-bg);
    backdrop-filter: blur(30px) saturate(1.6);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    flex: 0 0 56px;
    padding: 0 16px 0 20px;
    border-bottom: 1px solid var(--line);
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .close {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--text-secondary);
  }

  .close:hover {
    background: var(--surface-2);
    color: var(--text-primary);
  }

  .content {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: 8px 20px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 0;
    border-bottom: 1px solid var(--line);
  }

  .row:last-child {
    border-bottom: none;
  }

  .row-title {
    font-size: 13px;
    color: var(--text-primary);
  }

  .row-desc {
    margin-top: 2px;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .hotkey {
    min-width: 110px;
    height: 30px;
    padding: 0 12px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
    transition: box-shadow var(--duration-fast) var(--ease-out);
  }

  .hotkey:hover {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }

  .hotkey.recording {
    color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--accent-line);
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.55;
    }
  }

  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    flex: 0 0 38px;
    border-radius: 11px;
    background: var(--surface-3);
    transition: background var(--duration-base) var(--ease-out);
  }

  .toggle.on {
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform var(--duration-base) var(--ease-out);
  }

  .toggle.on .knob {
    transform: translateX(16px);
  }

  .capacity {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .cap {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 11.5px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    transition:
      background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .cap:hover {
    color: var(--text-primary);
  }

  .cap.on {
    background: var(--accent);
    color: #fff;
  }

  .about {
    margin-top: 18px;
    padding: 14px;
    border-radius: 12px;
    background: var(--surface-1);
  }

  .about-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .about-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .about-ver {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .about-desc {
    margin: 4px 0 10px;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .about-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 12px;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    transition: filter var(--duration-fast) var(--ease-out);
  }

  .about-link:hover {
    filter: brightness(1.12);
  }

  .notice {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    padding: 7px 14px;
    border-radius: 8px;
    font-size: 11.5px;
    background: rgba(20, 20, 24, 0.92);
    color: rgba(255, 255, 255, 0.92);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
  }

  .notice.error {
    color: #ffb4b4;
  }
</style>
