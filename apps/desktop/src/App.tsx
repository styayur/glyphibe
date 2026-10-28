import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import GraphCanvas from "./GraphCanvas";
import {
  hierarchy,
  indexGraph,
  chooseGalaxyLimit,
  impactDetails,
  initialView,
  parseGraph,
  project,
  reachable,
  related,
} from "./model";
import type { CodeGraph, ViewState, Mode } from "./model";
import registry from "../../../knowledge/registry.json";

type Explanation = {
  title: string;
  facts: string[];
  uncertainty: string[];
  concepts: string[];
  steps: { question: string; hint: string; answer: string; review: string }[];
};
type NativeImpact = {
  scope: {
    architecture_origins: string[];
    dependency_seeds: string[];
    structural_nodes: string[];
    basis: {
      edge_id: string;
      source: string;
      target: string;
      relation: string;
      evidence: string;
      method: string;
      step: string;
    }[];
    truncated: boolean;
  };
  reachability: {
    nodes: string[];
    paths: Record<string, string[]>;
    truncated: boolean;
  };
  components: number;
  functions: number;
  tests: number;
  interfaces: number;
  limitation: string;
};
const modes: Mode[] = ["Frontier", "Dense", "Cluster", "Galaxy"];
const levels = [
  "Domain",
  "Cluster",
  "Component",
  "File",
  "Function / block",
  "Statement / expression",
];
const depths = ["Absolute beginner", "Beginner", "Fluent", "Professional"];
const zhLevels = [
  "领域",
  "集群",
  "组件",
  "文件",
  "函数 / 代码块",
  "语句 / 表达式",
];
function Island({
  title,
  className,
  children,
}: {
  title: string;
  className: string;
  children: ReactNode;
}) {
  const [pinned, setPinned] = useState(true),
    [collapsed, setCollapsed] = useState(false);
  return (
    <section
      className={`${className} glass ${collapsed ? "island-collapsed" : ""}`}
      aria-label={title}
      onMouseEnter={() => {
        if (!pinned) setCollapsed(false);
      }}
      onMouseLeave={() => {
        if (!pinned) setCollapsed(true);
      }}
    >
      <div className="panel-heading">
        <strong>{title}</strong>
        <button
          aria-label={`Pin ${title}`}
          aria-pressed={pinned}
          onClick={() => setPinned(!pinned)}
        >
          ⌖
        </button>
        <button
          aria-label={`Collapse ${title}`}
          aria-expanded={!collapsed}
          onClick={() => setCollapsed(!collapsed)}
        >
          {collapsed ? "+" : "−"}
        </button>
      </div>
      <div className="island-content" hidden={collapsed}>
        {children}
      </div>
    </section>
  );
}

export default function App() {
  const [graph, setGraph] = useState<CodeGraph | null>(null);
  const [nativeGraph, setNativeGraph] = useState(false);
  const [view, setView] = useState<ViewState>(initialView);
  const [automaticGalaxyCap, setAutomaticGalaxyCap] = useState(2000);
  const [graphicsReason, setGraphicsReason] = useState("");
  const rendererDescription = useRef("");
  const effectiveView = useMemo(
    () => ({
      ...view,
      galaxyCap: view.graphicsPolicy === "full" ? 2000 : automaticGalaxyCap,
    }),
    [view, automaticGalaxyCap],
  );
  const reportCadence = useCallback((cadence: number) => {
    const next = chooseGalaxyLimit(rendererDescription.current, cadence);
    setAutomaticGalaxyCap((current) => Math.min(current, next));
    if (
      next < 2000 &&
      !/swiftshader|llvmpipe|softpipe|software/i.test(
        rendererDescription.current,
      )
    )
      setGraphicsReason(`Observed animation cadence ${Math.round(cadence)} Hz`);
  }, []);
  useEffect(() => {
    try {
      const canvas = document.createElement("canvas"),
        gl = canvas.getContext("webgl2") ?? canvas.getContext("webgl");
      if (!gl) return;
      const ext = gl.getExtension("WEBGL_debug_renderer_info");
      const description = String(
        ext
          ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)
          : gl.getParameter(gl.RENDERER),
      );
      rendererDescription.current = description;
      const limit = chooseGalaxyLimit(description);
      setAutomaticGalaxyCap(limit);
      if (limit < 2000) setGraphicsReason("Software rendering");
      gl.getExtension("WEBGL_lose_context")?.loseContext();
    } catch {
      /* A renderer error is presented by the graph canvas, with list navigation retained. */
    }
  }, []);
  const [search, setSearch] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState("");
  const [collapsed, setCollapsed] = useState(false);
  const [tab, setTab] = useState<"Evidence" | "Source" | "Learn" | "Knowledge">(
    "Evidence",
  );
  const [source, setSource] = useState("");
  const [explanation, setExplanation] = useState<Explanation | null>(null);
  const [nativeImpact, setNativeImpact] = useState<{
    key: string;
    result: NativeImpact;
  } | null>(null);
  const [step, setStep] = useState(0);
  const [revealed, setRevealed] = useState<"question" | "hint" | "answer">(
    "question",
  );
  const input = useRef<HTMLInputElement>(null);
  const startup = useRef<Promise<CodeGraph | null> | null>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const text = useCallback(
    (en: string, zh: string) => (view.locale === "zh" ? zh : en),
    [view.locale],
  );
  const update = useCallback(
    <K extends keyof ViewState>(key: K, value: ViewState[K]) =>
      setView((s) => ({ ...s, [key]: value })),
    [],
  );
  const select = useCallback(
    (id: string) => {
      const n = graph?.nodes[id];
      setView((s) => ({
        ...s,
        selected: id,
        level: n?.level ?? s.level,
        plane: n?.plane === "explanation" ? "explanation" : "architecture",
      }));
      setStep(0);
      setRevealed("question");
    },
    [graph],
  );
  const load = useCallback((g: CodeGraph, native = false) => {
    setGraph(g);
    setNativeGraph(native);
    setView((s) => ({ ...s, selected: null }));
    setError("");
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    if (!startup.current) {
      setBusy(true);
      setProgress("Opening selected repository…");
      startup.current = invoke<CodeGraph | null>("startup_repository");
    }
    let active = true;
    void startup.current
      .then((result) => {
        if (active && result) load(parseGraph(result), true);
      })
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) {
          setBusy(false);
          setProgress("");
        }
      });
    return () => {
      active = false;
    };
  }, [load]);
  const open = useCallback(async () => {
    if (!isTauri()) {
      input.current?.click();
      return;
    }
    setBusy(true);
    setError("");
    setProgress(text("Choose a repository folder…", "选择代码仓库目录…"));
    try {
      const result = await invoke<CodeGraph | null>("open_repository");
      if (result) load(parseGraph(result), true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setProgress("");
    }
  }, [load, text]);
  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<{
      phase: string;
      completed: number;
      total: number;
    }>("scan-progress", (e) =>
      setProgress(
        `${e.payload.phase}: ${e.payload.completed} / ${e.payload.total}`,
      ),
    );
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") update("focus", false);
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        searchInput.current?.focus();
      }
      if (
        e.key.toLowerCase() === "f" &&
        !(
          e.target instanceof HTMLInputElement ||
          e.target instanceof HTMLTextAreaElement
        )
      ) {
        e.preventDefault();
        setView((s) => ({ ...s, focus: !s.focus }));
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "o") {
        e.preventDefault();
        void open();
      }
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  }, [open, update]);
  useEffect(() => {
    document.documentElement.dataset.theme = view.theme;
    document.documentElement.lang = view.locale === "zh" ? "zh-CN" : "en";
  }, [view.theme, view.locale]);
  const topology = useMemo(() => (graph ? indexGraph(graph) : null), [graph]);
  const projection = useMemo(
    () =>
      graph
        ? project(graph, effectiveView, topology!)
        : { nodes: [], edges: [], total: 0 },
    [
      graph,
      view.mode,
      view.selected,
      view.level,
      view.density,
      view.plane,
      effectiveView.galaxyCap,
      topology,
    ],
  );
  const impactKey = `${view.selected}:${view.direction}:${view.hops}`;
  const currentNativeImpact =
    nativeImpact?.key === impactKey ? nativeImpact.result : null;
  useEffect(() => {
    if (!nativeGraph || !view.selected) return;
    let active = true;
    void invoke<NativeImpact>("query_impact", {
      id: view.selected,
      direction: view.direction,
      hops: view.hops,
    })
      .then((result) => {
        if (active) setNativeImpact({ key: impactKey, result });
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [nativeGraph, view.selected, view.direction, view.hops, impactKey]);
  const impact = useMemo(
    () =>
      nativeGraph
        ? new Set(
            view.selected
              ? [
                  view.selected,
                  ...(currentNativeImpact?.reachability.nodes ?? []),
                ]
              : [],
          )
        : graph && view.selected
          ? new Set(
              reachable(graph, view.selected, view.direction, view.hops).keys(),
            )
          : new Set<string>(),
    [
      graph,
      view.selected,
      view.direction,
      view.hops,
      nativeGraph,
      currentNativeImpact,
    ],
  );
  const node = graph && view.selected ? graph.nodes[view.selected] : null;
  const relationships = useMemo(
    () => (graph && view.selected ? related(graph, view.selected) : []),
    [graph, view.selected],
  );
  const ancestors = useMemo(
    () =>
      graph && view.selected
        ? hierarchy(graph, view.selected, true, topology!)
        : [],
    [graph, view.selected, topology],
  );
  const descendants = useMemo(
    () =>
      graph && view.selected
        ? hierarchy(graph, view.selected, false, topology!)
        : [],
    [graph, view.selected, topology],
  );
  const impactInfo = useMemo(
    () =>
      nativeGraph
        ? currentNativeImpact
          ? {
              ...currentNativeImpact,
              paths: currentNativeImpact.reachability.paths,
            }
          : null
        : graph && view.selected
          ? impactDetails(graph, view.selected, view.direction, view.hops)
          : null,
    [
      graph,
      view.selected,
      view.direction,
      view.hops,
      nativeGraph,
      currentNativeImpact,
    ],
  );
  const results = useMemo(
    () =>
      graph
        ? Object.values(graph.nodes)
            .filter(
              (n) =>
                n.plane === view.plane &&
                n.change !== "removed" &&
                n.label
                  .toLocaleLowerCase()
                  .includes(search.toLocaleLowerCase()),
            )
            .slice(0, 80)
        : [],
    [graph, search, view.plane],
  );
  useEffect(() => {
    setSource("");
    setExplanation(null);
    if (!node || !isTauri() || !nativeGraph) return;
    let active = true;
    void invoke<Explanation>("explain_node", {
      id: node.id,
      depth: view.depth,
      language: view.locale,
    })
      .then((v) => {
        if (active) setExplanation(v);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    if (node.source)
      void invoke<string>("read_source", { id: node.id })
        .then((v) => {
          if (active) setSource(v);
        })
        .catch((e) => {
          if (active) setSource(String(e));
        });
    return () => {
      active = false;
    };
  }, [node, view.depth, view.locale, nativeGraph]);
  const lessons = explanation?.steps ?? [];
  const lesson = lessons[step];
  const concepts = useMemo(() => {
    const ids = new Set([
      ...(node?.knowledge ?? []),
      ...relationships
        .filter((r) => r.edge.relation === "has_concept")
        .flatMap((r) => r.node.knowledge),
    ]);
    return registry.concepts.filter((c) => ids.has(c.id));
  }, [node, relationships]);
  async function importFile(file: File | undefined) {
    if (!file) return;
    if (file.size > 64 * 1024 * 1024) {
      setError("Graph file exceeds 64 MiB.");
      return;
    }
    try {
      load(parseGraph(JSON.parse(await file.text())));
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <main
      className={`${view.opaque ? "opaque " : ""}${view.focus ? "focus " : ""}`}
    >
      <input
        ref={input}
        type="file"
        accept=".json,application/json"
        aria-label="Import graph JSON"
        className="file-input"
        onChange={(e) => void importFile(e.target.files?.[0])}
      />
      {graph && (
        <GraphCanvas
          nodes={projection.nodes}
          edges={projection.edges}
          state={effectiveView}
          impact={impact}
          onSelect={select}
          onCadence={reportCadence}
          communities={topology!.communities}
        />
      )}
      <header className="titlebar glass" data-tauri-drag-region>
        <strong className="wordmark" data-tauri-drag-region>
          ◈ Glyphibe
        </strong>
        <span className="repository" data-tauri-drag-region>
          {graph
            ? text("Repository loaded", "仓库已加载")
            : text("Read code through its connections", "从代码关系理解结构")}
        </span>
        <span className="title-actions">
          <button onClick={() => void open()} disabled={busy}>
            {text("Open folder", "打开目录")}
          </button>
          <button onClick={() => input.current?.click()}>
            {text("Import graph", "导入图谱")}
          </button>
          <button onClick={() => update("focus", true)} title="F">
            {text("Focus", "专注")}
          </button>
          {isTauri() && (
            <span className="window-actions">
              {(["minimize", "maximize", "close"] as const).map((action, i) => (
                <button
                  key={action}
                  aria-label={action}
                  onClick={() => void invoke("window_action", { action })}
                >
                  {["−", "□", "×"][i]}
                </button>
              ))}
            </span>
          )}
        </span>
      </header>
      {!graph && !busy && (
        <section className="welcome">
          <div className="welcome-glyph" aria-hidden>
            ◈
          </div>
          <h1>
            {text("See how your code fits together.", "看清代码如何协同。")}
          </h1>
          <p>
            {text(
              "Open a local repository. Explore its structure, follow a call and learn from the source evidence.",
              "打开本地仓库，探索代码结构、追踪调用，并通过源码证据逐步学习。",
            )}
          </p>
          <button className="primary" onClick={() => void open()}>
            {text(
              isTauri() ? "Open repository" : "Import an analyzed graph",
              isTauri() ? "打开代码仓库" : "导入已分析的图谱",
            )}
          </button>
          <p className="muted">
            {text(
              "Local analysis · No model or API key required",
              "本地分析 · 无需模型或 API Key",
            )}
          </p>
          {!isTauri() && (
            <p className="muted">
              {text(
                "In a browser, import JSON from glyphibe export. The desktop app scans folders directly.",
                "浏览器支持导入 glyphibe export 输出的 JSON。桌面程序可直接扫描目录。",
              )}
            </p>
          )}
        </section>
      )}
      {busy && (
        <section className="scan glass" role="status">
          <h2>{text("Reading repository structure", "正在读取仓库结构")}</h2>
          <p>{progress}</p>
          <button onClick={() => void invoke("cancel_scan")}>
            {text("Cancel scan", "取消扫描")}
          </button>
        </section>
      )}
      {graph && (
        <>
          <Island className="modes" title={text("Graph modes", "图谱模式")}>
            <nav aria-label="Graph mode">
              {modes.map((mode) => (
                <button
                  key={mode}
                  aria-pressed={view.mode === mode}
                  onClick={() => update("mode", mode)}
                >
                  {mode}
                </button>
              ))}
            </nav>
          </Island>
          <aside
            className={`left-island glass ${collapsed ? "collapsed" : ""}`}
            onMouseLeave={() => {
              if (!view.pinned) setCollapsed(true);
            }}
            onMouseEnter={() => {
              if (!view.pinned) setCollapsed(false);
            }}
          >
            <div className="panel-heading">
              <strong>{text("Explore", "探索")}</strong>
              <button
                aria-label="Pin tools"
                aria-pressed={view.pinned}
                onClick={() => update("pinned", !view.pinned)}
              >
                ⌖
              </button>
              <button
                aria-label="Collapse tools"
                aria-expanded={!collapsed}
                onClick={() => setCollapsed(!collapsed)}
              >
                {collapsed ? "+" : "−"}
              </button>
            </div>
            {!collapsed && (
              <>
                <label>
                  {text("Plane", "平面")}
                  <select
                    aria-label="Graph plane"
                    value={view.plane}
                    onChange={(e) =>
                      update("plane", e.target.value as ViewState["plane"])
                    }
                  >
                    <option value="architecture">
                      {text("Architecture", "架构")}
                    </option>
                    <option value="explanation">
                      {text("Explanation", "解释")}
                    </option>
                  </select>
                </label>
                <label className="search-label">
                  {text("Find an entity", "查找节点")}
                  <input
                    ref={searchInput}
                    type="search"
                    placeholder={text(
                      "Name, symbol…  Ctrl K",
                      "名称、符号…  Ctrl K",
                    )}
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                </label>
                <div className="entity-list" role="list" aria-label="Entities">
                  {results.map((n) => (
                    <button
                      role="listitem"
                      key={n.id}
                      className={n.id === view.selected ? "selected" : ""}
                      onClick={() => select(n.id)}
                      title={
                        n.source
                          ? `${n.source.path}:${n.source.start_line}`
                          : n.kind
                      }
                    >
                      <span>{n.label}</span>
                      <small>{n.kind}</small>
                    </button>
                  ))}
                </div>
                <p className="muted">
                  {results.length === 80
                    ? text(
                        "First 80 matches. Refine your search.",
                        "显示前 80 项，请细化搜索。",
                      )
                    : `${results.length} ${text("entities", "个节点")}`}
                </p>
                <details>
                  <summary>
                    {text("Appearance & language", "外观与语言")}
                  </summary>
                  <label>
                    {text("Theme", "主题")}
                    <select
                      aria-label="Theme"
                      value={view.theme}
                      onChange={(e) =>
                        update("theme", e.target.value as ViewState["theme"])
                      }
                    >
                      <option value="dark">{text("Dark", "深色")}</option>
                      <option value="light">{text("Light", "浅色")}</option>
                    </select>
                  </label>
                  <label>
                    {text("Language", "语言")}
                    <select
                      aria-label="Language"
                      value={view.locale}
                      onChange={(e) =>
                        update("locale", e.target.value as ViewState["locale"])
                      }
                    >
                      <option value="en">English</option>
                      <option value="zh">中文</option>
                    </select>
                  </label>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={view.opaque}
                      onChange={(e) => update("opaque", e.target.checked)}
                    />
                    {text("Opaque panels", "不透明面板")}
                  </label>
                  <label>
                    {text("Galaxy rendering detail", "三维绘制细节")}
                    <select
                      aria-label="Galaxy rendering detail"
                      value={view.graphicsPolicy}
                      onChange={(e) =>
                        update(
                          "graphicsPolicy",
                          e.target.value as ViewState["graphicsPolicy"],
                        )
                      }
                    >
                      <option value="auto">
                        {text("Automatic device limit", "自动设备上限")}
                      </option>
                      <option value="full">
                        {text(
                          "Full detail (up to 2,000)",
                          "完整细节（最多 2,000）",
                        )}
                      </option>
                    </select>
                  </label>
                  <p className="muted">
                    {text(
                      "Rendering limits change visible nodes only; all analysis facts remain available.",
                      "绘制上限只改变可见节点，全部分析事实仍可查询。",
                    )}
                  </p>
                </details>
              </>
            )}
          </aside>
          <Island
            className="controls"
            title={text("Graph controls", "图谱参数")}
          >
            <div className="control-grid">
              <label>
                {text("Architecture", "架构粒度")}
                <output>
                  {(view.locale === "zh" ? zhLevels : levels)[view.level - 1]}
                </output>
                <input
                  aria-label="Architecture level"
                  type="range"
                  min="1"
                  max="6"
                  step="1"
                  value={view.level}
                  onChange={(e) => update("level", Number(e.target.value))}
                />
              </label>
              <label>
                {text("Explanation", "解释深度")}
                <output>
                  {view.locale === "zh"
                    ? ["零基础", "入门", "熟练", "专业"][view.depth - 1]
                    : depths[view.depth - 1]}
                </output>
                <input
                  aria-label="Explanation depth"
                  type="range"
                  min="1"
                  max="4"
                  step="1"
                  value={view.depth}
                  onChange={(e) => update("depth", Number(e.target.value))}
                />
              </label>
              <label>
                {text("Visible nodes", "显示密度")}
                <output>{view.density}</output>
                <input
                  aria-label="Graph density"
                  type="range"
                  min="50"
                  max="10000"
                  step="50"
                  value={view.density}
                  onChange={(e) => update("density", Number(e.target.value))}
                />
              </label>
            </div>
          </Island>
          <div className="status glass">
            {projection.nodes.length} / {projection.total}{" "}
            {text("visible", "可见")} · {projection.edges.length}{" "}
            {text("relations", "条关系")}
            {view.mode === "Galaxy" &&
              ` · ${text("3D rendering cap", "三维绘制上限")}: ${effectiveView.galaxyCap}${view.graphicsPolicy === "auto" && graphicsReason ? ` (${graphicsReason})` : ""}`}
            {graph.diagnostics.length > 0 &&
              ` · ${graph.diagnostics.length} diagnostics`}
          </div>
          {projection.nodes.length === 0 && (
            <p className="canvas-empty">
              {text(
                "No entities at this level. Choose another architecture level or select an entity.",
                "此层级没有节点。请切换架构粒度或选择其他节点。",
              )}
            </p>
          )}
          {node && (
            <Island className="detail" title={text("Node details", "节点详情")}>
              <div className="panel-heading">
                <span className={`evidence ${node.evidence.kind}`}>
                  {node.evidence.kind}
                </span>
                <button
                  aria-label="Close details"
                  onClick={() => update("selected", null)}
                >
                  ×
                </button>
              </div>
              <h2>{node.label}</h2>
              <p className="muted">
                L{node.level} / {node.kind} / {node.change}
              </p>
              <div className="tabs" role="tablist">
                {(["Evidence", "Source", "Learn", "Knowledge"] as const).map(
                  (t, i) => (
                    <button
                      role="tab"
                      aria-label={t}
                      aria-selected={tab === t}
                      key={t}
                      onClick={() => setTab(t)}
                    >
                      {view.locale === "zh"
                        ? ["证据", "源码", "学习", "知识"][i]
                        : t}
                    </button>
                  ),
                )}
              </div>
              {tab === "Evidence" && (
                <>
                  <p>{node.summary}</p>
                  {node.source && (
                    <p className="source-location">
                      {node.source.path}:{node.source.start_line}–
                      {node.source.end_line}
                    </p>
                  )}
                  <p className="muted">{node.evidence.method}</p>
                  {node.evidence.note && <p>{node.evidence.note}</p>}
                  <h3>{text("Modification reach", "修改影响范围")}</h3>
                  <p className="muted">
                    {text(
                      "Reachability shows possible dependencies, not a failure probability.",
                      "可达关系表示潜在依赖，不代表故障概率。",
                    )}
                  </p>
                  <label>
                    {text("Direction", "方向")}
                    <select
                      aria-label="Impact direction"
                      value={view.direction}
                      onChange={(e) =>
                        update(
                          "direction",
                          e.target.value as ViewState["direction"],
                        )
                      }
                    >
                      <option value="upstream">
                        {text("Dependents (upstream)", "被依赖方（上游）")}
                      </option>
                      <option value="downstream">
                        {text("Dependencies (downstream)", "依赖项（下游）")}
                      </option>
                    </select>
                  </label>
                  <label>
                    {text("Hops", "追踪跳数")}
                    <output>{view.hops}</output>
                    <input
                      aria-label="Impact hops"
                      type="range"
                      min="1"
                      max="10"
                      value={view.hops}
                      onChange={(e) => update("hops", Number(e.target.value))}
                    />
                  </label>
                  <p>
                    {Math.max(0, impact.size - 1)}{" "}
                    {text("reachable entities highlighted", "个可达节点已高亮")}
                  </p>
                  {impactInfo && (
                    <>
                      {currentNativeImpact ? (
                        <details aria-label="Impact source scope">
                          <summary>
                            {text(
                              "Source scope and evidence",
                              "源码范围与证据",
                            )}{" "}
                            ({currentNativeImpact.scope.dependency_seeds.length}
                            )
                          </summary>
                          <p className="muted">
                            {text(
                              "Containment and owning-function steps establish the source scope. Only dependency edges consume the selected hops.",
                              "包含关系与所属函数确定源码范围；只有依赖边消耗所选追踪跳数。",
                            )}
                          </p>
                          <p className="muted">
                            {currentNativeImpact.limitation}
                          </p>
                          {currentNativeImpact.scope.basis
                            .slice(0, 50)
                            .map((b) => (
                              <p key={`${b.edge_id}:${b.step}`}>
                                {graph.nodes[b.source]?.label} {b.relation}{" "}
                                {graph.nodes[b.target]?.label}
                                <small>
                                  {" "}
                                  {b.evidence}: {b.method}
                                </small>
                              </p>
                            ))}
                          {currentNativeImpact.scope.truncated && (
                            <p>
                              {text(
                                "Scope truncated at the safety limit; narrow the selected scope.",
                                "范围已达到安全上限，请缩小选择范围。",
                              )}
                            </p>
                          )}
                        </details>
                      ) : (
                        !nativeGraph && (
                          <p className="muted">
                            {text(
                              "Imported graph: raw dependency projection. Open the repository in the desktop app for owning-function and aggregate source-scope analysis.",
                              "导入图谱显示原始依赖投影。桌面打开仓库后可分析所属函数与聚合源码范围。",
                            )}
                          </p>
                        )
                      )}
                      <dl className="impact-counts">
                        <div>
                          <dt>{text("Components", "组件")}</dt>
                          <dd>{impactInfo.components}</dd>
                        </div>
                        <div>
                          <dt>{text("Functions", "函数")}</dt>
                          <dd>{impactInfo.functions}</dd>
                        </div>
                        <div>
                          <dt>{text("Tests", "测试")}</dt>
                          <dd>{impactInfo.tests}</dd>
                        </div>
                        <div>
                          <dt>{text("Known interfaces", "已知接口")}</dt>
                          <dd>{impactInfo.interfaces}</dd>
                        </div>
                      </dl>
                      <details>
                        <summary>
                          {text("Dependency paths", "依赖路径")}
                        </summary>
                        {Object.entries(impactInfo.paths)
                          .filter(([id]) => id !== node.id)
                          .slice(0, 50)
                          .map(([id, path]) => (
                            <button
                              className="path"
                              key={id}
                              onClick={() => select(id)}
                            >
                              {path
                                .map((part) => graph.nodes[part].label)
                                .join(" → ")}
                            </button>
                          ))}
                      </details>
                    </>
                  )}
                  <details>
                    <summary>
                      {text("Trace upward through structure", "向上追溯结构")} (
                      {ancestors.length})
                    </summary>
                    {ancestors.slice(0, 40).map((n) => (
                      <button
                        className="hierarchy-entry"
                        key={n.id}
                        onClick={() => select(n.id)}
                      >
                        L{n.level} {n.label}
                      </button>
                    ))}
                  </details>
                  <details>
                    <summary>
                      {text("Expand contained structure", "向下展开结构")} (
                      {descendants.length})
                    </summary>
                    {descendants.slice(0, 60).map((n) => (
                      <button
                        className="hierarchy-entry"
                        key={n.id}
                        onClick={() => select(n.id)}
                      >
                        L{n.level} {n.label}
                      </button>
                    ))}
                  </details>
                  <h3>{text("Connections & mappings", "关系与映射")}</h3>
                  <div className="relations">
                    {relationships.map(({ edge, node: n }) => (
                      <button key={edge.id} onClick={() => select(n.id)}>
                        <span>
                          {edge.relation}
                          {edge.stale ? " (stale)" : ""}
                        </span>
                        <strong>{n.label}</strong>
                        <small>{edge.evidence.kind}</small>
                      </button>
                    ))}
                  </div>
                </>
              )}
              {tab === "Source" && (
                <>
                  {node.source ? (
                    <>
                      <p className="source-location">
                        {node.source.path}:{node.source.start_line}
                      </p>
                      <pre>
                        {source ||
                          text(
                            "Source text is available when a repository is open in the desktop app. An exported graph preserves source locations.",
                            "桌面程序打开仓库后可读取源码。导入图谱保留源码位置。",
                          )}
                      </pre>
                    </>
                  ) : (
                    <p>
                      {text(
                        "This entity has no direct source range. Follow its mappings.",
                        "此节点没有直接源码范围，请沿映射关系定位。",
                      )}
                    </p>
                  )}
                </>
              )}
              {tab === "Learn" && (
                <>
                  {explanation ? (
                    <>
                      <h3>{explanation.title}</h3>
                      {explanation.facts.map((f, i) => (
                        <p key={i}>{f}</p>
                      ))}
                      {explanation.uncertainty.map((u, i) => (
                        <p className="uncertain" key={i}>
                          {u}
                        </p>
                      ))}
                      {lesson && (
                        <section className="lesson">
                          <p>
                            {step + 1} / {lessons.length}
                          </p>
                          <h3>{lesson.question}</h3>
                          {revealed !== "question" && <p>{lesson.hint}</p>}
                          {revealed === "answer" && <p>{lesson.answer}</p>}
                          {revealed === "answer" && (
                            <div className="review">
                              <h3>{text("Review", "回顾")}</h3>
                              <p>{lesson.review}</p>
                            </div>
                          )}
                          <button
                            onClick={() =>
                              setRevealed(
                                revealed === "question" ? "hint" : "answer",
                              )
                            }
                          >
                            {revealed === "question"
                              ? text("Show hint", "查看提示")
                              : text("Show answer", "查看答案")}
                          </button>
                          <button
                            disabled={step === 0}
                            onClick={() => {
                              setStep(step - 1);
                              setRevealed("question");
                            }}
                          >
                            {text("Previous", "上一步")}
                          </button>
                          <button
                            disabled={step >= lessons.length - 1}
                            onClick={() => {
                              setStep(step + 1);
                              setRevealed("question");
                            }}
                          >
                            {text("Next", "下一步")}
                          </button>
                        </section>
                      )}
                    </>
                  ) : (
                    <>
                      <h3>{text("Start with the evidence", "先观察证据")}</h3>
                      <p>{node.summary}</p>
                      <p>
                        {text(
                          "What entities connect to this one? Open Evidence and follow a relation. Full bilingual guided lessons are generated by the shared Rust explanation engine in the desktop app.",
                          "哪些节点与它相连？打开 Evidence 并沿关系探索。桌面程序通过共享 Rust 解释引擎提供完整双语教学。",
                        )}
                      </p>
                    </>
                  )}
                </>
              )}
              {tab === "Knowledge" && (
                <>
                  <h3>{text("Concept links", "概念关联")}</h3>
                  {concepts.map((concept) => (
                    <section className="concept" key={concept.id}>
                      <h3>{concept.label[view.locale]}</h3>
                      <p>{concept.description[view.locale]}</p>
                      <small>
                        {text("Prerequisites", "前置概念")}:{" "}
                        {concept.prerequisites
                          .map(
                            (id) =>
                              registry.concepts.find((c) => c.id === id)?.label[
                                view.locale
                              ] ?? id,
                          )
                          .join(" / ") || text("None", "无")}
                      </small>
                      <p className="muted">
                        {concept.attribution.author} ·{" "}
                        {concept.attribution.license}
                        <br />
                        {concept.attribution.redistribution}
                      </p>
                    </section>
                  ))}
                  {node.kind === "concept" && <p>{node.summary}</p>}
                  {node.knowledge.map((k) => (
                    <p key={k}>{k}</p>
                  ))}
                  {!relationships.some(
                    (r) => r.edge.relation === "has_concept",
                  ) &&
                    node.kind !== "concept" && (
                      <p>
                        {text(
                          "No concept is linked by the current analysis. Business meaning remains unverified.",
                          "当前分析没有关联概念。业务语义仍未验证。",
                        )}
                      </p>
                    )}
                  {relationships
                    .filter((r) => r.edge.relation === "has_concept")
                    .map(({ node: n, edge }) => (
                      <section key={edge.id}>
                        <button onClick={() => select(n.id)}>{n.label}</button>
                        <p>{n.summary}</p>
                        <small>{n.evidence.method}</small>
                      </section>
                    ))}
                </>
              )}
            </Island>
          )}
        </>
      )}
      {error && (
        <div className="error glass" role="alert">
          <span>{error}</span>
          <button aria-label="Dismiss error" onClick={() => setError("")}>
            ×
          </button>
        </div>
      )}
      {view.focus && (
        <button
          className="restore glass"
          onClick={() => update("focus", false)}
        >
          {text("Exit focus · Esc", "退出专注 · Esc")}
        </button>
      )}
    </main>
  );
}
