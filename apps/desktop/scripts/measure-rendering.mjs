// SPDX-License-Identifier: MPL-2.0
// Measures the real production UI, importing real source-derived graph exports.
import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { cpus, totalmem, platform, release, arch } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
  if (!process.argv[i].startsWith("--") || !process.argv[i + 1])
    throw new Error(
      "Usage: node scripts/measure-rendering.mjs --graphs DIR --out FILE [--duration 4000] [--repetitions 3] [--modes Dense,Galaxy] [--url URL]",
    );
  args.set(process.argv[i].slice(2), process.argv[i + 1]);
}
if (!args.has("graphs") || !args.has("out"))
  throw new Error("--graphs and --out are required.");
const graphRoot = path.resolve(args.get("graphs")),
  output = path.resolve(args.get("out"));
const duration = Number(args.get("duration") ?? 4000);
const repetitionCount = Number(args.get("repetitions") ?? 3);
const modes = (args.get("modes") ?? "Dense,Galaxy").split(",");
if (
  !Number.isInteger(repetitionCount) ||
  repetitionCount < 1 ||
  repetitionCount > 10 ||
  modes.some((mode) => !["Dense", "Galaxy"].includes(mode))
)
  throw new Error("Expected repetitions 1..10 and modes Dense,Galaxy.");
if (!Number.isFinite(duration) || duration < 2000 || duration > 30000)
  throw new Error("duration must be 2000..30000 milliseconds.");
const url = args.get("url") ?? "http://127.0.0.1:1421";
let server;
if (!args.has("url")) {
  server = spawn(
    process.execPath,
    [
      path.join(root, "node_modules/vite/bin/vite.js"),
      "preview",
      "--host",
      "127.0.0.1",
      "--port",
      "1421",
      "--strictPort",
    ],
    { cwd: root, windowsHide: true, stdio: "ignore" },
  );
  const until = Date.now() + 30000;
  while (true) {
    try {
      const response = await fetch(url);
      if (response.ok) break;
    } catch {}
    if (Date.now() > until)
      throw new Error(
        "Production preview failed to start. Run npm run build first.",
      );
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
}
const browser = await chromium.launch({
  headless: true,
  args: [
    "--use-gl=angle",
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
  ],
});
const report = {
  schema_version: 1,
  measured_at: new Date().toISOString(),
  environment: {
    os: platform(),
    os_release: release(),
    architecture: arch(),
    cpu: cpus()[0]?.model,
    logical_cpus: cpus().length,
    system_memory_bytes: totalmem(),
    node: process.version,
    browser: browser.version(),
    viewport: { width: 1440, height: 960 },
    device_scale_factor: 1,
    renderer_requested: "ANGLE SwiftShader (software)",
    production_build: true,
  },
  method: {
    duration_ms: duration,
    repetitions: repetitionCount,
    modes,
    interaction:
      "Repeated pointer pan/orbit and wheel zoom at the canvas center; standard glass overlays visible.",
    cadence:
      "requestAnimationFrame callback timestamps and WebGL draw submission activity; neither is a GPU-presented FPS measurement.",
    latency:
      "DOM input/click event to the second subsequent animation callback, including React projection and WebGL renderer reconstruction; not end-to-end GPU presentation.",
    graph_source:
      "Canonical function/call projection of genuinely parsed generated Python source. Function-only benchmark exports have no L4 nodes.",
    level_transition: "Function L5 to empty file L4 and back to populated L5.",
    limits:
      "Headless Chromium software rendering differs from installed WebView2 on hardware GPU. Measurements do not verify a native 30 FPS target.",
  },
  samples: [],
};
const round = (value) => Math.round(value * 100) / 100;
const quantile = (values, q) => {
  const ordered = [...values].sort((a, b) => a - b);
  return (
    ordered[Math.min(ordered.length - 1, Math.floor(ordered.length * q))] ?? 0
  );
};
try {
  for (const target of [1000, 5000, 10000]) {
    const graphFile = path.join(graphRoot, `graph-${target}.json`),
      bytes = await readFile(graphFile),
      graph = JSON.parse(bytes);
    const canonicalCount = Object.keys(graph.nodes).length,
      functions = Object.values(graph.nodes).filter(
        (n) =>
          n.plane === "architecture" && n.level === 5 && n.change !== "removed",
      ).length;
    for (const mode of modes) {
      const page = await browser.newPage({
        viewport: report.environment.viewport,
        deviceScaleFactor: 1,
      });
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await page.addInitScript(() => {
        window.__glyphibeRenderProbe = { drawCalls: 0 };
        for (const type of [
          window.WebGLRenderingContext,
          window.WebGL2RenderingContext,
        ]) {
          if (!type) continue;
          for (const method of [
            "drawArrays",
            "drawElements",
            "drawArraysInstanced",
            "drawElementsInstanced",
          ]) {
            const original = type.prototype[method];
            if (!original) continue;
            type.prototype[method] = function (...parameters) {
              window.__glyphibeRenderProbe.drawCalls++;
              return Reflect.apply(original, this, parameters);
            };
          }
        }
      });
      await page.goto(url);
      await page.waitForLoadState("networkidle");
      const importedAt = performance.now();
      await page.getByLabel("Import graph JSON").setInputFiles(graphFile);
      await page.getByTestId("graph-canvas").waitFor();
      await page.getByLabel("Graph density").focus();
      await page.keyboard.press("End");
      if (mode === "Galaxy")
        await page.getByRole("button", { name: "Galaxy", exact: true }).click();
      const effectiveCap =
        mode === "Galaxy"
          ? Number(
              await page.getByTestId("graph-canvas").getAttribute("data-cap"),
            )
          : 10000;
      const visible = Math.min(functions, effectiveCap);
      await page.waitForFunction(
        (count) =>
          Number(
            document
              .querySelector('[data-testid="graph-canvas"]')
              ?.getAttribute("data-nodes"),
          ) === count,
        visible,
      );
      await page
        .getByTestId("graph-canvas")
        .locator("canvas")
        .first()
        .waitFor({ state: "visible" });
      await page.evaluate(
        () =>
          new Promise((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(resolve)),
          ),
      );
      const importToSecondRaf = performance.now() - importedAt;
      const gpu = await page.evaluate(() => {
        const canvas = document.createElement("canvas"),
          gl = canvas.getContext("webgl2") ?? canvas.getContext("webgl");
        if (!gl) return { available: false };
        const ext = gl.getExtension("WEBGL_debug_renderer_info");
        const result = {
          available: true,
          vendor: ext
            ? gl.getParameter(ext.UNMASKED_VENDOR_WEBGL)
            : gl.getParameter(gl.VENDOR),
          renderer: ext
            ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)
            : gl.getParameter(gl.RENDERER),
          version: gl.getParameter(gl.VERSION),
        };
        gl.getExtension("WEBGL_lose_context")?.loseContext();
        return result;
      });
      const repetitions = [];
      for (let run = 0; run < repetitionCount; run++) {
        await page.mouse.move(720, 440);
        const sampling = page.evaluate(
          (milliseconds) =>
            new Promise((resolve) => {
              const times = [],
                drawFrames = [];
              let previousDraws = window.__glyphibeRenderProbe.drawCalls;
              const began = performance.now();
              function frame(timestamp) {
                times.push(timestamp);
                const current = window.__glyphibeRenderProbe.drawCalls;
                drawFrames.push(current > previousDraws);
                previousDraws = current;
                if (performance.now() - began < milliseconds)
                  requestAnimationFrame(frame);
                else resolve({ times, drawFrames, drawCalls: current });
              }
              requestAnimationFrame(frame);
            }),
          duration,
        );
        const until = Date.now() + duration;
        while (Date.now() < until) {
          await page.mouse.down();
          await page.mouse.move(850, 510, { steps: 10 });
          await page.mouse.move(720, 440, { steps: 10 });
          await page.mouse.up();
          await page.mouse.wheel(0, -100);
          await page.mouse.wheel(0, 100);
        }
        const result = await sampling,
          intervals = result.times
            .slice(1)
            .map((time, i) => time - result.times[i]),
          elapsed = result.times.at(-1) - result.times[0];
        repetitions.push({
          run: run + 1,
          raf_callbacks: result.times.length,
          elapsed_ms: round(elapsed),
          raf_cadence_hz: round((intervals.length * 1000) / elapsed),
          frame_interval_median_ms: round(quantile(intervals, 0.5)),
          frame_interval_p95_ms: round(quantile(intervals, 0.95)),
          callbacks_with_webgl_draws: result.drawFrames.filter(Boolean).length,
          callbacks_over_50ms: intervals.filter((value) => value > 50).length,
          webgl_draw_calls_total: result.drawCalls,
        });
      }
      const selection = await page.evaluate(async () => {
        const buttons = [
            ...document.querySelectorAll(".entity-list button"),
          ].slice(0, 15),
          times = [];
        for (const button of buttons) {
          const start = performance.now();
          button.click();
          await new Promise((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(resolve)),
          );
          times.push(performance.now() - start);
        }
        return times;
      });
      const level = await page.evaluate(async () => {
        const slider = document.querySelector(
            'input[aria-label="Architecture level"]',
          ),
          setter = Object.getOwnPropertyDescriptor(
            HTMLInputElement.prototype,
            "value",
          ).set,
          times = [];
        for (let i = 0; i < 10; i++) {
          const start = performance.now();
          setter.call(slider, i % 2 === 0 ? "4" : "5");
          slider.dispatchEvent(new Event("input", { bubbles: true }));
          slider.dispatchEvent(new Event("change", { bubbles: true }));
          await new Promise((resolve) =>
            requestAnimationFrame(() => requestAnimationFrame(resolve)),
          );
          times.push(performance.now() - start);
        }
        return times;
      });
      const session = await page.context().newCDPSession(page);
      await session.send("Performance.enable");
      const metrics = await session.send("Performance.getMetrics");
      const heap = metrics.metrics.find(
        (m) => m.name === "JSHeapUsedSize",
      )?.value;
      const sample = {
        graph_file: path.basename(graphFile),
        graph_sha256: createHash("sha256").update(bytes).digest("hex"),
        canonical_nodes: canonicalCount,
        canonical_edges: graph.edges.length,
        mode,
        rendered_nodes: visible,
        rendering_cap: effectiveCap,
        gpu,
        import_to_second_raf_ms: round(importToSecondRaf),
        repetitions,
        selection_to_second_raf_ms: {
          samples: selection.map(round),
          median: round(quantile(selection, 0.5)),
          p95: round(quantile(selection, 0.95)),
        },
        level_to_second_raf_ms: {
          samples: level.map(round),
          median: round(quantile(level, 0.5)),
          p95: round(quantile(level, 0.95)),
        },
        js_heap_after_sample_bytes: heap,
        page_errors: errors,
      };
      report.samples.push(sample);
      await page.close();
      await mkdir(path.dirname(output), { recursive: true });
      await writeFile(output, JSON.stringify(report, null, 2) + "\n");
      console.log(
        JSON.stringify({
          target,
          mode,
          rendered_nodes: visible,
          raf_cadence_hz: repetitions.map((r) => r.raf_cadence_hz),
          selection_p95_ms: sample.selection_to_second_raf_ms.p95,
          page_errors: errors.length,
        }),
      );
      if (errors.length)
        throw new Error(
          "UI errors occurred during rendering measurement. See report.",
        );
    }
  }
} finally {
  await browser.close();
  server?.kill();
}
