export type Handler = (e: KeyboardEvent) => void | boolean;

interface Binding {
  combo: string;
  handler: Handler;
}

const bindings: Binding[] = [];

function normalize(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  parts.push(key);
  return parts.join("+");
}

function dispatcher(e: KeyboardEvent) {
  const combo = normalize(e);
  for (const b of bindings) {
    if (b.combo === combo) {
      const stop = b.handler(e);
      if (stop !== false) {
        e.preventDefault();
        e.stopPropagation();
      }
      return;
    }
  }
}

let attached = false;
export function registerShortcut(combo: string, handler: Handler): () => void {
  if (!attached) {
    window.addEventListener("keydown", dispatcher);
    attached = true;
  }
  const binding: Binding = { combo, handler };
  bindings.push(binding);
  return () => {
    const idx = bindings.indexOf(binding);
    if (idx !== -1) bindings.splice(idx, 1);
  };
}
