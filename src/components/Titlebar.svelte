<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const win = getCurrentWindow();

  function minimize() {
    void win.minimize();
  }
  function maximize() {
    void win.toggleMaximize();
  }
  function closeWindow() {
    void win.close();
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="titlebar-left" data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>Inkdown</span>
  </div>
  <div class="titlebar-right">
    <button class="win-btn" aria-label="Minimize" onclick={minimize}>
      <svg width="10" height="10" viewBox="0 0 10 10"><line x1="1" y1="5" x2="9" y2="5" stroke="currentColor" stroke-width="1" /></svg>
    </button>
    <button class="win-btn" aria-label="Maximize" onclick={maximize}>
      <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" stroke="currentColor" stroke-width="1" fill="none" /></svg>
    </button>
    <button class="win-btn close" aria-label="Close" onclick={closeWindow}>
      <svg width="10" height="10" viewBox="0 0 10 10"><line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1" /><line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    background: var(--crust);
    border-bottom: 1px solid var(--surface0);
    height: var(--titlebar-h);
    padding-left: 12px;
    user-select: none;
  }
  .titlebar-left {
    display: flex;
    align-items: center;
    color: var(--subtext0);
    font-size: 12.5px;
    letter-spacing: 0.04em;
  }
  .brand {
    font-weight: 600;
    color: var(--lavender);
  }
  .titlebar-right {
    display: flex;
    height: 100%;
  }
  .win-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    height: 100%;
    color: var(--subtext0);
    transition: background 80ms;
  }
  .win-btn:hover {
    background: var(--surface0);
    color: var(--text);
  }
  .win-btn.close:hover {
    background: var(--red);
    color: var(--crust);
  }
</style>
