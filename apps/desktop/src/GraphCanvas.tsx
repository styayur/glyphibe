import {
  Component,
  lazy,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import type { ForceGraphMethods } from "react-force-graph-3d";
import { MultiDirectedGraph } from "graphology";
import Sigma from "sigma";
import { clusterPositions, colour, escapeMarkup, positions } from "./model";
import type { CodeEdge, CodeNode, ViewState } from "./model";
const ForceGraph3D = lazy(() => import("react-force-graph-3d"));
class GraphicsBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? (
      <p className="canvas-note" role="alert">
        3D graphics are unavailable on this device. Switch to a 2D view or
        navigate with the entity list.
      </p>
    ) : (
      this.props.children
    );
  }
}
interface Props {
  nodes: CodeNode[];
  edges: CodeEdge[];
  state: ViewState;
  impact: Set<string>;
  onSelect: (id: string) => void;
  onCadence: (cadence: number) => void;
  communities: Map<string, string[]>;
}
export default function GraphCanvas({
  nodes,
  edges,
  state,
  impact,
  onSelect,
  onCadence,
  communities,
}: Props) {
  const host = useRef<HTMLDivElement>(null);
  const galaxy = useRef<ForceGraphMethods | undefined>(undefined);
  const pickSphere = useRef<((x: number, y: number) => string | null) | null>(
    null,
  );
  const pointerGesture = useRef<{
    id: number;
    x: number;
    y: number;
    moved: number;
  } | null>(null);
  const handledPointer = useRef(-1);
  const renderer2D = useRef<Sigma | null>(null);
  const graph2D = useRef<MultiDirectedGraph | null>(null);
  const selectHandler = useRef(onSelect);
  const previousGraphKey = useRef("");
  const appliedPresentation = useRef<{
    renderer: Sigma;
    selected: string | null;
    theme: string;
    density: number;
    impact: Set<string>;
  } | null>(null);
  const camera = useRef<{
    x: number;
    y: number;
    angle: number;
    ratio: number;
  } | null>(null);
  const [size, setSize] = useState({ width: innerWidth, height: innerHeight });
  const [error, setError] = useState("");
  const nodeKey = useMemo(
    () =>
      nodes
        .map((n) => `${n.id}:${n.label}:${n.kind}`)
        .sort()
        .join("\u0000"),
    [nodes],
  );
  const edgeKey = useMemo(
    () =>
      edges
        .map((e) => e.id)
        .sort()
        .join("\u0000"),
    [edges],
  );
  const isGalaxy = state.mode === "Galaxy";
  const layoutKey =
    state.mode === "Cluster"
      ? `cluster:${nodes
          .map((n) => `${n.id}:${communities.get(n.id)?.join(",") ?? ""}`)
          .sort()
          .join("|")}`
      : "radial";
  const layout = useMemo(
    () =>
      state.mode === "Cluster"
        ? clusterPositions(nodes, communities)
        : positions(nodes),
    [nodeKey, layoutKey],
  );
  const data3D = useMemo(
    () => ({
      nodes: nodes.map((n) => ({
        id: n.id,
        name: n.label,
        kind: n.kind,
        ...layout.get(n.id),
        fx: layout.get(n.id)!.x,
        fy: layout.get(n.id)!.y,
        fz: layout.get(n.id)!.z,
      })),
      links: edges.map((e) => ({ source: e.source, target: e.target })),
    }),
    [nodeKey, edgeKey, layoutKey],
  );
  useEffect(() => {
    if (!isGalaxy) return;
    let active = true;
    // Keep Three.js lazy for 2D-only sessions.
    void import("./picking").then(({ createSpherePicker }) => {
      if (active)
        pickSphere.current = createSpherePicker(
          () => galaxy.current,
          data3D.nodes,
          5,
        );
    });
    return () => {
      active = false;
      pickSphere.current = null;
      pointerGesture.current = null;
    };
  }, [isGalaxy, data3D]);
  useEffect(() => {
    selectHandler.current = onSelect;
  }, [onSelect]);
  useEffect(() => {
    if (!isGalaxy) return;
    const position = state.selected ? layout.get(state.selected) : null;
    if (position)
      galaxy.current?.cameraPosition(
        { x: position.x + 80, y: position.y + 40, z: position.z + 140 },
        position,
        0,
      );
  }, [state.selected, isGalaxy, layout]);
  useEffect(() => {
    if (!isGalaxy) return;
    let frame = 0,
      started = 0,
      count = 0,
      cancelled = false;
    const sample = (time: number) => {
      if (cancelled) return;
      if (!started) started = time;
      count++;
      if (time - started >= 3000 && count >= 30) {
        onCadence(((count - 1) * 1000) / (time - started));
        return;
      }
      frame = requestAnimationFrame(sample);
    };
    frame = requestAnimationFrame(sample);
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [isGalaxy, nodeKey, onCadence]);
  useEffect(() => {
    const resize = () => setSize({ width: innerWidth, height: innerHeight });
    addEventListener("resize", resize);
    return () => removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    if (isGalaxy || !host.current) return;
    const graph = new MultiDirectedGraph();
    for (const n of nodes)
      graph.addNode(n.id, {
        label: n.label,
        ...layout.get(n.id),
        size: n.id === state.selected ? 10 : n.kind === "cluster" ? 8 : 5,
        color:
          n.id === state.selected
            ? "#f4d58e"
            : impact.has(n.id)
              ? "#df9b6c"
              : colour(n, state.theme),
        forceLabel: n.id === state.selected,
      });
    for (const e of edges)
      graph.addDirectedEdgeWithKey(e.id, e.source, e.target, {
        size: 1,
        color:
          e.evidence.kind === "inferred"
            ? "#8b7952"
            : state.theme === "dark"
              ? "#415366"
              : "#8b9ba9",
      });
    let renderer: Sigma | undefined;
    try {
      renderer = new Sigma(graph, host.current, {
        renderEdgeLabels: false,
        labelDensity: state.density < 500 ? 1 : 0.15,
        labelColor: { color: state.theme === "dark" ? "#e4edf4" : "#1c3142" },
        labelSize: 12,
        allowInvalidContainer: true,
      });
      renderer.on("clickNode", ({ node }) => selectHandler.current(node));
      if (
        camera.current &&
        previousGraphKey.current === `${nodeKey}|${edgeKey}|${layoutKey}`
      )
        renderer.getCamera().setState(camera.current);
      previousGraphKey.current = `${nodeKey}|${edgeKey}|${layoutKey}`;
      renderer2D.current = renderer;
      graph2D.current = graph;
      // Construction already supplied these presentation attributes. Reapplying
      // four settings here would repeatedly rebuild Sigma's entire index.
      appliedPresentation.current = {
        renderer,
        selected: state.selected,
        theme: state.theme,
        density: state.density,
        impact,
      };
      setError("");
    } catch (e) {
      setError(
        `Graphics unavailable: ${e instanceof Error ? e.message : String(e)}. Use the entity list to navigate.`,
      );
    }
    return () => {
      if (renderer) {
        camera.current = renderer.getCamera().getState();
        renderer.kill();
      }
      renderer2D.current = null;
      graph2D.current = null;
      appliedPresentation.current = null;
    };
  }, [nodeKey, edgeKey, isGalaxy, layoutKey]);
  useEffect(() => {
    const renderer = renderer2D.current,
      graph = graph2D.current;
    if (!renderer || !graph) return;
    const applied = appliedPresentation.current;
    if (
      applied?.renderer === renderer &&
      applied.selected === state.selected &&
      applied.theme === state.theme &&
      applied.density === state.density &&
      applied.impact === impact
    )
      return;
    const byId = new Map(nodes.map((n) => [n.id, n]));
    const edgeById = new Map(edges.map((e) => [e.id, e]));
    // Batch presentation changes. Canonical graph facts and GPU contexts stay
    // intact; selection/highlighting does not reconstruct a renderer.
    renderer.setSettings({
      nodeReducer: (id, attributes) => {
        const node = byId.get(id)!;
        return {
          ...attributes,
          label: node.label,
          size: id === state.selected ? 10 : node.kind === "cluster" ? 8 : 5,
          color:
            id === state.selected
              ? "#f4d58e"
              : impact.has(id)
                ? "#df9b6c"
                : colour(node, state.theme),
          forceLabel: id === state.selected,
        };
      },
      labelColor: {
        color: state.theme === "dark" ? "#e4edf4" : "#1c3142",
      },
      labelDensity: state.density < 500 ? 1 : 0.15,
      edgeReducer: (id, attributes) => ({
        ...attributes,
        color:
          edgeById.get(id)?.evidence.kind === "inferred"
            ? "#8b7952"
            : state.theme === "dark"
              ? "#415366"
              : "#8b9ba9",
      }),
    });
    appliedPresentation.current = {
      renderer,
      selected: state.selected,
      theme: state.theme,
      density: state.density,
      impact,
    };
  }, [
    nodeKey,
    edgeKey,
    layoutKey,
    isGalaxy,
    state.selected,
    state.theme,
    state.density,
    impact,
  ]);
  return (
    <div
      className="canvas-shell"
      aria-label={`${state.mode} graph`}
      data-testid="graph-canvas"
      data-mode={state.mode}
      data-nodes={nodes.length}
      data-cap={state.galaxyCap}
      data-level={state.level}
      data-layout={state.mode === "Cluster" ? "community" : "radial"}
      onPointerDownCapture={(event) => {
        if (
          !isGalaxy ||
          event.button !== 0 ||
          !(event.target instanceof HTMLCanvasElement)
        )
          return;
        pointerGesture.current = {
          id: event.pointerId,
          x: event.clientX,
          y: event.clientY,
          moved: 0,
        };
      }}
      onPointerMoveCapture={(event) => {
        const gesture = pointerGesture.current;
        if (gesture?.id === event.pointerId)
          gesture.moved = Math.max(
            gesture.moved,
            Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y),
          );
      }}
      onPointerCancelCapture={() => {
        pointerGesture.current = null;
      }}
      onPointerUpCapture={(event) => {
        const gesture = pointerGesture.current;
        pointerGesture.current = null;
        if (!gesture || gesture.id !== event.pointerId || event.button !== 0)
          return;
        const moved = Math.max(
          gesture.moved,
          Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y),
        );
        if (moved > 4) return; // Orbit/pan gestures must never select a sphere.
        const id = pickSphere.current?.(event.clientX, event.clientY);
        if (id) {
          handledPointer.current = event.timeStamp;
          selectHandler.current(id);
        }
      }}
    >
      {state.mode === "Galaxy" ? (
        <GraphicsBoundary>
          <Suspense
            fallback={<p className="canvas-note">Loading 3D renderer…</p>}
          >
            <ForceGraph3D
              ref={galaxy}
              width={size.width}
              height={size.height}
              graphData={data3D}
              backgroundColor={state.theme === "dark" ? "#171d26" : "#e6edf1"}
              nodeLabel={(node) => escapeMarkup(String(node.name))}
              nodeColor={(node) =>
                node.id === state.selected
                  ? "#f4d58e"
                  : impact.has(String(node.id))
                    ? "#df9b6c"
                    : colour(
                        { kind: String(node.kind) } as CodeNode,
                        state.theme,
                      )
              }
              nodeRelSize={5}
              nodeVal={1}
              linkColor={() => (state.theme === "dark" ? "#50657b" : "#92a1af")}
              linkOpacity={0.55}
              showNavInfo={false}
              enableNodeDrag={false}
              cooldownTicks={0}
              onEngineStop={() => {
                const selected = data3D.nodes.find(
                  (n) => n.id === state.selected,
                );
                if (selected)
                  galaxy.current?.cameraPosition(
                    {
                      x: (selected.x ?? 0) + 80,
                      y: (selected.y ?? 0) + 40,
                      z: (selected.z ?? 0) + 140,
                    },
                    {
                      x: selected.x ?? 0,
                      y: selected.y ?? 0,
                      z: selected.z ?? 0,
                    },
                    0,
                  );
                else galaxy.current?.zoomToFit(0, 70);
              }}
              onNodeClick={(node, event) => {
                if (event.timeStamp !== handledPointer.current)
                  selectHandler.current(String(node.id));
              }}
            />
          </Suspense>
        </GraphicsBoundary>
      ) : (
        <div ref={host} className="graph-2d" data-testid="sigma-canvas" />
      )}
      {error && state.mode !== "Galaxy" && (
        <p className="canvas-note" role="alert">
          {error}
        </p>
      )}
      <div className="graph-caption">
        {state.mode === "Cluster"
          ? state.locale === "zh"
            ? "结构集群布局 · 最近的已记录成员关系 · 未分配节点保持未分配"
            : "Structural community layout · nearest recorded memberships · unassigned stays unassigned"
          : state.mode === "Galaxy"
            ? "Drag to rotate · Scroll to zoom"
            : "Drag to pan · Scroll to zoom · Select a node"}
      </div>
    </div>
  );
}
