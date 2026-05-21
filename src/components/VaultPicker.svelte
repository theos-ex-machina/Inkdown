<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { vault } from "../lib/config.svelte";
  import { pushToast } from "../lib/toast.svelte";

  let busy = $state(false);

  async function pick() {
    if (busy) return;
    busy = true;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose a vault directory",
      });
      if (typeof selected === "string" && selected) {
        await vault.set(selected);
      }
    } catch (err) {
      pushToast(err instanceof Error ? err.message : String(err), "error");
    } finally {
      busy = false;
    }
  }
</script>

<div class="vault-picker">
  <div class="card">
    <h1>Inkdown</h1>
    <p class="tagline">One vault. Flat markdown files. Ink on top.</p>
    <p class="hint">Pick a folder on disk to use as your vault. Every <code>.md</code> file inside becomes a page.</p>
    <button class="primary" disabled={busy} onclick={pick}>
      {busy ? "Opening…" : "Choose vault folder"}
    </button>
  </div>
</div>

<style>
  .vault-picker {
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--base);
  }
  .card {
    background: var(--mantle);
    border: 1px solid var(--surface0);
    border-radius: 10px;
    padding: 40px 48px;
    max-width: 420px;
    text-align: center;
  }
  h1 {
    color: var(--lavender);
    margin: 0 0 8px;
    font-size: 28px;
    letter-spacing: -0.01em;
  }
  .tagline {
    color: var(--subtext1);
    margin: 0 0 28px;
    font-size: 13px;
  }
  .hint {
    color: var(--subtext0);
    font-size: 13px;
    margin: 0 0 24px;
  }
  code {
    background: var(--surface0);
    color: var(--peach);
    padding: 1px 5px;
    border-radius: 4px;
    font-family: var(--mono);
    font-size: 0.92em;
  }
  button.primary {
    background: var(--lavender);
    color: var(--crust);
    padding: 10px 22px;
    border-radius: 6px;
    font-weight: 600;
    transition: filter 120ms;
  }
  button.primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  button.primary:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
