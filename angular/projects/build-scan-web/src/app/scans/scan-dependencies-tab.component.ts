import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  input,
  signal,
} from "@angular/core";
import { takeUntilDestroyed, toObservable } from "@angular/core/rxjs-interop";
import type { OnInit } from "@angular/core";
import { Apollo, gql } from "apollo-angular";
import { filter, map, switchMap, tap } from "rxjs";

interface ConfigurationDependencySummary {
  id: string;
  displayName: string;
  details: string[];
}

interface ConfigurationDependencyNode {
  id: string;
  label: string;
}

interface ConfigurationDependencyEdge {
  sourceId: string;
  targetId: string;
}

interface ConfigurationDependencyGraph {
  nodes: ConfigurationDependencyNode[];
  edges: ConfigurationDependencyEdge[];
}

interface ConfigurationDependencyListData {
  buildScan: {
    id: string;
    configurationDependencies: ConfigurationDependencySummary[];
  } | null;
}

interface ConfigurationDependencyListVariables {
  id: string;
}

interface ConfigurationDependencyGraphData {
  buildScan: {
    id: string;
    configurationDependencyGraph: ConfigurationDependencyGraph | null;
  } | null;
}

interface ConfigurationDependencyGraphVariables {
  id: string;
  configurationId: string;
}

interface RenderedNode extends ConfigurationDependencyNode {
  x: number;
  y: number;
}

function isConfigurationDependencySummary(
  configuration: Partial<ConfigurationDependencySummary> | null | undefined,
): configuration is ConfigurationDependencySummary {
  return (
    !!configuration &&
    typeof configuration.id === "string" &&
    typeof configuration.displayName === "string" &&
    Array.isArray(configuration.details)
  );
}

const GET_SCAN_CONFIGURATION_DEPENDENCIES = gql`
  query GetScanConfigurationDependencies($id: ID!) {
    buildScan(id: $id) {
      id
      configurationDependencies {
        id
        displayName
        details
      }
    }
  }
`;

const GET_SCAN_CONFIGURATION_DEPENDENCY_GRAPH = gql`
  query GetScanConfigurationDependencyGraph(
    $id: ID!
    $configurationId: String!
  ) {
    buildScan(id: $id) {
      id
      configurationDependencyGraph(configurationId: $configurationId) {
        nodes {
          id
          label
        }
        edges {
          sourceId
          targetId
        }
      }
    }
  }
`;

@Component({
  selector: "app-scan-dependencies-tab",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="space-y-6">
      <h2 class="text-2xl font-bold">Dependencies</h2>

      @if (loadingConfigurations()) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          Loading configurations…
        </div>
      } @else if (configurations().length === 0) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          No configuration dependencies recorded for this scan.
        </div>
      } @else {
        <div class="grid gap-6 lg:grid-cols-[320px_minmax(0,1fr)]">
          <aside
            class="rounded-box border border-base-300 bg-base-200 p-3 lg:max-h-[calc(100vh-12rem)] lg:overflow-y-auto"
          >
            <div class="mb-3">
              <h3 class="font-semibold">Configurations</h3>
              <p class="text-xs opacity-60">
                Select a configuration to load its dependency graph.
              </p>
            </div>

            <div class="space-y-2">
              @for (configuration of configurations(); track configuration.id) {
                <button
                  type="button"
                  class="w-full rounded-md border px-3 py-2 text-left transition-colors"
                  [class.border-primary]="
                    selectedConfigurationId() === configuration.id
                  "
                  [class.bg-primary/10]="
                    selectedConfigurationId() === configuration.id
                  "
                  [class.border-base-300]="
                    selectedConfigurationId() !== configuration.id
                  "
                  [class.hover:bg-base-300]="
                    selectedConfigurationId() !== configuration.id
                  "
                  (click)="selectConfiguration(configuration)"
                >
                  <div class="font-medium break-words">
                    {{ configuration.displayName }}
                  </div>
                  @if (configuration.details.length > 0) {
                    <div class="mt-1 text-xs opacity-60 break-words">
                      {{ configuration.details.join(" · ") }}
                    </div>
                  }
                </button>
              }
            </div>
          </aside>

          <section class="space-y-4">
            @if (!selectedConfiguration()) {
              <div
                class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
              >
                Select a configuration from the list to fetch and render its
                dependencies.
              </div>
            } @else if (loadingGraph()) {
              <div
                class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
              >
                Loading dependency graph for
                {{ selectedConfiguration()!.displayName }}…
              </div>
            } @else if (selectedGraph(); as graph) {
              <div class="card bg-base-200">
                <div class="card-body p-4">
                  <div class="mb-3 flex items-start justify-between gap-4">
                    <div>
                      <h3 class="font-semibold">
                        {{ selectedConfiguration()!.displayName }}
                      </h3>
                      @if (selectedConfiguration()!.details.length > 0) {
                        <p class="text-xs opacity-60 break-words">
                          {{ selectedConfiguration()!.details.join(" · ") }}
                        </p>
                      }
                    </div>
                    <div class="text-xs opacity-60">
                      {{ graph.nodes.length }} nodes · {{ graph.edges.length }}
                      edges
                    </div>
                  </div>

                  @if (graph.nodes.length > 0) {
                    <div
                      class="overflow-x-auto rounded-box border border-base-300 bg-base-100/40 p-3"
                    >
                      <svg
                        data-testid="configuration-dependency-graph"
                        [attr.viewBox]="
                          '0 0 ' +
                          graphLayout().width +
                          ' ' +
                          graphLayout().height
                        "
                        [attr.width]="graphLayout().width"
                        [attr.height]="graphLayout().height"
                        class="h-auto min-w-full text-base-content"
                      >
                        <defs>
                          <marker
                            id="configuration-dependency-arrow"
                            viewBox="0 0 10 10"
                            refX="9"
                            refY="5"
                            markerWidth="6"
                            markerHeight="6"
                            orient="auto-start-reverse"
                          >
                            <path
                              d="M 0 0 L 10 5 L 0 10 z"
                              fill="currentColor"
                            ></path>
                          </marker>
                        </defs>

                        @for (
                          edge of graphLayout().edges;
                          track edge.sourceId + ":" + edge.targetId
                        ) {
                          <path
                            [attr.d]="edge.path"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            opacity="0.35"
                            marker-end="url(#configuration-dependency-arrow)"
                          ></path>
                        }

                        @for (node of graphLayout().nodes; track node.id) {
                          <g
                            [attr.transform]="
                              'translate(' + node.x + ' ' + node.y + ')'
                            "
                          >
                            <title>{{ node.label }}</title>
                            <rect
                              x="0"
                              y="0"
                              width="260"
                              height="52"
                              rx="12"
                              fill="oklch(55% 0.04 260 / 0.12)"
                              stroke="oklch(55% 0.04 260)"
                              stroke-width="2"
                            ></rect>
                            <text
                              x="16"
                              y="29"
                              fill="currentColor"
                              font-size="14"
                              font-weight="600"
                            >
                              {{ truncateLabel(node.label) }}
                            </text>
                          </g>
                        }
                      </svg>
                    </div>
                  } @else {
                    <p class="text-sm opacity-60">
                      No configuration dependency graph available.
                    </p>
                  }
                </div>
              </div>
            } @else {
              <div
                class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
              >
                No configuration dependency graph available.
              </div>
            }
          </section>
        </div>
      }
    </div>
  `,
})
export class ScanDependenciesTabComponent implements OnInit {
  scanId = input.required<string>();

  private apollo = inject(Apollo);
  private destroyRef = inject(DestroyRef);
  private scanId$ = toObservable(this.scanId);
  private graphCache = new Map<string, ConfigurationDependencyGraph | null>();

  configurations = signal<ConfigurationDependencySummary[]>([]);
  selectedConfigurationId = signal<string | null>(null);
  selectedGraph = signal<ConfigurationDependencyGraph | null>(null);
  loadingConfigurations = signal(true);
  loadingGraph = signal(false);

  selectedConfiguration = computed(() => {
    const selectedId = this.selectedConfigurationId();
    if (!selectedId) return null;
    return (
      this.configurations().find(
        (configuration) => configuration.id === selectedId,
      ) ?? null
    );
  });

  graphLayout = computed(() => {
    const graph = this.selectedGraph();
    if (!graph) {
      return {
        nodes: [] as RenderedNode[],
        edges: [] as Array<{
          sourceId: string;
          targetId: string;
          path: string;
        }>,
        width: 0,
        height: 0,
      };
    }

    const incoming = new Map<string, number>();
    for (const node of graph.nodes) {
      incoming.set(node.id, 0);
    }
    for (const edge of graph.edges) {
      incoming.set(edge.targetId, (incoming.get(edge.targetId) ?? 0) + 1);
    }

    const roots = graph.nodes.filter(
      (node) => (incoming.get(node.id) ?? 0) === 0,
    );
    const rootIds = new Set(roots.map((node) => node.id));
    const children = graph.nodes.filter((node) => !rootIds.has(node.id));
    const leftColumn = roots.length > 0 ? roots : graph.nodes.slice(0, 1);
    const rightColumn = children.length > 0 ? children : graph.nodes.slice(1);
    const positionedNodes = new Map<string, RenderedNode>();

    for (const [index, node] of leftColumn.entries()) {
      positionedNodes.set(node.id, {
        id: node.id,
        label: node.label,
        x: 24,
        y: 24 + index * 80,
      });
    }
    for (const [index, node] of rightColumn.entries()) {
      positionedNodes.set(node.id, {
        id: node.id,
        label: node.label,
        x: 360,
        y: 24 + index * 80,
      });
    }

    for (const node of graph.nodes) {
      if (!positionedNodes.has(node.id)) {
        positionedNodes.set(node.id, {
          id: node.id,
          label: node.label,
          x: 360,
          y: 24 + positionedNodes.size * 80,
        });
      }
    }

    const renderedNodes = Array.from(positionedNodes.values());
    const renderedEdges = graph.edges
      .map((edge) => {
        const source = positionedNodes.get(edge.sourceId);
        const target = positionedNodes.get(edge.targetId);
        if (!source || !target) return null;
        const startX = source.x + 260;
        const startY = source.y + 26;
        const endX = target.x;
        const endY = target.y + 26;
        const controlX = startX + (endX - startX) / 2;
        return {
          sourceId: edge.sourceId,
          targetId: edge.targetId,
          path: `M ${startX} ${startY} C ${controlX} ${startY}, ${controlX} ${endY}, ${endX} ${endY}`,
        };
      })
      .filter(
        (edge): edge is { sourceId: string; targetId: string; path: string } =>
          edge !== null,
      );

    return {
      nodes: renderedNodes,
      edges: renderedEdges,
      width: 660,
      height: Math.max(120, renderedNodes.length * 80),
    };
  });

  ngOnInit() {
    this.scanId$
      .pipe(
        switchMap((id) => {
          this.graphCache.clear();
          this.configurations.set([]);
          this.selectedConfigurationId.set(null);
          this.selectedGraph.set(null);
          this.loadingConfigurations.set(true);
          this.loadingGraph.set(false);

          return this.apollo
            .watchQuery<
              ConfigurationDependencyListData,
              ConfigurationDependencyListVariables
            >({
              query: GET_SCAN_CONFIGURATION_DEPENDENCIES,
              variables: { id },
              errorPolicy: "all",
            })
            .valueChanges.pipe(
              filter((result) => !!result.data),
              map(
                (result) =>
                  result.data?.buildScan?.configurationDependencies ?? [],
              ),
              map((configurations) =>
                configurations.filter(isConfigurationDependencySummary),
              ),
              tap((configurations) => {
                this.configurations.set(configurations);
                this.loadingConfigurations.set(false);
              }),
            );
        }),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe();
  }

  selectConfiguration(configuration: ConfigurationDependencySummary) {
    this.selectedConfigurationId.set(configuration.id);

    if (this.graphCache.has(configuration.id)) {
      this.selectedGraph.set(this.graphCache.get(configuration.id) ?? null);
      this.loadingGraph.set(false);
      return;
    }

    this.loadingGraph.set(true);
    this.selectedGraph.set(null);

    this.apollo
      .query<
        ConfigurationDependencyGraphData,
        ConfigurationDependencyGraphVariables
      >({
        query: GET_SCAN_CONFIGURATION_DEPENDENCY_GRAPH,
        variables: {
          id: this.scanId(),
          configurationId: configuration.id,
        },
        errorPolicy: "all",
        fetchPolicy: "no-cache",
      })
      .pipe(
        map(
          (result) =>
            result.data?.buildScan?.configurationDependencyGraph ?? null,
        ),
        tap((graph) => {
          this.graphCache.set(configuration.id, graph);
          this.selectedGraph.set(graph);
          this.loadingGraph.set(false);
        }),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe();
  }

  truncateLabel(label: string) {
    if (label.length <= 32) return label;
    return `${label.slice(0, 29)}…`;
  }
}
