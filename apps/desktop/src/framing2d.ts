export interface Point2D {
  x: number;
  y: number;
}
export interface Rect2D {
  left: number;
  top: number;
  right: number;
  bottom: number;
}
export interface Camera2D {
  x: number;
  y: number;
  ratio: number;
  angle: number;
}
export interface DockedPanel {
  dock: "left" | "right" | "top" | "bottom";
  rect: Rect2D;
}
/** Keep the canvas full-window while reserving the measured floating HUD. */
export function availableGraphRect(
  width: number,
  height: number,
  panels: DockedPanel[],
  padding = 24,
): Rect2D | null {
  const rect = {
    left: padding,
    top: padding,
    right: width - padding,
    bottom: height - padding,
  };
  for (const { dock, rect: panel } of panels) {
    if (panel.right <= panel.left || panel.bottom <= panel.top) continue;
    if (dock === "left") rect.left = Math.max(rect.left, panel.right + padding);
    if (dock === "right")
      rect.right = Math.min(rect.right, panel.left - padding);
    if (dock === "top") rect.top = Math.max(rect.top, panel.bottom + padding);
    if (dock === "bottom")
      rect.bottom = Math.min(rect.bottom, panel.top - padding);
  }
  return [width, height, ...Object.values(rect)].every(Number.isFinite) &&
    rect.right - rect.left > 32 &&
    rect.bottom - rect.top > 32
    ? rect
    : null;
}
export function displayNodeLabel(label: string): string {
  const characters = Array.from(label);
  return characters.length > 34
    ? characters.slice(0, 31).join("") + "…"
    : label;
}
/** Public coordinate conversions keep Sigma's normalization/rotation authoritative. */
export function fitGraphCamera(
  points: Iterable<Point2D>,
  rect: Rect2D,
  current: Camera2D,
  projection: {
    project: (point: Point2D, state: Camera2D) => Point2D;
    framed: (point: Point2D, state: Camera2D) => Point2D;
  },
  labelWidth = 0,
): Camera2D | null {
  const inner = {
    left: rect.left + 24,
    top: rect.top + 24,
    right:
      rect.right -
      Math.min(
        Math.max(28, labelWidth + 28),
        Math.max(28, (rect.right - rect.left - 48) / 2),
      ),
    bottom: rect.bottom - 24,
  };
  if (
    inner.right <= inner.left ||
    inner.bottom <= inner.top ||
    !Number.isFinite(current.ratio) ||
    current.ratio <= 0
  )
    return null;
  let left = Infinity,
    right = -Infinity,
    top = Infinity,
    bottom = -Infinity,
    count = 0;
  for (const point of points) {
    const pixel = projection.project(point, current);
    if (!Number.isFinite(pixel.x) || !Number.isFinite(pixel.y)) return null;
    left = Math.min(left, pixel.x);
    right = Math.max(right, pixel.x);
    top = Math.min(top, pixel.y);
    bottom = Math.max(bottom, pixel.y);
    count++;
  }
  if (!count) return null;
  const scale = Math.min(
    4,
    (inner.right - inner.left) / Math.max(40, right - left),
    (inner.bottom - inner.top) / Math.max(40, bottom - top),
  );
  if (!Number.isFinite(scale) || scale <= 0) return null;
  // A one-position projection has no span to fit. Repeated island toggles
  // must not multiply its zoom every time the available region changes.
  const next = {
    ...current,
    ratio:
      right - left < 1e-9 && bottom - top < 1e-9
        ? 1
        : current.ratio / scale,
  };
  const centre = projection.framed(
    { x: (left + right) / 2, y: (top + bottom) / 2 },
    current,
  );
  const destination = projection.framed(
    { x: (inner.left + inner.right) / 2, y: (inner.top + inner.bottom) / 2 },
    next,
  );
  next.x += centre.x - destination.x;
  next.y += centre.y - destination.y;
  return Object.values(next).every(Number.isFinite) ? next : null;
}
