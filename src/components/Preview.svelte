<script lang="ts">
  import { onMount } from "svelte";
  import { renderMarkdown, renderMathIn } from "../lib/markdown";
  import { debounce } from "../lib/debounce";

  type Props = {
    source: string;
    onScroll?: (topLine: number) => void;
  };

  let { source, onScroll }: Props = $props();

  let host: HTMLElement | null = $state(null);
  let scroller: HTMLDivElement | null = $state(null);
  let html = $state("");

  const renderDebounced = debounce(async (src: string) => {
    const result = await renderMarkdown(src);
    html = result;
    queueMicrotask(() => {
      if (host) renderMathIn(host);
    });
  }, 150);

  $effect(() => {
    renderDebounced(source);
  });

  function onScrollEvent() {
    if (!host || !onScroll || !scroller) return;
    const top = scroller.getBoundingClientRect().top;
    const candidates = host.querySelectorAll<HTMLElement>("[data-line]");
    let bestLine = 0;
    let bestDiff = Infinity;
    for (const el of candidates) {
      const diff = el.getBoundingClientRect().top - top;
      if (diff >= -10 && diff < bestDiff) {
        bestDiff = diff;
        bestLine = parseInt(el.dataset.line ?? "0", 10);
      }
    }
    onScroll(bestLine);
  }

  export function scrollToLine(line: number) {
    if (!host || !scroller) return;
    const candidates = host.querySelectorAll<HTMLElement>("[data-line]");
    let target: HTMLElement | null = null;
    for (const el of candidates) {
      const l = parseInt(el.dataset.line ?? "0", 10);
      if (l <= line) target = el;
      else break;
    }
    if (target) {
      const top = scroller.scrollTop + target.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
      scroller.scrollTo({ top, behavior: "auto" });
    }
  }

  onMount(() => {
    if (host) renderMathIn(host);
  });
</script>

<div class="pane preview-pane" bind:this={scroller} onscroll={onScrollEvent}>
  <article class="preview" bind:this={host}>{@html html}</article>
</div>
