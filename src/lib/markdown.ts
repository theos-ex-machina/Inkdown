import { cmd } from "./tauri";
import { renderMathIn } from "./katex";

/**
 * Backend-rendered markdown -> sanitized HTML string. Caller is expected to
 * inject the result into a container (e.g. via `bind:innerHTML`) and then
 * pass that container to `renderMathIn` so KaTeX upgrades the `<span class="math-*">`
 * placeholders into real math.
 *
 * Returns an empty string on error (so the UI stays calm instead of toasting
 * on every transient render miss).
 */
export async function renderMarkdown(src: string): Promise<string> {
  if (!src) return "";
  try {
    return await cmd.renderMarkdown(src);
  } catch (err) {
    console.warn("[markdown] render failed:", err);
    return "";
  }
}

export { renderMathIn };
