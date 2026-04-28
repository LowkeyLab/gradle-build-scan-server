import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  signal,
  type ElementRef,
  viewChild,
} from "@angular/core";
import {
  coordCenter,
  decrossTwoLayer,
  graphConnect,
  layeringSimplex,
  type Operator,
  sugiyama,
  tweakDirection,
} from "d3-dag";
import {
  select,
  zoom,
  zoomIdentity,
  type D3ZoomEvent,
  type ZoomBehavior,
} from "d3";

interface TaskEdge {
  node: {
    id: string;
    dependencies: string[];
    taskPath: string;
    outcome: string;
  };
}

interface TaskDependencyNode {
  id: string;
  label: string;
  displayLabel: string;
  outcome: string;
}

interface RenderedTaskDependencyNode extends TaskDependencyNode {
  layer: number;
  column: number;
  x: number;
  y: number;
  width: number;
  height: number;
  fillColor: string;
  strokeColor: string;
}

interface TaskDependencyEdge {
  sourceId: string;
  targetId: string;
}

interface RenderedTaskDependencyEdge extends TaskDependencyEdge {
  path: string;
  points: Point[];
  span: number;
  strokeColor: string;
  strokeWidth: number;
  strokeOpacity: number;
  strokeDasharray: string | null;
}

interface TaskDependencyLayout {
  nodes: RenderedTaskDependencyNode[];
  edges: RenderedTaskDependencyEdge[];
  width: number;
  height: number;
  zoomKey: string;
}

interface TaskDependencyHighlightState {
  activeNodeId: string;
  mode: "hover" | "selected";
  highlightedNodeIds: ReadonlySet<string>;
  highlightedEdgeKeys: ReadonlySet<string>;
}

interface Point {
  x: number;
  y: number;
}

interface DagConnectEdge extends TaskDependencyEdge {
  syntheticSingle?: boolean;
}

interface LegendNodeItem {
  label: string;
  fillColor: string;
  strokeColor: string;
}

const NODE_WIDTH = 220;
const NODE_HEIGHT = 52;
const COLUMN_GAP = 88;
const ROW_GAP = 28;
const HORIZONTAL_PADDING = 28;
const VERTICAL_PADDING = 28;
const NODE_SIZE = [NODE_WIDTH, NODE_HEIGHT] as const;
const NODE_GAP = [COLUMN_GAP, ROW_GAP] as const;
const POSITION_PRECISION = 1000;
const ZOOM_SCALE_EXTENT = [0.6, 2.5] as const;
const ZOOM_BOUNDS_PADDING = 96;
const CANVAS_RENDER_THRESHOLD = 500;
const CANVAS_NODE_WIDTH = 14;
const CANVAS_NODE_HEIGHT = 8;
const CANVAS_COLUMN_GAP = 18;
const CANVAS_ROW_GAP = 18;
const CANVAS_PADDING = 32;

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

const DAG_LAYOUT = sugiyama()
  .nodeSize(NODE_SIZE)
  .gap(NODE_GAP)
  .layering(layeringSimplex())
  .decross(decrossTwoLayer())
  .coord(coordCenter())
  .tweaks([tweakDirection("TB")]) as unknown as Operator<
  TaskDependencyNode,
  DagConnectEdge
>;

function truncateTaskLabel(label: string): string {
  if (label.length <= 28) return label;
  return `${label.slice(0, 25)}…`;
}

function compareLabels(
  left: { label: string; id: string },
  right: { label: string; id: string },
): number {
  const byLabel = left.label.localeCompare(right.label);
  return byLabel !== 0 ? byLabel : left.id.localeCompare(right.id);
}

function formatCoord(value: number): string {
  return `${Math.round(value * 10) / 10}`;
}

function buildOrthogonalPath(points: readonly Point[]): string {
  if (points.length === 0) return "";
  const start = points[0];
  if (!start) return "";
  if (points.length === 1) {
    return `M ${formatCoord(start.x)} ${formatCoord(start.y)}`;
  }

  const commands: string[] = [
    `M ${formatCoord(start.x)} ${formatCoord(start.y)}`,
  ];

  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const current = points[index];
    if (!previous || !current) continue;
    const midY = previous.y + (current.y - previous.y) / 2;

    commands.push(`L ${formatCoord(previous.x)} ${formatCoord(midY)}`);
    commands.push(`L ${formatCoord(current.x)} ${formatCoord(midY)}`);
    commands.push(`L ${formatCoord(current.x)} ${formatCoord(current.y)}`);
  }

  return commands.join(" ");
}

function anchorEdgePoints(
  points: readonly Point[],
  source: RenderedTaskDependencyNode,
  target: RenderedTaskDependencyNode,
): Point[] {
  const sourceAnchor = {
    x: source.x + source.width / 2,
    y: source.y + source.height,
  };
  const targetAnchor = {
    x: target.x + target.width / 2,
    y: target.y,
  };

  if (points.length === 0) {
    return [sourceAnchor, targetAnchor];
  }

  if (points.length === 1) {
    return [sourceAnchor, targetAnchor];
  }

  return [sourceAnchor, ...points.slice(1, -1), targetAnchor];
}

function positionKey(value: number): string {
  return `${Math.round(value * POSITION_PRECISION) / POSITION_PRECISION}`;
}

function buildZoomKey(layout: {
  nodes: RenderedTaskDependencyNode[];
  edges: RenderedTaskDependencyEdge[];
  width: number;
  height: number;
}): string {
  const nodeKey = layout.nodes
    .map(
      (node) =>
        `${node.id}:${node.layer}:${node.column}:${formatCoord(node.x)}:${formatCoord(node.y)}`,
    )
    .join("|");
  const edgeKey = layout.edges
    .map((edge) => `${edge.sourceId}:${edge.targetId}:${edge.span}`)
    .join("|");
  return `${layout.width}x${layout.height}|${nodeKey}|${edgeKey}`;
}

function edgeKey(edge: TaskDependencyEdge): string {
  return `${edge.sourceId}:${edge.targetId}`;
}

function buildCanvasOverviewLayout(graph: {
  nodes: TaskDependencyNode[];
  edges: TaskDependencyEdge[];
}): TaskDependencyLayout {
  const columns = Math.max(1, Math.ceil(Math.sqrt(graph.nodes.length * 1.6)));
  const positionedNodes = graph.nodes.map((node, index) => {
    const layer = Math.floor(index / columns);
    const column = index % columns;
    return {
      ...node,
      layer,
      column,
      x: CANVAS_PADDING + column * CANVAS_COLUMN_GAP,
      y: CANVAS_PADDING + layer * CANVAS_ROW_GAP,
      width: CANVAS_NODE_WIDTH,
      height: CANVAS_NODE_HEIGHT,
      ...(OUTCOME_STYLES[node.outcome] ?? FALLBACK_STYLE),
    };
  });

  const positionedById = new Map(
    positionedNodes.map((node) => [node.id, node]),
  );
  const edges = graph.edges
    .map<RenderedTaskDependencyEdge | null>((edge) => {
      const source = positionedById.get(edge.sourceId);
      const target = positionedById.get(edge.targetId);
      if (!source || !target) return null;
      const points = [
        {
          x: source.x + source.width / 2,
          y: source.y + source.height / 2,
        },
        {
          x: target.x + target.width / 2,
          y: target.y + target.height / 2,
        },
      ];
      const span = Math.max(Math.abs(target.layer - source.layer), 1);
      return {
        ...edge,
        points,
        span,
        path: "",
        strokeColor: target.strokeColor,
        strokeWidth: 1,
        strokeOpacity: 0.3,
        strokeDasharray: null,
      };
    })
    .filter((edge): edge is RenderedTaskDependencyEdge => !!edge);

  const rows = Math.max(1, Math.ceil(graph.nodes.length / columns));
  const width =
    CANVAS_PADDING * 2 +
    CANVAS_NODE_WIDTH +
    Math.max(0, columns - 1) * CANVAS_COLUMN_GAP;
  const height =
    CANVAS_PADDING * 2 +
    CANVAS_NODE_HEIGHT +
    Math.max(0, rows - 1) * CANVAS_ROW_GAP;

  return {
    nodes: positionedNodes,
    edges,
    width,
    height,
    zoomKey: `canvas:${graph.nodes.length}:${graph.edges.length}:${width}x${height}`,
  };
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
              @if (usesCanvasRenderer()) {
                Canvas overview of task prerequisites.
              } @else {
                Static graph of task prerequisites.
              }
            </p>
          </div>
          @if (layout().nodes.length > 0) {
            <div class="text-xs opacity-60">
              {{ layout().nodes.length }} nodes ·
              {{ layout().edges.length }} edges
            </div>
          }
        </div>

        @if (layout().nodes.length > 0) {
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
            @if (usesCanvasRenderer()) {
              <canvas
                #graphCanvas
                data-testid="task-dependency-canvas"
                [attr.width]="layout().width"
                [attr.height]="layout().height"
                class="block h-auto w-full text-base-content"
                aria-label="Large task dependency graph rendered as a canvas overview"
              ></canvas>
            } @else {
              <svg
                #graphSvg
                data-testid="task-dependency-graph"
                [attr.viewBox]="'0 0 ' + layout().width + ' ' + layout().height"
                [attr.width]="layout().width"
                [attr.height]="layout().height"
                preserveAspectRatio="xMidYMin meet"
                class="block h-auto w-full cursor-grab touch-none select-none text-base-content active:cursor-grabbing"
                style="touch-action: none"
                (click)="clearSelectedNodeFromBackground($event)"
              >
                <g #graphViewport data-testid="task-dependency-viewport">
                  @for (edge of layout().edges; track edgeKey(edge)) {
                    <g
                      [attr.data-edge-id]="edgeKey(edge)"
                      [attr.data-highlight-state]="edgeHighlightState(edge)"
                    >
                      <path
                        [attr.d]="edge.path"
                        fill="none"
                        stroke="currentColor"
                        [attr.stroke-width]="edge.strokeWidth + 3"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        vector-effect="non-scaling-stroke"
                        [attr.opacity]="edgeBackdropOpacity(edge)"
                      ></path>
                      <path
                        data-testid="dependency-edge"
                        [attr.d]="edge.path"
                        fill="none"
                        [attr.color]="edge.strokeColor"
                        stroke="currentColor"
                        [attr.stroke-width]="edge.strokeWidth"
                        [attr.stroke-opacity]="edgeStrokeOpacity(edge)"
                        [attr.stroke-dasharray]="edge.strokeDasharray"
                        [attr.data-edge-id]="edgeKey(edge)"
                        [attr.data-edge-span]="edge.span"
                        [attr.data-highlight-state]="edgeHighlightState(edge)"
                        [attr.data-point-count]="edge.points.length"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        vector-effect="non-scaling-stroke"
                      ></path>
                    </g>
                  }

                  @for (node of layout().nodes; track node.id) {
                    <g
                      data-testid="dependency-node"
                      [attr.data-node-id]="node.id"
                      [attr.data-highlight-state]="nodeHighlightState(node.id)"
                      [attr.transform]="
                        'translate(' + node.x + ' ' + node.y + ')'
                      "
                      [attr.opacity]="nodeOpacity(node.id)"
                      class="text-base-content transition-opacity"
                      (mouseenter)="setHoveredNode(node.id)"
                      (mouseleave)="clearHoveredNode(node.id)"
                      (click)="selectNode(node.id); $event.stopPropagation()"
                    >
                      <title>{{ node.label }}</title>
                      <rect
                        x="0"
                        y="0"
                        [attr.width]="node.width"
                        [attr.height]="node.height"
                        rx="12"
                        [attr.fill]="node.fillColor"
                        [attr.stroke]="node.strokeColor"
                        stroke-width="2"
                      ></rect>
                      <text
                        data-testid="dependency-node-label"
                        x="16"
                        y="29"
                        fill="currentColor"
                        font-size="14"
                        font-weight="600"
                      >
                        {{ node.displayLabel }}
                      </text>
                    </g>
                  }
                </g>
              </svg>
            }
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

  private graphSvg = viewChild<ElementRef<SVGSVGElement>>("graphSvg");
  private graphViewport = viewChild<ElementRef<SVGGElement>>("graphViewport");
  private graphCanvas = viewChild<ElementRef<HTMLCanvasElement>>("graphCanvas");
  private zoomBehavior: ZoomBehavior<SVGSVGElement, unknown> | null = null;
  private zoomSvgElement: SVGSVGElement | null = null;
  private lastZoomKey: string | null = null;
  private hoveredNodeId = signal<string | null>(null);
  private selectedNodeId = signal<string | null>(null);
  private selectedNodeIdInLayout = computed(() => {
    const selectedNodeId = this.selectedNodeId();
    if (!selectedNodeId) return null;
    return this.layout().nodes.some((node) => node.id === selectedNodeId)
      ? selectedNodeId
      : null;
  });

  constructor() {
    afterRenderEffect(() => {
      if (this.selectedNodeId() && !this.selectedNodeIdInLayout()) {
        this.selectedNodeId.set(null);
      }

      const svgRef = this.graphSvg();
      const viewportRef = this.graphViewport();
      const canvasRef = this.graphCanvas();
      const layout = this.layout();
      if (layout.nodes.length === 0) return;

      if (this.usesCanvasRenderer()) {
        if (canvasRef) {
          this.drawCanvasGraph(canvasRef.nativeElement, layout);
        }
        return;
      }

      if (!svgRef || !viewportRef) return;

      this.syncViewportZoom(
        svgRef.nativeElement,
        viewportRef.nativeElement,
        layout,
      );
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

  usesCanvasRenderer = computed(
    () => this.graph().nodes.length > CANVAS_RENDER_THRESHOLD,
  );

  layout = computed<TaskDependencyLayout>(() => {
    const graph = this.graph();
    if (graph.nodes.length === 0) {
      return { nodes: [], edges: [], width: 0, height: 0, zoomKey: "empty" };
    }

    if (graph.nodes.length > CANVAS_RENDER_THRESHOLD) {
      return buildCanvasOverviewLayout(graph);
    }

    const nodesById = new Map(graph.nodes.map((node) => [node.id, node]));
    const connectedNodeIds = new Set<string>();
    for (const edge of graph.edges) {
      connectedNodeIds.add(edge.sourceId);
      connectedNodeIds.add(edge.targetId);
    }

    const connectEdges: DagConnectEdge[] = [
      ...graph.edges,
      ...graph.nodes
        .filter((node) => !connectedNodeIds.has(node.id))
        .map((node) => ({
          sourceId: node.id,
          targetId: node.id,
          syntheticSingle: true,
        })),
    ];

    const dag = graphConnect()
      .sourceId((edge: DagConnectEdge) => edge.sourceId)
      .targetId((edge: DagConnectEdge) => edge.targetId)
      .nodeDatum((id: string) => {
        const node = nodesById.get(id);
        if (!node) {
          throw new Error(`Missing task dependency node for id ${id}`);
        }
        return node;
      })
      .single(true)(connectEdges);

    const { width: layoutWidth, height: layoutHeight } = DAG_LAYOUT(dag);
    const rawNodes = [...dag.nodes()];
    const layerKeys = [...new Set(rawNodes.map((node) => positionKey(node.y)))];
    const sortedLayerKeys = layerKeys.sort(
      (left, right) => Number(left) - Number(right),
    );
    const layerByKey = new Map(
      sortedLayerKeys.map((key, index) => [key, index]),
    );

    const positionedNodes = rawNodes.map((node) => {
      const data = node.data;
      const layer = layerByKey.get(positionKey(node.y)) ?? 0;
      const centerX = node.x + HORIZONTAL_PADDING;
      const centerY = node.y + VERTICAL_PADDING;

      return {
        ...data,
        layer,
        column: 0,
        x: centerX - NODE_WIDTH / 2,
        y: centerY - NODE_HEIGHT / 2,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        ...(OUTCOME_STYLES[data.outcome] ?? FALLBACK_STYLE),
      };
    });

    const nodesByLayer = new Map<number, RenderedTaskDependencyNode[]>();
    for (const node of positionedNodes) {
      const bucket = nodesByLayer.get(node.layer) ?? [];
      bucket.push(node);
      nodesByLayer.set(node.layer, bucket);
    }
    for (const nodes of nodesByLayer.values()) {
      nodes
        .sort((left, right) => {
          const positionDelta = left.x - right.x;
          return positionDelta !== 0
            ? positionDelta
            : compareLabels(left, right);
        })
        .forEach((node, column) => {
          node.column = column;
        });
    }

    positionedNodes.sort((left, right) => {
      const layerDelta = left.layer - right.layer;
      if (layerDelta !== 0) return layerDelta;
      const columnDelta = left.column - right.column;
      return columnDelta !== 0 ? columnDelta : compareLabels(left, right);
    });

    const positionedById = new Map(
      positionedNodes.map((node) => [node.id, node]),
    );
    const edges = [...dag.links()]
      .filter((link) => link.data.sourceId !== link.data.targetId)
      .map<RenderedTaskDependencyEdge | null>((link) => {
        const source = positionedById.get(link.source.data.id);
        const target = positionedById.get(link.target.data.id);
        if (!source || !target) return null;

        const span = Math.max(target.layer - source.layer, 1);
        const routedPoints = link.points.map(([x, y]) => ({
          x: x + HORIZONTAL_PADDING,
          y: y + VERTICAL_PADDING,
        }));
        const points = anchorEdgePoints(routedPoints, source, target);
        const strokeColor = target.strokeColor;

        return {
          sourceId: source.id,
          targetId: target.id,
          points,
          span,
          path: buildOrthogonalPath(points),
          strokeColor,
          strokeWidth: span > 1 ? 2.1 : 2.4,
          strokeOpacity: span > 1 ? 0.82 : 0.92,
          strokeDasharray: null as string | null,
        };
      })
      .filter((edge): edge is RenderedTaskDependencyEdge => !!edge)
      .sort((left, right) => {
        const spanDelta = left.span - right.span;
        if (spanDelta !== 0) return spanDelta;
        return `${left.sourceId}:${left.targetId}`.localeCompare(
          `${right.sourceId}:${right.targetId}`,
        );
      });

    const width = Math.max(
      Math.ceil(layoutWidth + HORIZONTAL_PADDING * 2),
      NODE_WIDTH + HORIZONTAL_PADDING * 2,
    );
    const height = Math.max(
      Math.ceil(layoutHeight + VERTICAL_PADDING * 2),
      NODE_HEIGHT + VERTICAL_PADDING * 2,
    );

    return {
      nodes: positionedNodes,
      edges,
      width,
      height,
      zoomKey: buildZoomKey({ nodes: positionedNodes, edges, width, height }),
    };
  });

  highlightState = computed<TaskDependencyHighlightState | null>(() => {
    const selectedNodeId = this.selectedNodeIdInLayout();
    const hoveredNodeId = this.hoveredNodeId();
    const activeNodeId = selectedNodeId ?? hoveredNodeId;
    if (!activeNodeId) return null;

    const incomingEdgesByTarget = new Map<
      string,
      RenderedTaskDependencyEdge[]
    >();
    for (const edge of this.layout().edges) {
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
    if (this.selectedNodeIdInLayout()) return;
    this.hoveredNodeId.set(nodeId);
  }

  clearHoveredNode(nodeId?: string): void {
    if (this.selectedNodeIdInLayout()) return;
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

  clearSelectedNodeFromBackground(event: MouseEvent): void {
    if (event.target !== event.currentTarget) return;
    this.clearSelectedNode();
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

  nodeOpacity(nodeId: string): number {
    return this.nodeHighlightState(nodeId) === "dimmed" ? 0.28 : 1;
  }

  edgeBackdropOpacity(edge: TaskDependencyEdge): number {
    switch (this.edgeHighlightState(edge)) {
      case "highlighted":
        return 0.2;
      case "dimmed":
        return 0.04;
      default:
        return 0.16;
    }
  }

  edgeStrokeOpacity(edge: RenderedTaskDependencyEdge): number {
    switch (this.edgeHighlightState(edge)) {
      case "highlighted":
        return Math.min(edge.strokeOpacity + 0.08, 1);
      case "dimmed":
        return 0.12;
      default:
        return edge.strokeOpacity;
    }
  }

  private drawCanvasGraph(
    canvasElement: HTMLCanvasElement,
    layout: TaskDependencyLayout,
  ): void {
    let context: CanvasRenderingContext2D | null = null;
    try {
      context = canvasElement.getContext("2d");
    } catch {
      return;
    }
    if (!context) return;

    context.clearRect(0, 0, layout.width, layout.height);
    context.lineCap = "round";
    context.lineJoin = "round";
    context.lineWidth = 0.8;

    for (const edge of layout.edges) {
      const [source, target] = edge.points;
      if (!source || !target) continue;
      context.globalAlpha = this.edgeStrokeOpacity(edge);
      context.strokeStyle = edge.strokeColor;
      context.beginPath();
      context.moveTo(source.x, source.y);
      context.lineTo(target.x, target.y);
      context.stroke();
    }

    context.globalAlpha = 1;
    for (const node of layout.nodes) {
      context.globalAlpha = this.nodeOpacity(node.id);
      context.fillStyle = node.fillColor;
      context.strokeStyle = node.strokeColor;
      context.lineWidth = 1.2;
      context.fillRect(node.x, node.y, node.width, node.height);
      context.strokeRect(node.x, node.y, node.width, node.height);
    }
    context.globalAlpha = 1;
  }

  private syncViewportZoom(
    svgElement: SVGSVGElement,
    viewportElement: SVGGElement,
    layout: TaskDependencyLayout,
  ): void {
    const svgSelection = select(svgElement);
    const viewportSelection = select(viewportElement);
    const needsNewBinding =
      this.zoomBehavior === null || this.zoomSvgElement !== svgElement;

    if (needsNewBinding) {
      this.zoomBehavior = zoom<SVGSVGElement, unknown>()
        .scaleExtent(ZOOM_SCALE_EXTENT)
        .on("zoom", (event: D3ZoomEvent<SVGSVGElement, unknown>) => {
          viewportSelection.attr("transform", event.transform.toString());
        });
      this.zoomSvgElement = svgElement;
    }

    const zoomBehavior = this.zoomBehavior;
    if (!zoomBehavior) return;

    zoomBehavior
      .extent([
        [0, 0],
        [layout.width, layout.height],
      ])
      .translateExtent([
        [-ZOOM_BOUNDS_PADDING, -ZOOM_BOUNDS_PADDING],
        [
          layout.width + ZOOM_BOUNDS_PADDING,
          layout.height + ZOOM_BOUNDS_PADDING,
        ],
      ]);

    svgSelection.call(zoomBehavior).on("dblclick.zoom", null);

    if (needsNewBinding || this.lastZoomKey !== layout.zoomKey) {
      svgSelection.call(zoomBehavior.transform, zoomIdentity);
      this.lastZoomKey = layout.zoomKey;
    }
  }
}
