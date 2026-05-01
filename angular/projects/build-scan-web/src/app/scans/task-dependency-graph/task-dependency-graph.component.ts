import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  type ElementRef,
  inject,
  input,
  signal,
  viewChild,
} from "@angular/core";
import {
  CanvasEvent,
  Graph,
  type GraphData,
  type GraphOptions,
  type IElementEvent,
  NodeEvent,
} from "@antv/g6";

interface TaskEdge {
  node: {
    id: string;
    dependencies: string[];
    taskPath: string;
    outcome: string;
    durationMs: number | null;
  };
}

interface TaskDependencyNode {
  id: string;
  label: string;
  displayLabel: string;
  outcome: string;
  durationMs: number | null;
}

interface TaskDependencyEdge {
  sourceId: string;
  targetId: string;
}

interface TaskDependencyHighlightState {
  activeNodeId: string;
  mode: "selected";
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

interface TaskGraphLayoutOptions extends Record<string, unknown> {
  type: "antv-dagre";
  rankdir: "LR";
  align: "DL";
  nodesep: number;
  ranksep: number;
}

const NODE_BASE_SIZE = 72;
const NODE_INCOMING_EDGE_SIZE_STEP = 14;
const NODE_MAX_SIZE = 128;
const NODE_VERTICAL_GAP = 56;
const NODE_RANK_GAP = 96;
const FIT_VIEW_PADDING = 32;

const TASK_GRAPH_LAYOUT: TaskGraphLayoutOptions = {
  type: "antv-dagre",
  rankdir: "LR",
  align: "DL",
  nodesep: NODE_VERTICAL_GAP,
  ranksep: NODE_RANK_GAP,
};

const SUCCESS_STYLE = {
  fillColor: "oklch(86% 0.12 150)",
  strokeColor: "oklch(54% 0.16 150)",
};

const UP_TO_DATE_STYLE = {
  fillColor: "oklch(90% 0.08 150)",
  strokeColor: "oklch(50% 0.1 150)",
};

const FROM_CACHE_STYLE = {
  fillColor: "oklch(86% 0.12 230)",
  strokeColor: "oklch(55% 0.15 230)",
};

const FAILED_STYLE = {
  fillColor: "oklch(85% 0.12 25)",
  strokeColor: "oklch(56% 0.2 25)",
};

const SKIPPED_STYLE = {
  fillColor: "oklch(88% 0.12 75)",
  strokeColor: "oklch(55% 0.15 75)",
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
  fillColor: "oklch(87% 0.04 260)",
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

function buildIncomingDependencyCounts(
  edges: TaskDependencyEdge[],
): Map<string, number> {
  const incomingCounts = new Map<string, number>();
  for (const edge of edges) {
    incomingCounts.set(
      edge.targetId,
      (incomingCounts.get(edge.targetId) ?? 0) + 1,
    );
  }
  return incomingCounts;
}

function getNodeSize(incomingEdgeCount: number): number {
  return Math.min(
    NODE_MAX_SIZE,
    NODE_BASE_SIZE + NODE_INCOMING_EDGE_SIZE_STEP * incomingEdgeCount,
  );
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
  const incomingCounts = buildIncomingDependencyCounts(graph.edges);
  const nodeKey = graph.nodes
    .map(
      (node) =>
        `${node.id}:${node.label}:${node.outcome}:${node.durationMs ?? ""}:${incomingCounts.get(node.id) ?? 0}`,
    )
    .join("|");
  const edgeKey = graph.edges
    .map((edge) => `${edge.sourceId}:${edge.targetId}`)
    .join("|");
  return `${nodeKey}::${edgeKey}`;
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
              Tasks are arranged as a layered dependency graph.
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
                  class="inline-block h-3.5 w-3.5 rounded-full border-2"
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
              class="task-dependency-g6 h-[36rem] min-h-[32rem] w-full touch-none select-none text-base-content"
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
  private hasSyncedActiveElementStates = false;
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

    const nodes = [...tasks.values()]
      .sort((a, b) => a.taskPath.localeCompare(b.taskPath))
      .map((task) => ({
        id: task.id,
        label: task.taskPath,
        displayLabel: truncateTaskLabel(task.taskPath),
        outcome: task.outcome,
        durationMs: task.durationMs,
      }));

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
    const incomingCounts = buildIncomingDependencyCounts(graph.edges);
    const nodes = graph.nodes.map((node) => {
      const style = getOutcomeStyle(node.outcome);
      const incomingDependencyCount = incomingCounts.get(node.id) ?? 0;
      return {
        id: node.id,
        data: {
          label: node.label,
          displayLabel: node.displayLabel,
          outcome: node.outcome,
          incomingDependencyCount,
        },
        style: {
          size: getNodeSize(incomingDependencyCount),
          fill: style.fillColor,
          stroke: style.strokeColor,
          lineWidth: 2,
          labelText: node.displayLabel,
          labelFill: "currentColor",
          labelFontSize: 16,
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
    const activeNodeId = this.selectedNodeIdInGraph();
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
      mode: "selected",
      highlightedNodeIds,
      highlightedEdgeKeys,
    };
  });

  selectNode(nodeId: string): void {
    if (this.selectedNodeId() === nodeId) {
      this.clearSelectedNode();
      return;
    }
    this.selectedNodeId.set(nodeId);
  }

  clearSelectedNode(): void {
    this.selectedNodeId.set(null);
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
      this.g6Graph.setLayout(TASK_GRAPH_LAYOUT);
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
      layout: TASK_GRAPH_LAYOUT,
      autoFit: "view",
      padding: FIT_VIEW_PADDING,
      zoomRange: [0.5, 2.5],
      animation: false,
      behaviors: ["drag-canvas", "zoom-canvas"],
      node: {
        type: "circle",
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
        type: "cubic-horizontal",
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
    if (!highlightState) {
      if (!this.hasSyncedActiveElementStates) return;
      this.hasSyncedActiveElementStates = false;
      const clearedStates: Record<string, string[]> = {};
      for (const nodeId of graphData.nodeIds) {
        clearedStates[nodeId] = [];
      }
      for (const edgeId of graphData.edgeIds) {
        clearedStates[edgeId] = [];
      }
      void this.g6Graph.setElementState(clearedStates);
      return;
    }
    this.hasSyncedActiveElementStates = true;

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
    this.hasSyncedActiveElementStates = false;
    if (!this.g6Graph) return;
    this.g6Graph.destroy();
    this.g6Graph = null;
  }
}
