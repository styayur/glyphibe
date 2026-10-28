import { expect, test } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { PNG } from "pngjs";
import type { CodeGraph } from "../../src/model";
function dots(bytes: Buffer) {
  const image = PNG.sync.read(bytes),
    visited = new Uint8Array(image.width * image.height),
    boxes: {
      left: number;
      right: number;
      top: number;
      bottom: number;
      count: number;
    }[] = [];
  const coloured = (index: number) => {
    const offset = index * 4,
      r = image.data[offset],
      g = image.data[offset + 1],
      b = image.data[offset + 2];
    return (
      (r === 120 && g === 190 && b === 194) ||
      (r === 244 && g === 213 && b === 142) ||
      (r === 223 && g === 155 && b === 108)
    );
  };
  for (let index = 0; index < visited.length; index++) {
    if (visited[index] || !coloured(index)) continue;
    const pending = [index],
      box = {
        left: image.width,
        right: 0,
        top: image.height,
        bottom: 0,
        count: 0,
      };
    while (pending.length) {
      const next = pending.pop()!;
      if (
        next < 0 ||
        next >= visited.length ||
        visited[next] ||
        !coloured(next)
      )
        continue;
      visited[next] = 1;
      const x = next % image.width,
        y = Math.floor(next / image.width);
      box.left = Math.min(box.left, x);
      box.right = Math.max(box.right, x);
      box.top = Math.min(box.top, y);
      box.bottom = Math.max(box.bottom, y);
      box.count++;
      if (x > 0) pending.push(next - 1);
      if (x + 1 < image.width) pending.push(next + 1);
      pending.push(next - image.width, next + image.width);
    }
    if (box.count >= 8) boxes.push(box);
  }
  return boxes.sort((a, b) => a.left - b.left);
}
test("Frontier nodes remain between the measured floating islands and highlighting preserves a user pan", async ({
  page,
}) => {
  const file =
      process.env.GLYPHIBE_TEST_GRAPH ??
      path.resolve("tests/data/fixture-graph.json"),
    fixture = JSON.parse(await readFile(file, "utf8")) as CodeGraph;
  const call = fixture.edges.find(
    (edge) =>
      edge.relation === "calls" &&
      !edge.stale &&
      edge.source !== edge.target &&
      fixture.nodes[edge.source].kind === "function" &&
      fixture.nodes[edge.target].kind === "function",
  )!;
  expect(call).toBeTruthy();
  const pair = {
    ...fixture,
    nodes: {
      [call.source]: fixture.nodes[call.source],
      [call.target]: fixture.nodes[call.target],
    },
    edges: [call],
  };
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles({
    name: "source-derived-call-pair.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(pair)),
  });
  await page
    .getByRole("listitem")
    .filter({
      has: page.getByText(pair.nodes[call.source].label, { exact: true }),
    })
    .click();
  await page.getByRole("button", { name: "Frontier", exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-nodes",
    "2",
  );
  const left = (await page.locator(".left-island").boundingBox())!,
    right = (await page.locator(".detail").boundingBox())!,
    top = (await page.locator(".modes").boundingBox())!,
    bottom = (await page.locator(".controls").boundingBox())!;
  await expect
    .poll(async () => {
      const points = dots(await page.screenshot());
      return (
        points.length === 2 &&
        points.every(
          (p) =>
            p.left > left.x + left.width + 8 &&
            p.right < right.x - 8 &&
            p.top > top.y + top.height + 8 &&
            p.bottom < bottom.y - 8,
        )
      );
    })
    .toBe(true);
  const initial = dots(await page.screenshot());
  await page.mouse.move(700, 450);
  await page.mouse.down();
  await page.mouse.move(730, 465, { steps: 8 });
  await page.mouse.up();
  await expect
    .poll(async () => {
      const now = dots(await page.screenshot());
      return now.length === 2 && Math.abs(now[0].left - initial[0].left) > 15;
    })
    .toBe(true);
  const panned = dots(await page.screenshot());
  await page.getByLabel("Impact hops").focus();
  await page.keyboard.press("End");
  const after = dots(await page.screenshot());
  expect(after).toHaveLength(2);
  expect(Math.abs(after[0].left - panned[0].left)).toBeLessThan(3);
  expect(Math.abs(after[1].left - panned[1].left)).toBeLessThan(3);
  await page.getByRole("button", { name: "Focus", exact: true }).click();
  await expect(page.locator(".left-island")).toBeHidden();
  await expect
    .poll(async () => {
      const focused = dots(await page.screenshot());
      return (
        focused.length === 2 &&
        focused[1].right - focused[0].left >
          initial[1].right - initial[0].left + 50
      );
    })
    .toBe(true);
});
