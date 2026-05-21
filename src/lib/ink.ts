import type { InkPoint, InkStroke } from "./tauri";

export const INK_COLORS = [
  "#cdd6f4",
  "#b4befe",
  "#f38ba8",
  "#fab387",
  "#a6e3a1",
  "#89dceb",
] as const;

export const INK_WIDTHS = {
  thin: 1.5,
  medium: 3,
  thick: 5.5,
} as const;

export type InkWidthName = keyof typeof INK_WIDTHS;

/**
 * Two iterations of Chaikin's corner-cutting smoothing. Each iteration
 * replaces every segment of consecutive points with two points 25% and 75%
 * along that segment, preserving start and end. The pressure / tilt /
 * timestamp values are interpolated linearly alongside x / y.
 */
export function chaikin(points: InkPoint[], iterations = 2): InkPoint[] {
  let pts = points;
  for (let iter = 0; iter < iterations; iter++) {
    if (pts.length < 3) break;
    const next: InkPoint[] = [pts[0]];
    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i];
      const b = pts[i + 1];
      const q: InkPoint = [
        a[0] * 0.75 + b[0] * 0.25,
        a[1] * 0.75 + b[1] * 0.25,
        a[2] * 0.75 + b[2] * 0.25,
        a[3] * 0.75 + b[3] * 0.25,
        a[4] * 0.75 + b[4] * 0.25,
        a[5] * 0.75 + b[5] * 0.25,
      ];
      const r: InkPoint = [
        a[0] * 0.25 + b[0] * 0.75,
        a[1] * 0.25 + b[1] * 0.75,
        a[2] * 0.25 + b[2] * 0.75,
        a[3] * 0.25 + b[3] * 0.75,
        a[4] * 0.25 + b[4] * 0.75,
        a[5] * 0.25 + b[5] * 0.75,
      ];
      next.push(q, r);
    }
    next.push(pts[pts.length - 1]);
    pts = next;
  }
  return pts;
}

/**
 * Build a centerline SVG path string from a sequence of points using
 * quadratic Bézier segments through the midpoints (classic smoothing).
 */
export function buildPath(points: InkPoint[]): string {
  if (points.length === 0) return "";
  if (points.length === 1) {
    const [x, y] = points[0];
    return `M${x} ${y} L${x + 0.1} ${y + 0.1}`;
  }
  let d = `M${points[0][0].toFixed(2)} ${points[0][1].toFixed(2)}`;
  for (let i = 1; i < points.length - 1; i++) {
    const mx = (points[i][0] + points[i + 1][0]) / 2;
    const my = (points[i][1] + points[i + 1][1]) / 2;
    d += ` Q${points[i][0].toFixed(2)} ${points[i][1].toFixed(2)} ${mx.toFixed(2)} ${my.toFixed(2)}`;
  }
  const last = points[points.length - 1];
  d += ` L${last[0].toFixed(2)} ${last[1].toFixed(2)}`;
  return d;
}

/**
 * Compute the average pressure-modulated width for a stroke (used as the
 * stroke-width attribute on the SVG path). This is a cheap approximation of
 * "variable width" — true variable width would require generating an outline
 * polygon, which is expensive in JS. Good enough for handwriting.
 */
export function averageWidth(points: InkPoint[], base: number): number {
  if (points.length === 0) return base;
  let sum = 0;
  for (const p of points) sum += 0.5 + p[2];
  const avg = sum / points.length;
  return base * avg;
}

/** Axis-aligned bounding box for a stroke. */
export function strokeBBox(stroke: InkStroke): [number, number, number, number] {
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const [x, y] of stroke.points) {
    if (x < minX) minX = x;
    if (y < minY) minY = y;
    if (x > maxX) maxX = x;
    if (y > maxY) maxY = y;
  }
  const pad = stroke.width;
  return [minX - pad, minY - pad, maxX + pad, maxY + pad];
}

/**
 * Does the eraser path (treated as a sequence of small AABBs) intersect any
 * portion of `stroke`'s bounding box? This matches the spec's bbox-intersect
 * eraser behavior.
 */
export function eraseHit(stroke: InkStroke, eraserPath: InkPoint[]): boolean {
  const [sx0, sy0, sx1, sy1] = strokeBBox(stroke);
  const r = Math.max(stroke.width, 8);
  for (const [x, y] of eraserPath) {
    if (x + r >= sx0 && x - r <= sx1 && y + r >= sy0 && y - r <= sy1) {
      return true;
    }
  }
  return false;
}
