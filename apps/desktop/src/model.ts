export type Mode = "Frontier" | "Dense" | "Cluster" | "Galaxy";
export type Direction = "upstream" | "downstream";
export interface Span {
  path: string;
  start_line: number;
  end_line: number;
  start_column: number;
  end_column: number;
  start_byte: number;
  end_byte: number;
}
export interface Evidence {
  kind: "observed" | "inferred" | "proposed" | "verified";
  method: string;
  sources: Span[];
  note: string;
}
export interface CodeNode {
  id: string;
  label: string;
  kind: string;
  level: number;
  plane: string;
  repository: string;
  revision: string;
  source?: Span | null;
  summary: string;
  evidence: Evidence;
  tags: string[];
  knowledge: string[];
  change: string;
  language?: string | null;
  fingerprint: string;
}
export interface CodeEdge {
  id: string;
  source: string;
  target: string;
  relation: string;
  stale: boolean;
  evidence: Evidence;
}
export interface CodeGraph {
  schema_version: number;
  repository: string;
  revision: string;
  nodes: Record<string, CodeNode>;
  edges: CodeEdge[];
  diagnostics: { code: string; message: string; path?: string }[];
  stats: Record<string, number>;
}
export interface GraphIndex {
  children: Map<string, string[]>;
  parents: Map<string, string[]>;
  communities: Map<string, string[]>;
  edges: Map<string, CodeEdge>;
}
export function sameSpan(
  a: Span | null | undefined,
  b: Span | null | undefined,
): boolean {
  if (a == null || b == null) return a == null && b == null;
  return (
    a.path === b.path &&
    a.start_line === b.start_line &&
    a.start_column === b.start_column &&
    a.end_line === b.end_line &&
    a.end_column === b.end_column &&
    a.start_byte === b.start_byte &&
    a.end_byte === b.end_byte
  );
}
/** One adjacency/index per loaded immutable graph, shared by projections and layout. */
export function indexGraph(graph: CodeGraph): GraphIndex {
  const children = new Map<string, string[]>(),
    parents = new Map<string, string[]>(),
    edges = new Map<string, CodeEdge>();
  for (const edge of graph.edges) {
    edges.set(edge.id, edge);
    if (
      edge.stale ||
      edge.evidence.kind === "proposed" ||
      edge.relation !== "contains" ||
      graph.nodes[edge.source]?.change === "removed" ||
      graph.nodes[edge.target]?.change === "removed"
    )
      continue;
    const down = children.get(edge.source) ?? [];
    down.push(edge.target);
    children.set(edge.source, down);
    const up = parents.get(edge.target) ?? [];
    up.push(edge.source);
    parents.set(edge.target, up);
  }
  for (const adjacency of [children, parents])
    for (const [id, list] of adjacency)
      adjacency.set(id, [...new Set(list)].sort());
  const distances = new Map<string, number>(),
    membership = new Map<string, Set<string>>(),
    queue: string[] = [];
  for (const node of Object.values(graph.nodes)
    .filter(
      (n) =>
        n.kind === "cluster" &&
        n.plane === "architecture" &&
        n.change !== "removed" &&
        n.evidence.kind !== "proposed",
    )
    .sort((a, b) => a.id.localeCompare(b.id))) {
    distances.set(node.id, 0);
    membership.set(node.id, new Set([node.id]));
    queue.push(node.id);
  }
  // Multi-source shortest containment distance retains every equally-near
  // canonical community. Positive distances make cycles terminate naturally.
  for (let i = 0; i < queue.length; i++) {
    const id = queue[i],
      distance = distances.get(id)!;
    for (const child of children.get(id) ?? []) {
      const next = distance + 1,
        known = distances.get(child);
      if (known !== undefined && known < next) continue;
      let changed = false;
      if (known === undefined || next < known) {
        distances.set(child, next);
        membership.set(child, new Set(membership.get(id)));
        changed = true;
      } else {
        const groups = membership.get(child)!;
        for (const group of membership.get(id) ?? [])
          if (!groups.has(group)) {
            groups.add(group);
            changed = true;
          }
      }
      if (changed) queue.push(child);
    }
  }
  const mappedArchitecture = new Map<string, string[]>();
  for (const edge of graph.edges) {
    if (
      edge.stale ||
      edge.evidence.kind === "proposed" ||
      edge.relation !== "maps_to"
    )
      continue;
    const a = graph.nodes[edge.source],
      b = graph.nodes[edge.target];
    if (
      !a ||
      !b ||
      a.change === "removed" ||
      b.change === "removed" ||
      a.plane === b.plane
    )
      continue;
    const [architecture, explanation] =
      a.plane === "architecture" ? [a, b] : [b, a];
    const ids = mappedArchitecture.get(explanation.id) ?? [];
    ids.push(architecture.id);
    mappedArchitecture.set(explanation.id, ids);
  }
  for (const [id, candidates] of mappedArchitecture) {
    const explanation = graph.nodes[id];
    const sameLevel = candidates.filter(
      (candidate) => graph.nodes[candidate].level === explanation.level,
    );
    const exact = sameLevel.filter((candidate) =>
      sameSpan(graph.nodes[candidate].source, explanation.source),
    );
    const chosen = exact.length
      ? exact
      : sameLevel.length
        ? sameLevel
        : candidates;
    const groups = new Set(
      chosen.flatMap((candidate) => [...(membership.get(candidate) ?? [])]),
    );
    if (groups.size) membership.set(id, groups);
  }
  return {
    children,
    parents,
    communities: new Map(
      [...membership].map(([id, groups]) => [id, [...groups].sort()]),
    ),
    edges,
  };
}
export interface ViewState {
  mode: Mode;
  selected: string | null;
  level: number;
  density: number;
  depth: number;
  direction: Direction;
  hops: number;
  locale: "en" | "zh";
  theme: "dark" | "light";
  focus: boolean;
  opaque: boolean;
  pinned: boolean;
  plane: "architecture" | "explanation";
  galaxyCap: number;
  graphicsPolicy: "auto" | "full";
}
export const initialView: ViewState = {
  mode: "Dense",
  selected: null,
  level: 5,
  density: 1200,
  depth: 2,
  direction: "upstream",
  hops: 2,
  locale: "en",
  theme: "dark",
  focus: false,
  opaque: false,
  pinned: true,
  plane: "architecture",
  galaxyCap: 2000,
  graphicsPolicy: "auto",
};
export function chooseGalaxyLimit(renderer: string, cadence?: number): number {
  if (/swiftshader|llvmpipe|softpipe|software/i.test(renderer)) return 500;
  if (cadence !== undefined && Number.isFinite(cadence)) {
    if (cadence < 15) return 500;
    if (cadence < 24) return 1000;
  }
  return 2000;
}
export function escapeMarkup(value: string): string {
  return value.replace(
    /[&<>"']/g,
    (char) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        char
      ]!,
  );
}
const RELATIONS = new Set([
  "calls",
  "imports",
  "references",
  "depends_on",
  "implements",
]);
const NODE_KINDS = new Set([
  "domain",
  "cluster",
  "component",
  "file",
  "class",
  "function",
  "block",
  "statement",
  "expression",
  "concept",
  "explanation",
]);
const EVIDENCE_KINDS = new Set([
  "observed",
  "inferred",
  "proposed",
  "verified",
]);
const CHANGES = new Set([
  "unchanged",
  "added",
  "modified",
  "removed",
  "ambiguous",
]);
const ALL_RELATIONS = new Set([
  ...RELATIONS,
  "contains",
  "maps_to",
  "has_concept",
  "verified_by",
]);
const boundedText = (value: unknown, max = 32768): value is string =>
  typeof value === "string" && value.length <= max;
function validSpan(value: unknown): value is Span {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const span = value as Span;
  if (
    !boundedText(span.path) ||
    !span.path ||
    span.path.includes("\\") ||
    span.path.includes(":") ||
    span.path.split("/").some((part) => !part || part === "." || part === "..")
  )
    return false;
  const positive = [
    span.start_line,
    span.start_column,
    span.end_line,
    span.end_column,
  ];
  return (
    positive.every((number) => Number.isSafeInteger(number) && number >= 1) &&
    Number.isSafeInteger(span.start_byte) &&
    span.start_byte >= 0 &&
    Number.isSafeInteger(span.end_byte) &&
    span.end_byte >= span.start_byte &&
    span.end_line >= span.start_line &&
    (span.end_line !== span.start_line || span.end_column >= span.start_column)
  );
}
function validEvidence(value: unknown): value is Evidence {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const evidence = value as Evidence;
  return (
    EVIDENCE_KINDS.has(evidence.kind) &&
    boundedText(evidence.method, 4096) &&
    boundedText(evidence.note, 1024 * 1024) &&
    Array.isArray(evidence.sources) &&
    evidence.sources.every(validSpan)
  );
}

export function parseGraph(value: unknown): CodeGraph {
  if (!value || typeof value !== "object")
    throw new Error("Expected a graph object.");
  const g = value as CodeGraph;
  if (
    g.schema_version !== 1 ||
    !g.nodes ||
    typeof g.nodes !== "object" ||
    Array.isArray(g.nodes) ||
    !Array.isArray(g.edges) ||
    !Array.isArray(g.diagnostics) ||
    !g.stats ||
    typeof g.stats !== "object" ||
    Array.isArray(g.stats) ||
    !Object.values(g.stats).every((n) => Number.isSafeInteger(n) && n >= 0) ||
    !boundedText(g.repository) ||
    !boundedText(g.revision, 4096)
  )
    throw new Error("Unsupported or malformed Glyphibe graph.");
  if (Object.keys(g.nodes).length > 200000 || g.edges.length > 1000000)
    throw new Error("Graph exceeds the import safety limit.");
  for (const [id, n] of Object.entries(g.nodes))
    if (
      !n ||
      n.id !== id ||
      !boundedText(id, 4096) ||
      !id ||
      ["__proto__", "constructor", "prototype"].includes(id) ||
      !boundedText(n.label) ||
      !NODE_KINDS.has(n.kind) ||
      !["architecture", "explanation"].includes(n.plane) ||
      !CHANGES.has(n.change) ||
      !boundedText(n.repository) ||
      !boundedText(n.revision, 4096) ||
      !boundedText(n.fingerprint, 4096) ||
      !(
        n.language === null ||
        n.language === undefined ||
        boundedText(n.language, 256)
      ) ||
      !(n.source === null || n.source === undefined || validSpan(n.source)) ||
      !boundedText(n.summary, 1024 * 1024) ||
      !Number.isInteger(n.level) ||
      n.level < 1 ||
      n.level > 6 ||
      !validEvidence(n.evidence) ||
      !Array.isArray(n.knowledge) ||
      !n.knowledge.every((k) => boundedText(k, 4096)) ||
      !Array.isArray(n.tags) ||
      !n.tags.every((tag) => boundedText(tag, 32768))
    )
      throw new Error("Invalid graph node.");
  const edgeIds = new Set<string>();
  for (const e of g.edges) {
    if (
      !e ||
      !boundedText(e.source, 4096) ||
      !boundedText(e.target, 4096) ||
      !Object.hasOwn(g.nodes, e.source) ||
      !Object.hasOwn(g.nodes, e.target) ||
      !ALL_RELATIONS.has(e.relation) ||
      !boundedText(e.id, 4096) ||
      !e.id ||
      typeof e.stale !== "boolean" ||
      !validEvidence(e.evidence) ||
      edgeIds.has(e.id)
    )
      throw new Error("Invalid edge or missing graph node.");
    edgeIds.add(e.id);
  }
  for (const diagnostic of g.diagnostics)
    if (
      !diagnostic ||
      !boundedText(diagnostic.code, 4096) ||
      !boundedText(diagnostic.message, 1024 * 1024) ||
      !(
        diagnostic.path === null ||
        diagnostic.path === undefined ||
        boundedText(diagnostic.path)
      )
    )
      throw new Error("Invalid graph diagnostic.");
  return g;
}

export function reachable(
  graph: CodeGraph,
  origin: string,
  direction: Direction,
  hops: number,
  dependencyOnly = true,
): Map<string, number> {
  const distance = new Map<string, number>([[origin, 0]]);
  const adjacency = new Map<string, string[]>();
  for (const e of graph.edges) {
    if (
      e.stale ||
      e.evidence.kind === "proposed" ||
      graph.nodes[e.source]?.change === "removed" ||
      graph.nodes[e.target]?.change === "removed" ||
      (dependencyOnly && !RELATIONS.has(e.relation))
    )
      continue;
    const [a, b] =
      direction === "downstream" ? [e.source, e.target] : [e.target, e.source];
    const entries = adjacency.get(a) ?? [];
    entries.push(b);
    adjacency.set(a, entries);
  }
  const queue = [origin];
  for (let i = 0; i < queue.length; i++) {
    const node = queue[i],
      depth = distance.get(node)!;
    if (depth >= hops) continue;
    for (const next of adjacency.get(node) ?? [])
      if (!distance.has(next)) {
        distance.set(next, depth + 1);
        queue.push(next);
      }
  }
  return distance;
}

export function project(
  graph: CodeGraph,
  state: ViewState,
  index: GraphIndex = indexGraph(graph),
): { nodes: CodeNode[]; edges: CodeEdge[]; total: number } {
  let nodes = Object.values(graph.nodes).filter(
    (n) =>
      n.change !== "removed" &&
      n.plane === state.plane &&
      n.level === state.level,
  );
  if (state.mode === "Frontier" && state.selected) {
    const a = reachable(graph, state.selected, "upstream", 2, false),
      b = reachable(graph, state.selected, "downstream", 2, false);
    const structural = new Set([
      state.selected,
      ...hierarchy(graph, state.selected, true, index).map((n) => n.id),
      ...hierarchy(graph, state.selected, false, index).map((n) => n.id),
    ]);
    nodes = nodes.filter(
      (n) => a.has(n.id) || b.has(n.id) || structural.has(n.id),
    );
  }
  const total = nodes.length;
  nodes.sort((a, b) =>
    a.id === state.selected
      ? -1
      : b.id === state.selected
        ? 1
        : a.label.localeCompare(b.label),
  );
  nodes = nodes.slice(
    0,
    Math.min(state.density, state.mode === "Galaxy" ? state.galaxyCap : 10000),
  );
  const ids = new Set(nodes.map((n) => n.id));
  let edges = graph.edges.filter(
    (e) => !e.stale && ids.has(e.source) && ids.has(e.target),
  );
  if (
    state.mode === "Cluster" &&
    state.plane === "architecture" &&
    state.level <= 4
  ) {
    // These display-only edges aggregate canonical dependency evidence; no graph facts are added.
    const owners = new Map<string, Set<string>>();
    if (state.level === 2) {
      for (const [member, groups] of index.communities) {
        const visible = new Set(groups.filter((id) => ids.has(id)));
        if (visible.size) owners.set(member, visible);
      }
    } else {
      const queue: [string, string][] = [];
      for (const node of nodes) {
        owners.set(node.id, new Set([node.id]));
        queue.push([node.id, node.id]);
      }
      for (let i = 0; i < queue.length; i++) {
        const [member, owner] = queue[i];
        for (const child of index.children.get(member) ?? []) {
          const set = owners.get(child) ?? new Set<string>();
          if (!set.has(owner)) {
            set.add(owner);
            owners.set(child, set);
            queue.push([child, owner]);
          }
        }
      }
    }
    const aggregated = new Map<string, CodeEdge>();
    for (const edge of graph.edges) {
      if (
        edge.stale ||
        edge.evidence.kind === "proposed" ||
        !RELATIONS.has(edge.relation)
      )
        continue;
      for (const source of owners.get(edge.source) ?? [])
        for (const target of owners.get(edge.target) ?? []) {
          if (source === target) continue;
          const id = `projection:${source}:${target}`;
          if (!aggregated.has(id))
            aggregated.set(id, {
              ...edge,
              id,
              source,
              target,
              relation: "depends_on",
              evidence: {
                ...edge.evidence,
                kind: "inferred",
                method: "display aggregation of canonical dependency evidence",
              },
            });
        }
    }
    edges = [...edges, ...aggregated.values()];
  }
  return { nodes, edges, total };
}
export function related(
  graph: CodeGraph,
  id: string,
): { edge: CodeEdge; node: CodeNode }[] {
  return graph.edges
    .filter((e) => e.source === id || e.target === id)
    .map((edge) => ({
      edge,
      node: graph.nodes[edge.source === id ? edge.target : edge.source],
    }));
}
export function colour(node: CodeNode, theme = "dark"): string {
  const colours: Record<string, string> = {
    domain: "#c19bd9",
    cluster: "#d6b36d",
    component: "#8fbb91",
    file: "#77b2d2",
    class: "#a698d4",
    function: "#78bec2",
    block: "#c68f9e",
    statement: "#aab7c7",
    expression: "#bdc68f",
  };
  return colours[node.kind] ?? (theme === "light" ? "#456478" : "#9aafc3");
}
export function positions(
  nodes: CodeNode[],
): Map<string, { x: number; y: number; z: number }> {
  return new Map(
    [...nodes]
      .sort((a, b) => a.id.localeCompare(b.id))
      .map((n, i) => {
        const angle = i * 2.399963229728653,
          r = Math.sqrt(i + 1) * 12;
        return [
          n.id,
          {
            x: Math.cos(angle) * r,
            y: Math.sin(angle) * r,
            z: Math.sin(i * 1.7) * r * 0.6,
          },
        ];
      }),
  );
}
/** Frame the owned fixed layout without depending on asynchronous scene bounds. */
export function fitGalaxyCamera(
  points: Iterable<{ x: number; y: number; z: number }>,
  aspect: number,
  fovDegrees: number,
  nodeRadius = 5,
) {
  const values = [...points];
  if (
    !values.length ||
    !Number.isFinite(aspect) ||
    aspect <= 0 ||
    !Number.isFinite(fovDegrees) ||
    fovDegrees <= 0 ||
    fovDegrees >= 179 ||
    !Number.isFinite(nodeRadius) ||
    nodeRadius <= 0
  )
    return null;
  let minX = Infinity,
    minY = Infinity,
    minZ = Infinity;
  let maxX = -Infinity,
    maxY = -Infinity,
    maxZ = -Infinity;
  for (const point of values) {
    if (![point.x, point.y, point.z].every(Number.isFinite)) return null;
    minX = Math.min(minX, point.x);
    maxX = Math.max(maxX, point.x);
    minY = Math.min(minY, point.y);
    maxY = Math.max(maxY, point.y);
    minZ = Math.min(minZ, point.z);
    maxZ = Math.max(maxZ, point.z);
  }
  const target = {
    x: (minX + maxX) / 2,
    y: (minY + maxY) / 2,
    z: (minZ + maxZ) / 2,
  };
  let radius = nodeRadius;
  for (const point of values)
    radius = Math.max(
      radius,
      Math.hypot(point.x - target.x, point.y - target.y, point.z - target.z) +
        nodeRadius,
    );
  const vertical = (fovDegrees * Math.PI) / 360;
  const horizontal = Math.atan(Math.tan(vertical) * aspect);
  const distance = Math.max(
    nodeRadius * 3,
    (radius / Math.sin(Math.min(vertical, horizontal))) * 1.15,
  );
  return {
    position: { x: target.x, y: target.y, z: target.z + distance },
    target,
  };
}

/** Display placement only. Multiple memberships remain in GraphIndex; the
 * lexicographically first equally-near community is a stable visual anchor. */
export function clusterPositions(
  nodes: CodeNode[],
  communities: Map<string, string[]>,
): Map<string, { x: number; y: number; z: number }> {
  const groups = new Map<string, CodeNode[]>();
  for (const node of nodes) {
    const group = communities.get(node.id)?.[0] ?? "unassigned";
    const members = groups.get(group) ?? [];
    members.push(node);
    groups.set(group, members);
  }
  const ordered = [...groups].sort(([a], [b]) => a.localeCompare(b)),
    step = Math.max(
      90,
      ...ordered.map(([, members]) => Math.sqrt(members.length) * 28),
    );
  const result = new Map<string, { x: number; y: number; z: number }>();
  for (let groupIndex = 0; groupIndex < ordered.length; groupIndex++) {
    const [, members] = ordered[groupIndex],
      angle = groupIndex * 2.399963229728653,
      radius = Math.sqrt(groupIndex) * step,
      cx = Math.cos(angle) * radius,
      cy = Math.sin(angle) * radius;
    members
      .sort((a, b) => a.id.localeCompare(b.id))
      .forEach((node, index) => {
        const a = index * 2.399963229728653,
          r = Math.sqrt(index) * 11;
        result.set(node.id, {
          x: cx + Math.cos(a) * r,
          y: cy + Math.sin(a) * r,
          z: 0,
        });
      });
  }
  return result;
}
export function hierarchy(
  graph: CodeGraph,
  id: string,
  upward: boolean,
  index: GraphIndex = indexGraph(graph),
): CodeNode[] {
  const adjacency = upward ? index.parents : index.children;
  const seen = new Set([id]),
    queue = [id];
  for (let i = 0; i < queue.length && i < 50000; i++)
    for (const target of adjacency.get(queue[i]) ?? [])
      if (!seen.has(target)) {
        seen.add(target);
        queue.push(target);
      }
  return queue
    .slice(1)
    .map((id) => graph.nodes[id])
    .filter((n) => Boolean(n) && n.change !== "removed");
}
export function impactDetails(
  graph: CodeGraph,
  origin: string,
  direction: Direction,
  hops: number,
) {
  const distances = reachable(graph, origin, direction, hops),
    adjacency = new Map<string, string[]>(),
    parents = new Map<string, string[]>();
  for (const e of graph.edges) {
    if (e.stale || e.evidence.kind === "proposed") continue;
    if (e.relation === "contains") {
      const entries = parents.get(e.target) ?? [];
      entries.push(e.source);
      parents.set(e.target, entries);
    }
    if (!RELATIONS.has(e.relation)) continue;
    const [a, b] =
      direction === "downstream" ? [e.source, e.target] : [e.target, e.source];
    const entries = adjacency.get(a) ?? [];
    entries.push(b);
    adjacency.set(a, entries);
  }
  const paths: Record<string, string[]> = { [origin]: [origin] },
    queue = [origin];
  for (let i = 0; i < queue.length; i++) {
    const current = queue[i];
    if ((distances.get(current) ?? 0) >= hops) continue;
    for (const next of adjacency.get(current) ?? [])
      if (distances.has(next) && !Object.hasOwn(paths, next)) {
        paths[next] = [...paths[current], next];
        queue.push(next);
      }
  }
  const nodes = [...distances.keys()]
    .filter((id) => id !== origin)
    .map((id) => graph.nodes[id]);
  const ancestors = new Set(distances.keys()),
    ancestorsQueue = [...ancestors];
  for (let i = 0; i < ancestorsQueue.length; i++)
    for (const parent of parents.get(ancestorsQueue[i]) ?? [])
      if (!ancestors.has(parent)) {
        ancestors.add(parent);
        ancestorsQueue.push(parent);
      }
  return {
    paths,
    components: [...ancestors].filter(
      (id) => graph.nodes[id].kind === "component",
    ).length,
    functions: nodes.filter((n) => n.kind === "function").length,
    tests: nodes.filter((n) => n.tags.includes("test")).length,
    interfaces: nodes.filter(
      (n) => n.tags.includes("interface") || n.tags.includes("export"),
    ).length,
  };
}
