import { describe, expect, it } from "vitest";
import {
  escapeMarkup,
  chooseGalaxyLimit,
  indexGraph,
  clusterPositions,
  hierarchy,
  impactDetails,
  initialView,
  parseGraph,
  positions,
  project,
  reachable,
} from "./model";
import type { CodeGraph, CodeNode, CodeEdge } from "./model";
const evidence = {
  kind: "observed" as const,
  method: "manual test annotation",
  sources: [],
  note: "",
};
function n(id: string, level = 5): CodeNode {
  return {
    id,
    label: id,
    kind: level === 2 ? "cluster" : "function",
    level,
    plane: "architecture",
    repository: "test",
    revision: "working",
    summary: "test declaration",
    evidence,
    tags: [],
    knowledge: [],
    change: "unchanged",
    fingerprint: id,
  };
}
function e(
  source: string,
  target: string,
  stale = false,
  proposed = false,
): CodeEdge {
  return {
    id: `${source}-${target}`,
    source,
    target,
    relation: "calls",
    stale,
    evidence: { ...evidence, kind: proposed ? "proposed" : "observed" },
  };
}
function graph(): CodeGraph {
  return {
    schema_version: 1,
    repository: "test",
    revision: "working",
    nodes: { a: n("a"), b: n("b"), c: n("c"), d: n("d"), x: n("x", 2) },
    edges: [
      e("a", "b"),
      e("b", "c"),
      e("c", "a"),
      e("c", "d", false, true),
      e("a", "x", true),
    ],
    diagnostics: [],
    stats: {},
  };
}
describe("evidence-safe graph projections", () => {
  it("projects every shared level in all four modes and groups fine entities by canonical communities", () => {
    const g = graph();
    for (let level = 1; level <= 6; level++)
      g.nodes[`level-${level}`] = n(`level-${level}`, level);
    g.nodes["level-1"].kind = "domain";
    g.nodes["level-3"].kind = "component";
    g.nodes["level-4"].kind = "file";
    g.nodes["level-6"].kind = "statement";
    for (const [source, target] of [
      ["level-2", "level-3"],
      ["level-3", "level-4"],
      ["level-4", "level-5"],
      ["level-5", "level-6"],
    ])
      g.edges.push({ ...e(source, target), relation: "contains" });
    const index = indexGraph(g);
    for (const mode of ["Frontier", "Dense", "Cluster", "Galaxy"] as const)
      for (let level = 1; level <= 6; level++) {
        const visible = project(
          g,
          { ...initialView, mode, level },
          index,
        ).nodes;
        expect(visible.length).toBeGreaterThan(0);
        expect(visible.every((node) => node.level === level)).toBe(true);
      }
    expect(index.communities.get("level-6")).toContain("level-2");
    const grouped = clusterPositions(
      [g.nodes["level-5"], g.nodes.a],
      index.communities,
    );
    expect(grouped.size).toBe(2);
    expect(g.nodes["level-5"].level).toBe(5);
  });
  it("rejects malformed and escaping source spans before React renders", () => {
    const g = graph();
    const invalid = [
      { path: {} },
      {
        path: "../private.rs",
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 2,
        start_byte: 0,
        end_byte: 1,
      },
      {
        path: "valid.rs",
        start_line: 0,
        start_column: 1,
        end_line: 1,
        end_column: 2,
        start_byte: 0,
        end_byte: 1,
      },
      {
        path: "valid.rs",
        start_line: 1,
        start_column: 3,
        end_line: 1,
        end_column: 2,
        start_byte: 0,
        end_byte: 1,
      },
    ];
    for (const source of invalid) {
      g.nodes.a.source = source as CodeNode["source"];
      expect(() => parseGraph(g)).toThrow("Invalid graph node");
    }
  });
  it("rejects unknown enums and malformed evidence while accepting canonical graph data", () => {
    const g = graph();
    expect(parseGraph(g)).toBe(g);
    g.nodes.a.evidence = { ...evidence, kind: "invented" as "observed" };
    expect(() => parseGraph(g)).toThrow();
    g.nodes.a.evidence = evidence;
    g.nodes.a.kind = "invented";
    expect(() => parseGraph(g)).toThrow();
  });
  it("degrades only rendering for software and measured slow devices", () => {
    expect(chooseGalaxyLimit("ANGLE SwiftShader")).toBe(500);
    expect(chooseGalaxyLimit("Radeon", 12)).toBe(500);
    expect(chooseGalaxyLimit("Radeon", 20)).toBe(1000);
    expect(chooseGalaxyLimit("Radeon", 60)).toBe(2000);
    const g = graph();
    expect(
      project(g, { ...initialView, mode: "Galaxy", galaxyCap: 2 }).nodes,
    ).toHaveLength(2);
    expect(Object.keys(g.nodes)).toHaveLength(5);
  });
  it("escapes untrusted 3D tooltip markup", () => {
    expect(escapeMarkup('<img src=x onerror="alert(1)">')).toBe(
      "&lt;img src=x onerror=&quot;alert(1)&quot;&gt;",
    );
  });
  it("keeps layout coordinates stable when selection reorders nodes", () => {
    const g = graph();
    expect(positions([g.nodes.a, g.nodes.b])).toEqual(
      positions([g.nodes.b, g.nodes.a]),
    );
  });
  it("traverses many-to-many containment and reports impact paths", () => {
    const g = graph();
    g.nodes.x.kind = "component";
    g.edges.push({ ...e("x", "a"), relation: "contains" });
    expect(hierarchy(g, "a", true).map((n) => n.id)).toEqual(["x"]);
    const impact = impactDetails(g, "a", "downstream", 2);
    expect(impact.paths.c).toEqual(["a", "b", "c"]);
    expect(impact.components).toBe(1);
    expect(impact.functions).toBe(2);
  });
  it("crosses planes through an explicit view parameter", () => {
    const g = graph();
    g.nodes.c.plane = "explanation";
    expect(
      project(g, { ...initialView, plane: "explanation" }).nodes.map(
        (n) => n.id,
      ),
    ).toEqual(["c"]);
  });
  it("rejects malformed imports and prototype properties as edge endpoints", () => {
    expect(() => parseGraph({})).toThrow();
    const g = graph();
    g.edges.push(e("a", "__proto__"));
    expect(() => parseGraph(g)).toThrow();
  });
  it("rejects duplicate edge IDs before renderer construction", () => {
    const g = graph();
    g.edges.push({ ...g.edges[0] });
    expect(() => parseGraph(g)).toThrow();
  });
  it("rejects coerced non-string edge endpoints", () => {
    const g = graph();
    g.nodes["1"] = n("1");
    g.edges[0].source = 1 as unknown as string;
    expect(() => parseGraph(g)).toThrow("Invalid edge");
  });
  it("bounds cyclic reachability and excludes proposed and stale relations", () => {
    const g = graph();
    expect([...reachable(g, "a", "downstream", 1).keys()]).toEqual(["a", "b"]);
    expect([...reachable(g, "a", "downstream", 10).keys()]).toEqual([
      "a",
      "b",
      "c",
    ]);
    expect([...reachable(g, "a", "upstream", 1).keys()]).toEqual(["a", "c"]);
  });
  it("keeps six-level projection and teaching depth independent", () => {
    const g = graph();
    expect(
      project(g, { ...initialView, level: 2 }).nodes.map((n) => n.id),
    ).toEqual(["x"]);
    expect(project(g, { ...initialView, depth: 4 }).nodes).toEqual(
      project(g, { ...initialView, depth: 1 }).nodes,
    );
  });
  it("caps rendering without deleting canonical graph data", () => {
    const g = graph();
    expect(project(g, { ...initialView, density: 2 }).nodes).toHaveLength(2);
    expect(Object.keys(g.nodes)).toHaveLength(5);
  });
  it("removes historical nodes from active projections", () => {
    const g = graph();
    g.nodes.a.change = "removed";
    expect(project(g, initialView).nodes.some((n) => n.id === "a")).toBe(false);
  });
});
