import { expect, test } from "@playwright/test";
import path from "node:path";
import { access, readFile } from "node:fs/promises";
import { PNG } from "pngjs";
import type { CodeGraph } from "../../src/model";
const graphPath =
  process.env.GLYPHIBE_TEST_GRAPH ??
  path.resolve("tests/data/fixture-graph.json");
test.beforeAll(async () => {
  await access(graphPath);
});

test("Galaxy sphere clicks select a canonical node without a preselected entity", async ({
  page,
}) => {
  await page.goto("/");
  const fixture = JSON.parse(await readFile(graphPath, "utf8")) as CodeGraph;
  const nodes = Object.fromEntries(
    Object.entries(fixture.nodes)
      .filter(([, n]) => n.kind === "function" && n.plane === "architecture")
      .slice(0, 17),
  );
  await page.getByLabel("Import graph JSON").setInputFiles({
    name: "source-derived-17.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({
        ...fixture,
        nodes,
        edges: fixture.edges.filter((e) => nodes[e.source] && nodes[e.target]),
      }),
    ),
  });
  await page.getByRole("button", { name: "Galaxy", exact: true }).click();
  await expect(
    page.getByTestId("graph-canvas").locator("canvas").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Focus", exact: true }).click();
  let point: { x: number; y: number } | undefined;
  await expect
    .poll(async () => {
      const png = PNG.sync.read(await page.screenshot());
      const teal = (x: number, y: number) => {
        const i = (y * png.width + x) * 4;
        const [r, g, b] = png.data.subarray(i, i + 3);
        return g > 65 && b > 65 && g > r * 1.2 && b > r * 1.2;
      };
      let tealPixels = 0;
      for (let y = 0; y < png.height; y += 2)
        for (let x = 0; x < png.width; x += 2) if (teal(x, y)) tealPixels++;
      // An initial camera inside a sphere can still produce a clickable canvas.
      // Require a genuinely fitted sparse graph, not a viewport-filling sphere.
      if (tealPixels * 4 > png.width * png.height * 0.1) return false;
      for (let y = 80; y < png.height - 80; y += 2)
        for (let x = 80; x < png.width - 80; x += 2)
          if (
            teal(x, y) &&
            teal(x - 5, y) &&
            teal(x + 5, y) &&
            teal(x, y - 5) &&
            teal(x, y + 5)
          ) {
            point = { x, y };
            return true;
          }
      return false;
    })
    .toBe(true);
  await page.mouse.move(point!.x, point!.y);
  await page.mouse.down();
  // A normal click may include sub-threshold pointer jitter, including WebView2
  // input injection. It must remain a click rather than an orbit gesture.
  await page.mouse.move(point!.x + 1, point!.y);
  await page.mouse.up();
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Node details", { exact: true })).toBeVisible();
  await expect(
    page.getByLabel("Node details", { exact: true }).locator("h2"),
  ).not.toBeEmpty();
});

test("Cluster grain changes the real projection through all six levels", async ({
  page,
}) => {
  const fixture = JSON.parse(await readFile(graphPath, "utf8")) as CodeGraph;
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles(graphPath);
  await page.getByRole("button", { name: "Cluster", exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-layout",
    "community",
  );
  for (let level = 1; level <= 6; level++) {
    await page.getByLabel("Architecture level").focus();
    await page.keyboard.press("Home");
    for (let step = 1; step < level; step++)
      await page.keyboard.press("ArrowRight");
    const expected = Object.values(fixture.nodes).filter(
      (n) =>
        n.plane === "architecture" &&
        n.change !== "removed" &&
        n.level === level,
    ).length;
    await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
      "data-nodes",
      String(expected),
    );
  }
});
test("malformed source span is rejected before node content can crash React", async ({
  page,
}) => {
  const fixture = JSON.parse(await readFile(graphPath, "utf8")) as CodeGraph;
  const node = Object.values(fixture.nodes).find((n) => n.source)!;
  (node.source as unknown as { path: unknown }).path = { malformed: true };
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles({
    name: "invalid-span.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(fixture)),
  });
  await expect(page.getByRole("alert")).toContainText("Invalid graph node");
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
  await expect(
    page.getByText("See how your code fits together."),
  ).toBeVisible();
  expect(errors).toEqual([]);
});
test("selection reuses the 2D WebGL canvas and software 3D limits can be overridden", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles(graphPath);
  const canvas = await page
    .getByTestId("sigma-canvas")
    .locator("canvas")
    .first()
    .elementHandle();
  await page
    .getByRole("listitem")
    .filter({ hasText: "function" })
    .first()
    .click();
  await expect(page.getByLabel("Node details", { exact: true })).toBeVisible();
  expect(
    await page.evaluate(
      (element) =>
        document.querySelector('[data-testid="sigma-canvas"] canvas') ===
        element,
      canvas,
    ),
  ).toBe(true);
  await page.getByLabel("Impact hops").focus();
  await page.keyboard.press("End");
  expect(
    await page.evaluate(
      (element) =>
        document.querySelector('[data-testid="sigma-canvas"] canvas') ===
        element,
      canvas,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Galaxy", exact: true }).click();
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-cap",
    "500",
  );
  await page.getByText("Appearance & language").click();
  await page.getByLabel("Galaxy rendering detail").selectOption("full");
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-cap",
    "2000",
  );
  await page.getByLabel("Galaxy rendering detail").selectOption("auto");
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-cap",
    "500",
  );
});
test("real canvas supports pixel-picked selection, pan, zoom and 3D orbit", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles(graphPath);
  await expect(
    page.getByTestId("sigma-canvas").locator("canvas").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Focus", exact: true }).click();
  let point: { x: number; y: number } | undefined;
  await expect
    .poll(async () => {
      const png = PNG.sync.read(await page.screenshot());
      for (let y = 100; y < png.height - 70; y++)
        for (let x = 60; x < png.width - 60; x++) {
          const i = (y * png.width + x) * 4;
          if (
            png.data[i] === 120 &&
            png.data[i + 1] === 190 &&
            png.data[i + 2] === 194
          ) {
            // Pick the centre of a connected node interior, not its antialiased rim.
            const pending = [[x, y]];
            const visited = new Set<number>();
            let sx = 0,
              sy = 0,
              count = 0;
            while (pending.length) {
              const [cx, cy] = pending.pop()!;
              if (cx < 0 || cy < 0 || cx >= png.width || cy >= png.height)
                continue;
              const key = cy * png.width + cx;
              if (visited.has(key)) continue;
              visited.add(key);
              const p = key * 4;
              if (
                png.data[p] !== 120 ||
                png.data[p + 1] !== 190 ||
                png.data[p + 2] !== 194
              )
                continue;
              sx += cx;
              sy += cy;
              count++;
              pending.push(
                [cx - 1, cy],
                [cx + 1, cy],
                [cx, cy - 1],
                [cx, cy + 1],
              );
            }
            if (count < 8) continue;
            point = { x: sx / count, y: sy / count };
            return true;
          }
        }
      return false;
    })
    .toBe(true);
  await page.mouse.click(point!.x, point!.y);
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Node details", { exact: true })).toBeVisible();
  const selectedBeforeOrbit = await page
    .getByLabel("Node details", { exact: true })
    .locator("h2")
    .innerText();
  await page.getByRole("button", { name: "Focus", exact: true }).click();
  const before = await page.screenshot();
  await page.mouse.move(700, 450);
  await page.mouse.down();
  await page.mouse.move(850, 500, { steps: 12 });
  await page.mouse.up();
  await page.mouse.wheel(0, -300);
  await expect
    .poll(async () => !(await page.screenshot()).equals(before))
    .toBe(true);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Galaxy", exact: true }).click();
  await expect(
    page.getByTestId("graph-canvas").locator("canvas").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Focus", exact: true }).click();
  const galaxyBefore = await page.screenshot();
  await page.mouse.move(700, 450);
  await page.mouse.down();
  await page.mouse.move(850, 550, { steps: 15 });
  await page.mouse.up();
  await expect
    .poll(async () => !(await page.screenshot()).equals(galaxyBefore))
    .toBe(true);
  await page.keyboard.press("Escape");
  await expect(
    page.getByLabel("Node details", { exact: true }).locator("h2"),
  ).toHaveText(selectedBeforeOrbit);
});
test("plane mappings, hierarchy and all floating islands are operable", async ({
  page,
}) => {
  const fixture = JSON.parse(await readFile(graphPath, "utf8")) as CodeGraph;
  const origin = Object.values(fixture.nodes).find(
    (n) =>
      n.kind === "function" &&
      fixture.edges.some((e) => e.source === n.id && e.relation === "maps_to"),
  )!;
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles(graphPath);
  await page.getByRole("searchbox").fill(origin.label);
  await page
    .getByRole("listitem")
    .filter({ hasText: origin.label })
    .first()
    .click();
  await expect(page.getByLabel("Architecture level")).toHaveValue("5");
  await page
    .getByText("Trace upward through structure", { exact: false })
    .click();
  await expect(page.locator(".hierarchy-entry").first()).toBeVisible();
  const mapping = page
    .locator(".relations button")
    .filter({ hasText: "maps_to" })
    .first();
  const target = await mapping.locator("strong").innerText();
  await mapping.click();
  await expect(page.getByLabel("Graph plane")).toHaveValue("explanation");
  await expect(
    page.getByRole("heading", { name: target, exact: true }),
  ).toBeVisible();
  for (const title of ["Graph modes", "Graph controls", "Node details"]) {
    await page.getByLabel(`Collapse ${title}`, { exact: true }).click();
    await expect(
      page.getByLabel(`Collapse ${title}`, { exact: true }),
    ).toHaveAttribute("aria-expanded", "false");
    await page.getByLabel(`Collapse ${title}`, { exact: true }).click();
    await expect(
      page.getByLabel(`Collapse ${title}`, { exact: true }),
    ).toHaveAttribute("aria-expanded", "true");
    await page.getByLabel(`Pin ${title}`, { exact: true }).click();
    await expect(
      page.getByLabel(`Pin ${title}`, { exact: true }),
    ).toHaveAttribute("aria-pressed", "false");
    await page.getByLabel(`Pin ${title}`, { exact: true }).click();
  }
  await page.getByLabel("Collapse tools", { exact: true }).click();
  await expect(page.getByRole("searchbox")).toBeHidden();
  await page.getByLabel("Collapse tools", { exact: true }).click();
  await expect(page.getByRole("searchbox")).toBeVisible();
});
test("real exported graph renders in all modes with synchronized selection and controls", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(
    page.getByText("See how your code fits together."),
  ).toBeVisible();
  await page.getByLabel("Import graph JSON").setInputFiles(graphPath);
  await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
    "data-nodes",
    /^[1-9]\d*$/,
  );
  await expect(
    page.getByTestId("sigma-canvas").locator("canvas").first(),
  ).toBeVisible();
  const entity = page.getByRole("listitem").first();
  const label = await entity.locator("span").innerText();
  await entity.click();
  await expect(
    page.getByRole("heading", { name: label, exact: true }),
  ).toBeVisible();
  for (const mode of ["Frontier", "Dense", "Cluster", "Galaxy"]) {
    await page.getByRole("button", { name: mode, exact: true }).click();
    await expect(page.getByTestId("graph-canvas")).toHaveAttribute(
      "data-mode",
      mode,
    );
    await expect(
      page.getByTestId("graph-canvas").locator("canvas").first(),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: label, exact: true }),
    ).toBeVisible();
  }
  await page.getByRole("button", { name: "Dense", exact: true }).click();
  await page.getByLabel("Architecture level").focus();
  await page.keyboard.press("Home");
  await expect(page.getByLabel("Architecture level")).toHaveValue("1");
  await page.getByLabel("Explanation depth").focus();
  await page.keyboard.press("End");
  await expect(page.getByLabel("Explanation depth")).toHaveValue("4");
  await page.getByLabel("Impact direction").selectOption("downstream");
  await page.getByLabel("Impact hops").focus();
  await page.keyboard.press("End");
  await expect(page.getByLabel("Impact hops")).toHaveValue("10");
  await page.getByRole("tab", { name: "Source", exact: true }).click();
  await page.getByRole("tab", { name: "Learn", exact: true }).click();
  await expect(page.getByText("Start with the evidence")).toBeVisible();
  await page.getByRole("tab", { name: "Knowledge", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Concept links" }),
  ).toBeVisible();
  await page.getByText("Appearance & language").click();
  await page.getByLabel("Theme", { exact: true }).selectOption("light");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByLabel("Language", { exact: true }).selectOption("zh");
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-CN");
  await page.getByRole("button", { name: "专注", exact: true }).click();
  await expect(page.getByLabel("Graph mode")).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(page.getByLabel("Graph mode")).toBeVisible();
  expect(errors).toEqual([]);
});
test("untrusted malformed graph is rejected without executing its text", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Import graph JSON").setInputFiles({
    name: "malicious.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({
        schema_version: 1,
        nodes: {},
        edges: [{ source: "missing", target: "missing", relation: "calls" }],
        repository: "<script>alert(1)</script>",
        revision: "x",
        diagnostics: [],
        stats: {},
      }),
    ),
  });
  await expect(page.getByRole("alert")).toContainText("Invalid edge");
  await expect(page.getByTestId("graph-canvas")).toHaveCount(0);
});
