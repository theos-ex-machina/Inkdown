<script lang="ts">
  type Props = {
    mode: "type" | "ink";
    onToggle: () => void;
  };
  let { mode, onToggle }: Props = $props();
</script>

<button
  class="mode-toggle"
  class:ink={mode === "ink"}
  onclick={onToggle}
  title={mode === "type" ? "Switch to ink mode (Ctrl+D)" : "Switch to type mode (Ctrl+D)"}
  aria-label="Toggle mode"
>
  <span class="icon" aria-hidden="true">
    {#if mode === "type"}
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
        <path d="M2 4h10M4 4v8M6 12h-2M10 12h-2" stroke-linecap="round" />
      </svg>
    {:else}
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
        <path d="M2 12l3-1 6.5-6.5a1.5 1.5 0 00-2.1-2.1L3 8.9 2 12z" stroke-linejoin="round" />
      </svg>
    {/if}
  </span>
  <span class="label">{mode === "type" ? "Type" : "Ink"}</span>
</button>

<style>
  .mode-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--surface0);
    color: var(--text);
    border: 1px solid var(--surface1);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12.5px;
    transition: background 80ms;
  }
  .mode-toggle:hover {
    background: var(--surface1);
  }
  .mode-toggle.ink {
    background: var(--lavender);
    color: var(--crust);
    border-color: var(--lavender);
  }
  .icon {
    display: inline-flex;
    align-items: center;
  }
</style>
