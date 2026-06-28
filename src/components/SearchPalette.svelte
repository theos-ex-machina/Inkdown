<script lang="ts">
  import { cmd, type SearchHit } from "../lib/tauri";
  import { debounce } from "../lib/debounce";

  type Props = {
    open: boolean;
    onClose: () => void;
    onSelect: (path: string) => void;
  };

  let { open, onClose, onSelect }: Props = $props();

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let selectedIndex = $state(0);
  let inputRef: HTMLInputElement;

  const runSearch = debounce(async (q: string) => {
    if (!q.trim()) {
      hits = [];
      selectedIndex = 0;
      return;
    }
    hits = (await cmd.search(q.trim(), 50)) ?? [];
    selectedIndex = 0;
  }, 150);

  $effect(() => {
    if (open) {
      query = "";
      hits = [];
      selectedIndex = 0;
      queueMicrotask(() => inputRef?.focus());
    }
  });

  $effect(() => {
    if (!open) return;
    runSearch(query);
  });

  function onInput(e: Event) {
    query = (e.target as HTMLInputElement).value;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, hits.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === "Enter" && hits[selectedIndex]) {
      onSelect(hits[selectedIndex].path);
      onClose();
    }
  }

  function selectHit(hit: SearchHit) {
    onSelect(hit.path);
    onClose();
  }
</script>

{#if open}
  <div class="palette-overlay" onclick={onClose}>
    <div class="palette" onkeydown={onKeyDown} onclick={(e) => e.stopPropagation()}>
      <div class="palette-header">
        <kbd class="kbd">⌘K</kbd>
        <span class="palette-title">Search</span>
      </div>
      <input
        bind:this={inputRef}
        class="palette-input"
        type="search"
        value={query}
        oninput={onInput}
        placeholder="Search pages…"
        aria-label="Search pages"
        autocomplete="off"
      />
      {#if hits.length > 0}
        <div class="palette-results" role="listbox">
          {#each hits as hit, i (hit.path)}
            <button
              class="palette-item"
              class:selected={i === selectedIndex}
              role="option"
              aria-selected={i === selectedIndex}
              onclick={() => selectHit(hit)}
              onmouseover={() => (selectedIndex = i)}
            >
              <div class="item-main">
                <span class="item-title">{hit.title}</span>
                <span class="item-path">{hit.path}</span>
              </div>
              {#if hit.snippet}
                <div class="item-snippet">{@html hit.snippet}</div>
              {/if}
            </button>
          {/each}
        </div>
      {:else if query.trim()}
        <div class="palette-empty">No matches</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .palette-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
    z-index: 1000;
    animation: fadeIn 120ms ease;
  }
  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .palette {
    background: var(--mantle);
    border: 1px solid var(--surface1);
    border-radius: 10px;
    width: min(720px, 90vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    animation: slideDown 120ms ease;
    overflow: hidden;
  }
  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .palette-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--surface1);
    color: var(--overlay1);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .kbd {
    background: var(--surface1);
    border: 1px solid var(--surface2);
    border-radius: 4px;
    padding: 2px 6px;
    font-family: var(--mono);
    font-size: 10px;
  }
  .palette-input {
    width: 100%;
    background: var(--crust);
    border: none;
    color: var(--text);
    padding: 12px 14px;
    font: inherit;
    font-size: 15px;
    outline: none;
  }
  .palette-input::placeholder {
    color: var(--overlay1);
  }
  .palette-results {
    flex: 1;
    overflow-y: auto;
    max-height: 55vh;
  }
  .palette-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 10px 14px;
    background: transparent;
    border: none;
    color: var(--subtext1);
    cursor: pointer;
  }
  .palette-item:hover,
  .palette-item.selected {
    background: var(--surface0);
    color: var(--text);
  }
  .item-main {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .item-title {
    font-weight: 500;
    font-size: 14px;
    color: var(--text);
  }
  .item-path {
    font: 11px var(--mono);
    color: var(--overlay1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .item-snippet {
    margin-top: 4px;
    font-size: 12px;
    color: var(--subtext0);
    line-height: 1.4;
  }
  .item-snippet :global(b) {
    color: var(--peach);
    font-weight: 600;
  }
  .palette-empty {
    padding: 20px 14px;
    color: var(--overlay1);
    font-size: 13px;
    text-align: center;
  }
</style>