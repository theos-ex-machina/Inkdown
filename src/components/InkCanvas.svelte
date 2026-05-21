<script lang="ts">
  import { onDestroy } from "svelte";
  import { cmd, type InkPoint, type InkStroke } from "../lib/tauri";
  import { debounce } from "../lib/debounce";
  import {
    INK_WIDTHS,
    averageWidth,
    buildPath,
    chaikin,
    eraseHit,
    type InkWidthName,
  } from "../lib/ink";

  type Props = {
    virtualPath: string;
    active: boolean;
    color: string;
    width: InkWidthName;
    /**
     * Fired when a non-pen pointer (mouse / touch) lands on the overlay
     * while ink mode is active. Parent typically reverts to type mode and
     * positions the editor cursor at the click coordinates.
     */
    onCancel?: (clientX: number, clientY: number) => void;
  };

  let { virtualPath, active, color, width, onCancel }: Props = $props();

  let svgEl: SVGSVGElement | null = $state(null);
  let strokes = $state<InkStroke[]>([]);
  let drawing: InkPoint[] | null = $state(null);
  let erasing: InkPoint[] | null = $state(null);
  let currentColor = $derived(color);
  let currentWidth = $derived(INK_WIDTHS[width]);

  let currentPath = "";

  async function loadFor(path: string) {
    currentPath = path;
    const doc = await cmd.readInk(path);
    if (currentPath !== path) return;
    strokes = doc?.strokes ?? [];
  }

  const saveDebounced = debounce(async (path: string, list: InkStroke[]) => {
    await cmd.writeInk(path, { version: 1, strokes: list });
  }, 500);

  $effect(() => {
    if (virtualPath) {
      void loadFor(virtualPath);
    }
  });

  function localPoint(e: PointerEvent): InkPoint {
    const rect = svgEl!.getBoundingClientRect();
    return [
      e.clientX - rect.left,
      e.clientY - rect.top,
      e.pressure || 0.5,
      e.tiltX || 0,
      e.tiltY || 0,
      e.timeStamp,
    ];
  }

  function isEraser(e: PointerEvent): boolean {
    return e.pointerType === "pen" && (e.buttons === 32 || (e as PointerEvent & { button?: number }).button === 5);
  }

  function onPointerDown(e: PointerEvent) {
    if (!active || !svgEl) return;
    // Mouse / touch clicks land on the overlay because it's opaque while
    // ink mode is active. We swallow them and tell the parent to revert
    // to type mode + position the editor cursor at the click coordinates.
    if (e.pointerType !== "pen") {
      e.preventDefault();
      onCancel?.(e.clientX, e.clientY);
      return;
    }
    svgEl.setPointerCapture(e.pointerId);
    e.preventDefault();
    const p = localPoint(e);
    if (isEraser(e)) {
      erasing = [p];
    } else {
      drawing = [p];
    }
  }

  function onPointerMove(e: PointerEvent) {
    // Once a stroke has started we trust the pointer-capture and accept
    // every event regardless of mode flickering — that fix is what stops
    // strokes from collapsing to straight lines if a palm-touch sneaks in.
    if (e.pointerType !== "pen") return;
    if (drawing) {
      drawing = [...drawing, localPoint(e)];
    } else if (erasing) {
      erasing = [...erasing, localPoint(e)];
    }
  }

  function commitStroke() {
    if (!drawing || drawing.length < 2) {
      drawing = null;
      return;
    }
    const smooth = chaikin(drawing, 2);
    const stroke: InkStroke = {
      points: smooth,
      color: currentColor,
      width: currentWidth,
    };
    strokes = [...strokes, stroke];
    drawing = null;
    saveDebounced(virtualPath, strokes);
  }

  function commitErase() {
    if (!erasing) return;
    const path = erasing;
    erasing = null;
    const kept = strokes.filter((s) => !eraseHit(s, path));
    if (kept.length !== strokes.length) {
      strokes = kept;
      saveDebounced(virtualPath, strokes);
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (!active || !svgEl) return;
    if (svgEl.hasPointerCapture(e.pointerId)) {
      svgEl.releasePointerCapture(e.pointerId);
    }
    if (drawing) commitStroke();
    else if (erasing) commitErase();
  }

  function onPointerCancel(_e: PointerEvent) {
    drawing = null;
    erasing = null;
  }

  onDestroy(() => {
    saveDebounced.flush();
  });

  // Public method: clear strokes for current page (TODO: expose via menu)
  export function clearAll() {
    strokes = [];
    saveDebounced(virtualPath, strokes);
  }

  let drawingPath = $derived(drawing ? buildPath(drawing) : "");
  let drawingWidth = $derived(drawing ? averageWidth(drawing, currentWidth) : currentWidth);
</script>

<svg
  bind:this={svgEl}
  class="ink-overlay"
  class:active
  role="img"
  aria-label="Ink canvas"
  width="100%"
  height="100%"
  preserveAspectRatio="none"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
>
    {#each strokes as stroke, i (i)}
      <path
        d={buildPath(stroke.points)}
        stroke={stroke.color}
        stroke-width={averageWidth(stroke.points, stroke.width)}
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    {/each}
    {#if drawing}
      <path
        d={drawingPath}
        stroke={currentColor}
        stroke-width={drawingWidth}
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    {/if}
    {#if erasing}
      <path
        d={buildPath(erasing)}
        stroke="rgba(243, 139, 168, 0.35)"
        stroke-width={12}
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    {/if}
</svg>

<style>
  .ink-overlay {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 5;
    touch-action: none;
    overflow: visible;
  }
  .ink-overlay.active {
    pointer-events: auto;
    cursor: crosshair;
  }
</style>
