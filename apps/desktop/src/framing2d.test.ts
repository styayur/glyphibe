import { expect, it } from "vitest";
import { availableGraphRect, fitGraphCamera } from "./framing2d";
import type { Camera2D, Point2D } from "./framing2d";
const width = 1440,
  height = 960;
const projection = {
  project(p: Point2D, s: Camera2D) {
    const dx = p.x - s.x,
      dy = p.y - s.y,
      c = Math.cos(s.angle),
      t = Math.sin(s.angle),
      scale = Math.min(width, height) / s.ratio;
    return {
      x: width / 2 + (dx * c - dy * t) * scale,
      y: height / 2 - (dx * t + dy * c) * scale,
    };
  },
  framed(p: Point2D, s: Camera2D) {
    const x = ((p.x - width / 2) * s.ratio) / Math.min(width, height),
      y = (-(p.y - height / 2) * s.ratio) / Math.min(width, height),
      c = Math.cos(s.angle),
      t = Math.sin(s.angle);
    return { x: s.x + x * c + y * t, y: s.y - x * t + y * c };
  },
};
it("fits two opposite points between the actual left/right/top/bottom HUD bounds", () => {
  const area = availableGraphRect(width, height, [
    { dock: "left", rect: { left: 16, top: 69, right: 261, bottom: 700 } },
    { dock: "right", rect: { left: 1084, top: 69, right: 1424, bottom: 840 } },
    { dock: "top", rect: { left: 526, top: 69, right: 914, bottom: 114 } },
    { dock: "bottom", rect: { left: 365, top: 832, right: 1075, bottom: 942 } },
  ])!;
  const points = [
    { x: 0.02, y: 0.05 },
    { x: 0.98, y: 0.95 },
  ];
  for (const angle of [0, 0.4, -0.8]) {
    const frame = fitGraphCamera(
      points,
      area,
      { x: 0.5, y: 0.5, ratio: 1, angle },
      projection,
      90,
    )!;
    for (const point of points) {
      const p = projection.project(point, frame);
      expect(p.x).toBeGreaterThanOrEqual(area.left + 23);
      expect(p.x + 90).toBeLessThan(area.right);
      expect(p.y).toBeGreaterThanOrEqual(area.top + 23);
      expect(p.y).toBeLessThanOrEqual(area.bottom - 23);
    }
  }
  const focus = availableGraphRect(width, height, [])!;
  expect(focus.right - focus.left).toBeGreaterThan(area.right - area.left);
  const focused = fitGraphCamera(
    points,
    focus,
    { x: 0.5, y: 0.5, ratio: 1, angle: 0 },
    projection,
    90,
  )!;
  expect(focused.ratio).toBeLessThan(
    fitGraphCamera(
      points,
      area,
      { x: 0.5, y: 0.5, ratio: 1, angle: 0 },
      projection,
      90,
    )!.ratio,
  );
});
it("handles empty/degenerate projections without an invalid camera", () => {
  const area = availableGraphRect(width, height, [])!,
    camera = { x: 0.5, y: 0.5, ratio: 1, angle: 0 };
  expect(fitGraphCamera([], area, camera, projection)).toBeNull();
  const singleton = fitGraphCamera(
    [{ x: 0.5, y: 0.5 }],
    area,
    camera,
    projection,
  )!;
  expect(singleton.ratio).toBeGreaterThan(0);
  expect(Object.values(singleton).every(Number.isFinite)).toBe(true);
  expect(
    fitGraphCamera(
      [{ x: 0.5, y: 0.5 }],
      { ...area, left: area.left + 200 },
      singleton,
      projection,
    )!.ratio,
  ).toBe(singleton.ratio);
  const narrow = { left: 240, top: 160, right: 530, bottom: 480 };
  const withLongLabel = fitGraphCamera(
    [
      { x: 0.02, y: 0.05 },
      { x: 0.98, y: 0.95 },
    ],
    narrow,
    camera,
    projection,
    400,
  )!;
  expect(withLongLabel).toBeTruthy();
  for (const point of [
    { x: 0.02, y: 0.05 },
    { x: 0.98, y: 0.95 },
  ]) {
    const pixel = projection.project(point, withLongLabel);
    expect(pixel.x).toBeGreaterThan(narrow.left);
    expect(pixel.x).toBeLessThan(narrow.right);
    expect(pixel.y).toBeGreaterThan(narrow.top);
    expect(pixel.y).toBeLessThan(narrow.bottom);
  }
  expect(availableGraphRect(20, 20, [])).toBeNull();
});
