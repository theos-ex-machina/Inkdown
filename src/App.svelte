<script lang="ts">
  import { onMount } from "svelte";
  import Titlebar from "./components/Titlebar.svelte";
  import VaultPicker from "./components/VaultPicker.svelte";
  import PageTree from "./components/PageTree.svelte";
  import Editor from "./components/Editor.svelte";
  import InkCanvas from "./components/InkCanvas.svelte";
  import ModeToggle from "./components/ModeToggle.svelte";
  import Toast from "./components/Toast.svelte";
  import { vault } from "./lib/config.svelte";
  import { registerShortcut } from "./lib/shortcuts";
  import { INK_COLORS, type InkWidthName } from "./lib/ink";
  import { onStylus } from "./lib/stylus";

  let activePath = $state<string | null>(null);
  let dirty = $state(false);
  let treeRefreshKey = $state(0);

  let sidebarCollapsed = $state(false);
  let mode = $state<"type" | "ink">("type");

  let inkColor = $state<string>(INK_COLORS[0]);
  let inkWidth = $state<InkWidthName>("medium");

  let editorRef: Editor | null = $state(null);

  function setMode(next: "type" | "ink") {
    if (mode === next) return;
    mode = next;
    editorRef?.setEditable(mode === "type");
  }

  function toggleMode() {
    setMode(mode === "type" ? "ink" : "type");
  }

  /**
   * Called by InkCanvas when a non-pen pointer (mouse / touch) hits the ink
   * overlay. We jump back to type mode and place the text cursor at the
   * same coordinates so the user gets the single-click behavior they'd
   * expect, without the overlay flickering mid-stroke (which is what broke
   * inking before — palm-rejection touches would toggle the overlay
   * transparent and turn every stroke into a straight line).
   */
  function onInkCancelledByPointer(clientX: number, clientY: number) {
    setMode("type");
    queueMicrotask(() => editorRef?.positionAtCoords(clientX, clientY));
  }

  /**
   * Auto-switch to ink mode when a pen approaches the screen.
   *
   * Windows/macOS: the WebView emits `pointerover`/`pointerdown` with
   * `pointerType === 'pen'` (Surface, Wacom etc. report hover).
   *
   * Linux: WebKitGTK reports the stylus as a mouse, so the Rust backend
   * watches the GTK layer instead and emits native `stylus` events; the
   * `proximity` phase below is the Linux equivalent of pen hover.
   */
  function onWindowPointerOver(e: PointerEvent) {
    if (e.pointerType === "pen" && mode !== "ink") setMode("ink");
  }

  function onWindowPointerDown(e: PointerEvent) {
    if (e.pointerType === "pen" && mode !== "ink") setMode("ink");
  }

  /**
   * Any printable key falls back to type mode, in case the user managed to
   * enter ink without a pen.
   */
  function onWindowKeyDown(e: KeyboardEvent) {
    if (mode !== "ink") return;
    if (e.key === "Control" || e.key === "Shift" || e.key === "Alt" || e.key === "Meta") return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    setMode("type");
  }

  function selectPage(path: string) {
    if (!path) {
      activePath = null;
      return;
    }
    activePath = path;
  }

  function breadcrumbs(): string[] {
    if (!activePath) return [];
    return activePath.split("/");
  }

  onMount(() => {
    const unlistenStylus = onStylus((e) => {
      if (e.phase === "proximity" && mode !== "ink") setMode("ink");
    });

    void vault.init();
    registerShortcut("Mod+\\", () => {
      sidebarCollapsed = !sidebarCollapsed;
    });
    registerShortcut("Mod+D", () => {
      toggleMode();
    });
    registerShortcut("Mod+S", () => {
      // No-op: edits already auto-save after 500ms. Surface a hint.
    });

    return () => {
      void unlistenStylus.then((unlisten) => unlisten());
    };
  });
</script>

<svelte:window
  onpointerover={onWindowPointerOver}
  onpointerdown={onWindowPointerDown}
  onkeydown={onWindowKeyDown}
/>

<div class="app-shell">
  <Titlebar />
  {#if vault.loading}
    <div class="loading">Loading…</div>
  {:else if !vault.path}
    <VaultPicker />
  {:else}
    <div class="app-body" class:sidebar-collapsed={sidebarCollapsed}>
      {#if !sidebarCollapsed}
        <PageTree
          activePath={activePath}
          onSelect={selectPage}
          refreshKey={treeRefreshKey}
        />
      {/if}
      <main class="content-area">
        <div class="content-toolbar">
          <nav class="breadcrumb">
            <span class="sep">📁</span>
            {#each breadcrumbs() as crumb, i (i)}
              {#if i > 0}<span class="sep">/</span>{/if}
              <span class:current={i === breadcrumbs().length - 1}>{crumb}</span>
            {/each}
            {#if dirty}
              <span class="dirty-indicator" title="Unsaved changes">●</span>
            {/if}
          </nav>
          <div class="toolbar-right">
            {#if mode === "ink"}
              <div class="ink-controls">
                {#each INK_COLORS as c (c)}
                  <button
                    class="swatch"
                    class:selected={inkColor === c}
                    style="--swatch:{c}"
                    onclick={() => (inkColor = c)}
                    aria-label="Color {c}"
                  ></button>
                {/each}
                <div class="width-group">
                  {#each ['thin', 'medium', 'thick'] as w (w)}
                    <button
                      class="width-btn"
                      class:selected={inkWidth === w}
                      aria-label="Stroke {w}"
                      onclick={() => (inkWidth = w as InkWidthName)}
                    >
                      <span class="dot {w}"></span>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
            <ModeToggle mode={mode} onToggle={toggleMode} />
          </div>
        </div>
        {#if !activePath}
          <div class="empty">Select or create a page to get started.</div>
        {:else}
          <div class="single-pane">
            <div class="canvas-stack">
              <Editor
                bind:this={editorRef}
                virtualPath={activePath}
                onChange={() => {}}
                onDirty={(d) => (dirty = d)}
              />
              <InkCanvas
                virtualPath={activePath}
                active={mode === "ink"}
                color={inkColor}
                width={inkWidth}
                onCancel={onInkCancelledByPointer}
                onNativePenDown={() => setMode("ink")}
              />
            </div>
          </div>
        {/if}
      </main>
    </div>
  {/if}
  <Toast />
</div>

<style>
  .loading {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--subtext0);
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--subtext0);
    font-size: 14px;
  }
  .dirty-indicator {
    color: var(--peach);
    margin-left: 6px;
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .ink-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 8px;
    border-right: 1px solid var(--surface0);
  }
  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--swatch);
    border: 2px solid transparent;
    transition: transform 80ms;
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch.selected {
    border-color: var(--text);
    transform: scale(1.1);
  }
  .width-group {
    display: flex;
    gap: 2px;
    margin-left: 4px;
  }
  .width-btn {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background 80ms;
  }
  .width-btn:hover {
    background: var(--surface0);
  }
  .width-btn.selected {
    background: var(--surface0);
  }
  .dot {
    background: var(--text);
    border-radius: 50%;
    display: inline-block;
  }
  .dot.thin {
    width: 4px;
    height: 4px;
  }
  .dot.medium {
    width: 7px;
    height: 7px;
  }
  .dot.thick {
    width: 11px;
    height: 11px;
  }
  .single-pane {
    flex: 1;
    overflow: hidden;
    position: relative;
  }
  .canvas-stack {
    position: relative;
    height: 100%;
    min-height: 100%;
  }
</style>
