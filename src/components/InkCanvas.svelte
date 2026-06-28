<script lang="ts">
  import { onDestroy, onMount } from "svelte";
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
  import { onStylus, type StylusEvent } from "../lib/stylus";

  type Props = {
    virtualPath: string;
    active: boolean;
    color: string;
    width: InkWidthName;
    /**
     * Fired when a mouse click lands on the overlay while ink mode is
     * active. Parent typically reverts to type mode and positions the
     * editor cursor at the click coordinates.
     */
    onCancel?: (clientX: number, clientY: number) => void;
    /**
     * Fired when a native (Linux) stylus stroke starts. Parent should
     * switch to ink mode so the editor stops accepting input.
     */
    onNativePenDown?: () => void;
  };

  let { virtualPath, active, color, width, onCancel, onNativePenDown }: Props =
    $props();

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

  function isDomEraser(e: PointerEvent): boolean {
    return e.pointerType === "pen" && (e.buttons === 32 || e.button === 5);
  }

  /**
   * DOM pointer path — used on Windows/macOS where the WebView reports
   * `pointerType === "pen"`. On Linux the pen never reaches the DOM: the
   * Rust backend claims it at the GTK layer and streams it via `onStylus`
   * below, so DOM events here can only be a real mouse or a touch.
   */
  function onPointerDown(e: PointerEvent) {
    if (!active || !svgEl) return;
    // Palm rejection: swallow touch without leaving ink mode.
    if (e.pointerType === "touch") {
      e.preventDefault();
      return;
    }
    // Mouse click reverts to type mode at the click position.
    if (e.pointerType !== "pen") {
      e.preventDefault();
      onCancel?.(e.clientX, e.clientY);
      return;
    }
    svgEl.setPointerCapture(e.pointerId);
    e.preventDefault();
    const p = localPoint(e);
    if (isDomEraser(e)) {
      erasing = [p];
    } else {
      drawing = [p];
    }
  }

  function onPointerMove(e: PointerEvent) {
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
    if (!active || !svgEl || nativeStroke) return;
    if (svgEl.hasPointerCapture(e.pointerId)) {
      svgEl.releasePointerCapture(e.pointerId);
    }
    if (drawing) commitStroke();
    else if (erasing) commitErase();
  }

  function onPointerCancel(_e: PointerEvent) {
    if (nativeStroke) return;
    drawing = null;
    erasing = null;
  }

  // --- Native stylus path (Linux) -----------------------------------------
  // WebKitGTK reports pens as mice, so the Rust backend captures the tablet
  // tool at the GTK layer and streams it here. Coordinates arrive in CSS px
  // relative to the viewport, same space as clientX/Y.

  let nativeStroke = false;

  function nativePoint(e: StylusEvent): InkPoint {
    const rect = svgEl!.getBoundingClientRect();
    return [
      e.x - rect.left,
      e.y - rect.top,
      e.pressure || 0.5,
      e.tiltX || 0,
      e.tiltY || 0,
      performance.now(),
    ];
  }

  function onNativeStylus(e: StylusEvent) {
    if (!svgEl || e.phase === "proximity") return;
    if (e.phase === "down") {
      nativeStroke = true;
      onNativePenDown?.();
      const p = nativePoint(e);
      if (e.eraser) erasing = [p];
      else drawing = [p];
    } else if (e.phase === "motion") {
      if (drawing) drawing = [...drawing, nativePoint(e)];
      else if (erasing) erasing = [...erasing, nativePoint(e)];
    } else if (e.phase === "up") {
      if (drawing) commitStroke();
      else if (erasing) commitErase();
      nativeStroke = false;
    }
  }

  // Keep the backend's claim region in sync with where the canvas sits, so
  // pen input on the canvas is captured natively while pen input on the
  // toolbar / sidebar still works as a normal click.
  function reportRegion() {
    if (!svgEl) return;
    const r = svgEl.getBoundingClientRect();
    void cmd.setStylusRegion({ x: r.left, y: r.top, width: r.width, height: r.height });
  }

  onMount(() => {
    const unlistenPromise = onStylus(onNativeStylus);

    reportRegion();
    const observer = new ResizeObserver(reportRegion);
    if (svgEl) observer.observe(svgEl);
    window.addEventListener("resize", reportRegion);

    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
      observer.disconnect();
      window.removeEventListener("resize", reportRegion);
      void cmd.setStylusRegion(null);
    };
  });

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
