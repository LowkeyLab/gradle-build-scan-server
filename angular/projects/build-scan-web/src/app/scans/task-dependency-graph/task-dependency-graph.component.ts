import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  inject,
  input,
  signal,
  type ElementRef,
  viewChild,
} from "@angular/core";
import {
  CanvasEvent,
  Graph,
  NodeEvent,
  type GraphData,
  type GraphOptions,
  type IElementEvent,
} from "@antv/g6";

interface TaskEdge {
  node: {
    id: string;
    dependencies: string[];
    taskPath: string;
    outcome: string;
    durationMs: number | null;
    startTimestamp: number | null;
    finishTimestamp: number | null;
  };
}

interface TaskDependencyNode {
  id: string;
  label: string;
  displayLabel: string;
  outcome: string;
  startTimestamp: number | null;
  finishTimestamp: number | null;
  durationMs: number | null;
  x: number;
  y: number;
}

interface TaskDependencyEdge {
  sourceId: string;
  targetId: string;
}

interface TaskDependencyHighlightState {
  activeNodeId: string;
  mode: "hover" | "selected";
  highlightedNodeIds: ReadonlySet<string>;
  highlightedEdgeKeys: ReadonlySet<string>;
}

interface LegendNodeItem {
  label: string;
  fillColor: string;
  strokeColor: string;
}

interface G6TaskGraphData {
  data: GraphData;
  key: string;
  nodeIds: string[];
  edgeIds: string[];
}

const NODE_WIDTH = 220;
const NODE_HEIGHT = 52;
const NODE_RADIUS = 12;
const NODE_SIZE: [number, number] = [NODE_WIDTH, NODE_HEIGHT];
const NODE_VERTICAL_GAP = 56;
const NODE_RANK_GAP = 96;
const TIMELINE_WIDTH = 1400;
const FIT_VIEW_PADDING = 32;

const SUCCESS_STYLE = {
  fillColor: "oklch(72% 0.17 150 / 0.18)",
  strokeColor: "oklch(72% 0.17 150)",
};

const UP_TO_DATE_STYLE = {
  fillColor: "oklch(72% 0.17 150 / 0.12)",
  strokeColor: "oklch(72% 0.17 150 / 0.7)",
};

const FROM_CACHE_STYLE = {
  fillColor: "oklch(72% 0.15 230 / 0.18)",
  strokeColor: "oklch(72% 0.15 230)",
};

const FAILED_STYLE = {
  fillColor: "oklch(62% 0.2 25 / 0.18)",
  strokeColor: "oklch(62% 0.2 25)",
};

const SKIPPED_STYLE = {
  fillColor: "oklch(75% 0.15 75 / 0.18)",
  strokeColor: "oklch(75% 0.15 75)",
};

const OUTCOME_STYLES: Record<
  string,
  { fillColor: string; strokeColor: string }
> = {
  Success: SUCCESS_STYLE,
  UpToDate: UP_TO_DATE_STYLE,
  FromCache: FROM_CACHE_STYLE,
  Failed: FAILED_STYLE,
  Skipped: SKIPPED_STYLE,
};

const FALLBACK_STYLE = {
  fillColor: "oklch(55% 0.04 260 / 0.12)",
  strokeColor: "oklch(55% 0.04 260)",
};

const LEGEND_NODE_ITEMS: LegendNodeItem[] = [
  { label: "Success", ...SUCCESS_STYLE },
  { label: "From Cache", ...FROM_CACHE_STYLE },
  { label: "Up To Date", ...UP_TO_DATE_STYLE },
  { label: "Skipped", ...SKIPPED_STYLE },
  { label: "Other / Unknown", ...FALLBACK_STYLE },
  { label: "Failed", ...FAILED_STYLE },
];

function truncateTaskLabel(label: string): string {
  if (label.length <= 28) return label;
  return `${label.slice(0, 25)}…`;
}

function edgeKey(edge: TaskDependencyEdge): string {
  return `${edge.sourceId}:${edge.targetId}`;
}

function getOutcomeStyle(outcome: string): {
  fillColor: string;
  strokeColor: string;
} {
  return OUTCOME_STYLES[outcome] ?? FALLBACK_STYLE;
}

function buildGraphDataKey(graph: {
  nodes: TaskDependencyNode[];
  edges: TaskDependencyEdge[];
}): string {
  const nodeKey = graph.nodes
    .map(
      (node) =>
        `${node.id}:${node.label}:${node.outcome}:${node.startTimestamp ?? ""}:${node.finishTimestamp ?? ""}:${node.durationMs ?? ""}:${node.x}:${node.y}`,
    )
    .join("|");
  const edgeKey = graph.edges
    .map((edge) => `${edge.sourceId}:${edge.targetId}`)
    .join("|");
  return `${nodeKey}::${edgeKey}`;
}

function taskStartMs(task: TaskEdge["node"]): number | null {
  if (task.startTimestamp != null) return task.startTimestamp;
  if (task.finishTimestamp != null && task.durationMs != null) {
    return task.finishTimestamp - task.durationMs;
  }
  return null;
}

function taskFinishMs(task: TaskEdge["node"]): number | null {
  if (task.finishTimestamp != null) return task.finishTimestamp;
  const start = taskStartMs(task);
  if (start != null && task.durationMs != null) return start + task.durationMs;
  return start;
}

function positionTaskNodes(tasks: TaskEdge["node"][]): TaskDependencyNode[] {
  const timedTasks = tasks
    .map((task) => ({
      task,
      startMs: taskStartMs(task),
      finishMs: taskFinishMs(task),
    }))
    .filter(
      (
        entry,
      ): entry is {
        task: TaskEdge["node"];
        startMs: number;
        finishMs: number | null;
      } => entry.startMs != null,
    );

  const startTimes = timedTasks.map((entry) => entry.startMs);
  const minStartMs = startTimes.length > 0 ? Math.min(...startTimes) : 0;
  const maxStartMs = startTimes.length > 0 ? Math.max(...startTimes) : 0;
  const timeSpanMs = Math.max(maxStartMs - minStartMs, 1);
  const pxPerMs = TIMELINE_WIDTH / timeSpanMs;
  const laneEndTimes: number[] = [];

  return tasks
    .sort((a, b) => {
      const startA = taskStartMs(a);
      const startB = taskStartMs(b);
      if (startA != null && startB != null && startA !== startB) {
        return startA - startB;
      }
      if (startA != null && startB == null) return -1;
      if (startA == null && startB != null) return 1;
      return a.taskPath.localeCompare(b.taskPath);
    })
    .map((task, index) => {
      const startMs = taskStartMs(task);
      const finishMs = taskFinishMs(task);
      const x =
        startMs == null
          ? TIMELINE_WIDTH + (index + 1) * NODE_RANK_GAP
          : Math.round((startMs - minStartMs) * pxPerMs);
      const safeFinishMs = finishMs ?? startMs ?? Number.POSITIVE_INFINITY;
      let laneIndex = laneEndTimes.findIndex(
        (endMs) => endMs <= (startMs ?? 0),
      );
      if (laneIndex === -1) {
        laneIndex = laneEndTimes.length;
        laneEndTimes.push(safeFinishMs);
      } else {
        laneEndTimes[laneIndex] = safeFinishMs;
      }

      return {
        id: task.id,
        label: task.taskPath,
        displayLabel: truncateTaskLabel(task.taskPath),
        outcome: task.outcome,
        startTimestamp: task.startTimestamp,
        finishTimestamp: task.finishTimestamp,
        durationMs: task.durationMs,
        x,
        y: laneIndex * (NODE_HEIGHT + NODE_VERTICAL_GAP),
      };
    });
}

function getNodeIdFromEvent(event: IElementEvent): string | null {
  return event.target?.id ?? null;
}

@Component({
  selector: "app-task-dependency-graph",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="card bg-base-200 mb-6">
      <div class="card-body p-4">
        <div class="mb-3 flex items-start justify-between gap-4">
          <div>
            <h4 class="font-semibold">Task Dependencies</h4>
            <p class="text-xs opacity-60">
              Tasks are positioned horizontally by execution time.
            </p>
          </div>
          @if (graph().nodes.length > 0) {
            <div class="text-xs opacity-60">
              {{ graph().nodes.length }} nodes ·
              {{ graph().edges.length }} edges
            </div>
          }
        </div>

        @if (graph().nodes.length > 0) {
          <div
            data-testid="task-dependency-legend"
            class="mb-3 flex flex-wrap items-center gap-x-4 gap-y-2 rounded-box border border-base-300/70 bg-base-100/50 px-3 py-2 text-[11px] opacity-80"
          >
            <div class="font-medium uppercase tracking-wide opacity-70">
              Legend
            </div>
            @for (item of legendNodeItems; track item.label) {
              <div class="flex items-center gap-2">
                <span
                  class="inline-block h-3.5 w-3.5 rounded-[4px] border-2"
                  [style.background-color]="item.fillColor"
                  [style.border-color]="item.strokeColor"
                ></span>
                <span>{{ item.label }}</span>
              </div>
            }
            <div class="flex items-center gap-2">
              <span class="inline-block w-6 border-t-2 border-current"></span>
              <span>Task dependency</span>
            </div>
          </div>

          <div
            class="overflow-hidden rounded-box border border-base-300 bg-base-100/40 p-3"
          >
            <div
              #graphContainer
              data-testid="task-dependency-graph"
              class="task-dependency-g6 h-96 min-h-80 w-full touch-none select-none text-base-content"
            ></div>
          </div>
        } @else {
          <p class="text-sm opacity-60">No task dependency graph available.</p>
        }
      </div>
    </div>
  `,
})
export class TaskDependencyGraphComponent {
  taskEdges = input.required<TaskEdge[]>();
  legendNodeItems = LEGEND_NODE_ITEMS;
  readonly edgeKey = edgeKey;

  private graphContainer = viewChild<ElementRef<HTMLElement>>("graphContainer");
  private destroyRef = inject(DestroyRef);
  private g6Graph: Graph | null = null;
  private g6ContainerElement: HTMLElement | null = null;
  private lastRenderedGraphKey: string | null = null;
  private pendingGraphKey: string | null = null;
  private renderRequestId = 0;
  private hoveredNodeId = signal<string | null>(null);
  private selectedNodeId = signal<string | null>(null);
  private selectedNodeIdInGraph = computed(() => {
    const selectedNodeId = this.selectedNodeId();
    if (!selectedNodeId) return null;
    return this.graph().nodes.some((node) => node.id === selectedNodeId)
      ? selectedNodeId
      : null;
  });

  constructor() {
    this.destroyRef.onDestroy(() => {
      this.destroyGraph();
    });

    afterRenderEffect(() => {
      if (this.selectedNodeId() && !this.selectedNodeIdInGraph()) {
        this.selectedNodeId.set(null);
      }

      const containerRef = this.graphContainer();
      const graphData = this.g6GraphData();
      if (!containerRef || graphData.nodeIds.length === 0) {
        this.destroyGraph();
        return;
      }

      const container = containerRef.nativeElement;
      if (
        this.g6Graph &&
        this.g6ContainerElement === container &&
        this.lastRenderedGraphKey === graphData.key
      ) {
        return;
      }
      if (this.pendingGraphKey === graphData.key) return;

      this.pendingGraphKey = graphData.key;
      void this.renderG6Graph(container, graphData);
    });

    afterRenderEffect(() => {
      this.highlightState();
      this.g6GraphData();
      this.syncG6ElementStates();
    });
  }

  graph = computed(() => {
    const tasks = new Map<string, TaskEdge["node"]>();
    for (const edge of this.taskEdges()) {
      tasks.set(edge.node.id, edge.node);
    }

    const nodes = positionTaskNodes([...tasks.values()]);

    const nodeIds = new Set(nodes.map((node) => node.id));
    const seenEdges = new Set<string>();
    const edges = [...tasks.values()]
      .flatMap((task) =>
        (task.dependencies ?? []).map((sourceId) => ({
          sourceId,
          targetId: task.id,
        })),
      )
      .filter(
        (edge): edge is TaskDependencyEdge =>
          !!edge.sourceId &&
          !!edge.targetId &&
          nodeIds.has(edge.sourceId) &&
          nodeIds.has(edge.targetId) &&
          edge.sourceId !== edge.targetId,
      )
      .filter((edge) => {
        const key = `${edge.sourceId}:${edge.targetId}`;
        if (seenEdges.has(key)) return false;
        seenEdges.add(key);
        return true;
      });

    return { nodes, edges };
  });

  private g6GraphData = computed<G6TaskGraphData>(() => {
    const graph = this.graph();
    const nodes = graph.nodes.map((node) => {
      const style = getOutcomeStyle(node.outcome);
      return {
        id: node.id,
        data: {
          label: node.label,
          displayLabel: node.displayLabel,
          outcome: node.outcome,
        },
        style: {
          x: node.x,
          y: node.y,
          size: NODE_SIZE,
          radius: NODE_RADIUS,
          fill: style.fillColor,
          stroke: style.strokeColor,
          lineWidth: 2,
          labelText: node.displayLabel,
          labelFill: "currentColor",
          labelFontSize: 14,
          labelFontWeight: 600,
        },
      };
    });

    const nodesById = new Map(graph.nodes.map((node) => [node.id, node]));
    const edges = graph.edges.map((edge) => {
      const targetNode = nodesById.get(edge.targetId);
      const targetStyle = targetNode
        ? getOutcomeStyle(targetNode.outcome)
        : FALLBACK_STYLE;
      return {
        id: edgeKey(edge),
        source: edge.sourceId,
        target: edge.targetId,
        data: {
          sourceId: edge.sourceId,
          targetId: edge.targetId,
        },
        style: {
          stroke: targetStyle.strokeColor,
          lineWidth: 2.4,
          opacity: 0.92,
        },
      };
    });

    return {
      data: { nodes, edges },
      key: buildGraphDataKey(graph),
      nodeIds: graph.nodes.map((node) => node.id),
      edgeIds: graph.edges.map((edge) => edgeKey(edge)),
    };
  });

  highlightState = computed<TaskDependencyHighlightState | null>(() => {
    const selectedNodeId = this.selectedNodeIdInGraph();
    const hoveredNodeId = this.hoveredNodeId();
    const activeNodeId = selectedNodeId ?? hoveredNodeId;
    if (!activeNodeId) return null;

    const incomingEdgesByTarget = new Map<string, TaskDependencyEdge[]>();
    for (const edge of this.graph().edges) {
      const bucket = incomingEdgesByTarget.get(edge.targetId) ?? [];
      bucket.push(edge);
      incomingEdgesByTarget.set(edge.targetId, bucket);
    }

    const highlightedNodeIds = new Set<string>([activeNodeId]);
    const highlightedEdgeKeys = new Set<string>();
    const pendingNodeIds = [activeNodeId];

    while (pendingNodeIds.length > 0) {
      const currentNodeId = pendingNodeIds.pop();
      if (!currentNodeId) continue;

      for (const edge of incomingEdgesByTarget.get(currentNodeId) ?? []) {
        highlightedEdgeKeys.add(edgeKey(edge));
        if (highlightedNodeIds.has(edge.sourceId)) continue;
        highlightedNodeIds.add(edge.sourceId);
        pendingNodeIds.push(edge.sourceId);
      }
    }

    return {
      activeNodeId,
      mode: selectedNodeId ? "selected" : "hover",
      highlightedNodeIds,
      highlightedEdgeKeys,
    };
  });

  setHoveredNode(nodeId: string): void {
    if (this.selectedNodeIdInGraph()) return;
    this.hoveredNodeId.set(nodeId);
  }

  clearHoveredNode(nodeId?: string): void {
    if (this.selectedNodeIdInGraph()) return;
    if (nodeId && this.hoveredNodeId() !== nodeId) return;
    this.hoveredNodeId.set(null);
  }

  selectNode(nodeId: string): void {
    if (this.selectedNodeId() === nodeId) {
      this.clearSelectedNode();
      return;
    }
    this.selectedNodeId.set(nodeId);
    this.hoveredNodeId.set(null);
  }

  clearSelectedNode(): void {
    this.selectedNodeId.set(null);
    this.hoveredNodeId.set(null);
  }

  nodeHighlightState(nodeId: string): "idle" | "highlighted" | "dimmed" {
    const highlightState = this.highlightState();
    if (!highlightState) return "idle";
    return highlightState.highlightedNodeIds.has(nodeId)
      ? "highlighted"
      : "dimmed";
  }

  edgeHighlightState(
    edge: TaskDependencyEdge,
  ): "idle" | "highlighted" | "dimmed" {
    const highlightState = this.highlightState();
    if (!highlightState) return "idle";
    return highlightState.highlightedEdgeKeys.has(edgeKey(edge))
      ? "highlighted"
      : "dimmed";
  }

  private async renderG6Graph(
    container: HTMLElement,
    graphData: G6TaskGraphData,
  ): Promise<void> {
    const requestId = ++this.renderRequestId;
    if (!this.g6Graph || this.g6ContainerElement !== container) {
      this.destroyGraph();
      this.g6ContainerElement = container;
      this.g6Graph = new Graph(this.buildGraphOptions(container, graphData));
      this.bindGraphEvents(this.g6Graph);
    } else {
      this.g6Graph.setData(graphData.data);
    }

    const graph = this.g6Graph;
    await graph.render();
    if (requestId !== this.renderRequestId || this.g6Graph !== graph) return;

    this.lastRenderedGraphKey = graphData.key;
    if (this.pendingGraphKey === graphData.key) this.pendingGraphKey = null;
    await graph.fitView();
    this.syncG6ElementStates();
  }

  private buildGraphOptions(
    container: HTMLElement,
    graphData: G6TaskGraphData,
  ): GraphOptions {
    return {
      container,
      data: graphData.data,
      autoFit: "view",
      padding: FIT_VIEW_PADDING,
      zoomRange: [0.5, 2.5],
      animation: false,
      behaviors: ["drag-canvas", "zoom-canvas"],
      node: {
        type: "rect",
        state: {
          highlighted: {
            lineWidth: 3,
            shadowBlur: 8,
            shadowColor: "currentColor",
          },
          selected: {
            lineWidth: 4,
          },
          dimmed: {
            opacity: 0.28,
            labelOpacity: 0.36,
          },
        },
      },
      edge: {
        type: "polyline",
        style: {
          endArrow: false,
        },
        state: {
          highlighted: {
            lineWidth: 3,
            opacity: 1,
          },
          dimmed: {
            opacity: 0.12,
          },
        },
      },
    };
  }

  private bindGraphEvents(graph: Graph): void {
    graph.on(NodeEvent.POINTER_ENTER, (event: IElementEvent) => {
      const nodeId = getNodeIdFromEvent(event);
      if (nodeId) this.setHoveredNode(nodeId);
    });
    graph.on(NodeEvent.POINTER_LEAVE, (event: IElementEvent) => {
      const nodeId = getNodeIdFromEvent(event);
      if (nodeId) this.clearHoveredNode(nodeId);
    });
    graph.on(NodeEvent.CLICK, (event: IElementEvent) => {
      const nodeId = getNodeIdFromEvent(event);
      if (nodeId) this.selectNode(nodeId);
    });
    graph.on(CanvasEvent.CLICK, () => {
      this.clearSelectedNode();
    });
  }

  private syncG6ElementStates(): void {
    if (!this.g6Graph) return;

    const graphData = this.g6GraphData();
    const highlightState = this.highlightState();
    const states: Record<string, string[]> = {};

    for (const nodeId of graphData.nodeIds) {
      const state = this.nodeHighlightState(nodeId);
      if (state === "highlighted") {
        states[nodeId] =
          highlightState?.mode === "selected" &&
          highlightState.activeNodeId === nodeId
            ? ["highlighted", "selected"]
            : ["highlighted"];
      } else if (state === "dimmed") {
        states[nodeId] = ["dimmed"];
      } else {
        states[nodeId] = [];
      }
    }

    for (const edge of this.graph().edges) {
      const state = this.edgeHighlightState(edge);
      states[edgeKey(edge)] = state === "idle" ? [] : [state];
    }

    void this.g6Graph.setElementState(states);
  }

  private destroyGraph(): void {
    this.renderRequestId += 1;
    this.lastRenderedGraphKey = null;
    this.pendingGraphKey = null;
    this.g6ContainerElement = null;
    if (!this.g6Graph) return;
    this.g6Graph.destroy();
    this.g6Graph = null;
  }
}
