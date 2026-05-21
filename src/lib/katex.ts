import katex from "katex";

/**
 * Walk the container, find any `.math-inline` / `.math-block` spans
 * emitted by the backend renderer, and replace their text content with
 * KaTeX-rendered HTML.
 */
export function renderMathIn(container: HTMLElement): void {
  const inline = container.querySelectorAll<HTMLElement>("span.math-inline");
  for (const el of inline) {
    if (el.dataset.rendered === "1") continue;
    const src = el.textContent ?? "";
    try {
      el.innerHTML = katex.renderToString(src, {
        throwOnError: false,
        displayMode: false,
      });
    } catch {
      // KaTeX returns rendered error HTML even with throwOnError=false,
      // so this branch is rare. Leave text in place.
    }
    el.dataset.rendered = "1";
  }
  const block = container.querySelectorAll<HTMLElement>("div.math-block");
  for (const el of block) {
    if (el.dataset.rendered === "1") continue;
    const src = el.textContent ?? "";
    try {
      el.innerHTML = katex.renderToString(src, {
        throwOnError: false,
        displayMode: true,
      });
    } catch {
      /* ignore */
    }
    el.dataset.rendered = "1";
  }
}
