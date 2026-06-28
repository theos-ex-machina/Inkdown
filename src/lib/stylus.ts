import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Native stylus events, emitted by the Rust backend on Linux where WebKitGTK
 * hides the pen from the DOM (reports it as a mouse). On Windows/macOS the
 * DOM's own `pointerType === "pen"` events are used instead and these never
 * fire.
 *
 * Coordinates are CSS pixels relative to the webview viewport — the same
 * space as `PointerEvent.clientX/Y`.
 */
export interface StylusEvent {
  /**
   * `proximity` = pen hovering (or pen contact outside the ink canvas);
   * `down` / `motion` / `up` = a captured stroke on the canvas.
   */
  phase: "proximity" | "down" | "motion" | "up";
  x: number;
  y: number;
  /** 0..1; 0.5 when the tool doesn't report pressure. */
  pressure: number;
  tiltX: number;
  tiltY: number;
  /** True when the eraser end of the pen is in use. */
  eraser: boolean;
}

export function onStylus(
  handler: (e: StylusEvent) => void,
): Promise<UnlistenFn> {
  return listen<StylusEvent>("stylus", (event) => handler(event.payload));
}
