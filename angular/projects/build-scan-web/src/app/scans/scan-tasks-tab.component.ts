import {
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  inject,
  input,
  signal,
} from "@angular/core";
import type { OnInit } from "@angular/core";
import { Apollo, gql, type QueryRef } from "apollo-angular";
import { EMPTY, filter, map, switchMap, tap } from "rxjs";
import { takeUntilDestroyed, toObservable } from "@angular/core/rxjs-interop";
import { CacheBreakdownComponent } from "./cache-breakdown/cache-breakdown.component";
import { TaskDependencyGraphComponent } from "./task-dependency-graph/task-dependency-graph.component";
import { TasksTableComponent } from "./tasks-table/tasks-table.component";

interface TaskEdge {
  node: {
    id: string;
    dependencies: string[];
    taskPath: string;
    className: string;
    outcome: string;
    cacheable: boolean | null;
    durationMs: number | null;
    cacheKey: string | null;
    cachingDisabledReason: string | null;
    cachingDisabledExplanation: string | null;
    upToDateMessages: string[] | null;
    originBuildInvocationId: string | null;
    originExecutionTime: number | null;
    cacheOperations: Array<{
      id: string;
      operationType: string;
      succeeded: boolean;
      archiveSize: number | null;
      cacheKey: string | null;
      durationMs: number | null;
    }> | null;
  };
  cursor: string;
}

interface TaskScan {
  id: string;
  tasks: {
    edges: TaskEdge[];
    pageInfo: {
      hasNextPage: boolean;
      endCursor: string | null;
    };
  };
}

interface GetScanTasksData {
  buildScan: TaskScan | null;
}

interface GetScanTasksVariables {
  id: string;
  firstTasks: number;
  afterTasks?: string;
}

interface PartialTaskScan {
  tasks?: {
    edges?: TaskEdge[];
    pageInfo?: {
      hasNextPage?: boolean;
      endCursor?: string | null;
    };
  };
}

const LARGE_GRAPH_TASK_THRESHOLD = 500;

function taskWeight(edge: TaskEdge): number {
  return Math.max(
    0,
    edge.node.durationMs ?? edge.node.originExecutionTime ?? 0,
  );
}

function compareCriticalPathCandidates(
  left: { score: number; length: number; edge: TaskEdge },
  right: { score: number; length: number; edge: TaskEdge },
): number {
  const scoreDelta = left.score - right.score;
  if (scoreDelta !== 0) return scoreDelta;

  const lengthDelta = left.length - right.length;
  if (lengthDelta !== 0) return lengthDelta;

  const pathDelta = right.edge.node.taskPath.localeCompare(
    left.edge.node.taskPath,
  );
  return pathDelta !== 0
    ? pathDelta
    : right.edge.node.id.localeCompare(left.edge.node.id);
}

function selectCriticalPathTaskEdges(taskEdges: TaskEdge[]): TaskEdge[] {
  if (taskEdges.length <= 1) return taskEdges;

  const edgeById = new Map(taskEdges.map((edge) => [edge.node.id, edge]));
  const successorsById = new Map<string, string[]>();
  const predecessorsById = new Map<string, string[]>();
  const indegreeById = new Map<string, number>();

  for (const edge of taskEdges) {
    successorsById.set(edge.node.id, []);
    predecessorsById.set(edge.node.id, []);
    indegreeById.set(edge.node.id, 0);
  }

  for (const edge of taskEdges) {
    for (const dependencyId of edge.node.dependencies ?? []) {
      if (!edgeById.has(dependencyId) || dependencyId === edge.node.id)
        continue;
      successorsById.get(dependencyId)?.push(edge.node.id);
      predecessorsById.get(edge.node.id)?.push(dependencyId);
      indegreeById.set(edge.node.id, (indegreeById.get(edge.node.id) ?? 0) + 1);
    }
  }

  const ready = taskEdges
    .filter((edge) => (indegreeById.get(edge.node.id) ?? 0) === 0)
    .sort((left, right) =>
      left.node.taskPath.localeCompare(right.node.taskPath),
    );
  const topoOrder: TaskEdge[] = [];

  while (ready.length > 0) {
    const edge = ready.shift();
    if (!edge) continue;
    topoOrder.push(edge);

    for (const successorId of successorsById.get(edge.node.id) ?? []) {
      const nextIndegree = (indegreeById.get(successorId) ?? 0) - 1;
      indegreeById.set(successorId, nextIndegree);
      if (nextIndegree === 0) {
        const successor = edgeById.get(successorId);
        if (successor) {
          ready.push(successor);
          ready.sort((left, right) =>
            left.node.taskPath.localeCompare(right.node.taskPath),
          );
        }
      }
    }
  }

  if (topoOrder.length !== taskEdges.length) {
    const topWeightedEdge = [...taskEdges].sort((left, right) => {
      const weightDelta = taskWeight(right) - taskWeight(left);
      return weightDelta !== 0
        ? weightDelta
        : left.node.taskPath.localeCompare(right.node.taskPath);
    })[0];
    return topWeightedEdge ? [topWeightedEdge] : [];
  }

  const bestScoreById = new Map<string, number>();
  const bestLengthById = new Map<string, number>();
  const previousIdById = new Map<string, string | null>();

  for (const edge of topoOrder) {
    let previousId: string | null = null;
    let previousScore = 0;
    let previousLength = 0;

    for (const predecessorId of predecessorsById.get(edge.node.id) ?? []) {
      const predecessor = edgeById.get(predecessorId);
      if (!predecessor) continue;
      const candidate = {
        score: bestScoreById.get(predecessorId) ?? 0,
        length: bestLengthById.get(predecessorId) ?? 0,
        edge: predecessor,
      };
      const current = {
        score: previousScore,
        length: previousLength,
        edge: previousId
          ? (edgeById.get(previousId) ?? predecessor)
          : predecessor,
      };
      if (
        !previousId ||
        compareCriticalPathCandidates(candidate, current) > 0
      ) {
        previousId = predecessorId;
        previousScore = candidate.score;
        previousLength = candidate.length;
      }
    }

    bestScoreById.set(edge.node.id, previousScore + taskWeight(edge));
    bestLengthById.set(edge.node.id, previousLength + 1);
    previousIdById.set(edge.node.id, previousId);
  }

  const terminalEdge = topoOrder.reduce<TaskEdge | null>((best, edge) => {
    if (!best) return edge;
    const candidate = {
      score: bestScoreById.get(edge.node.id) ?? 0,
      length: bestLengthById.get(edge.node.id) ?? 0,
      edge,
    };
    const current = {
      score: bestScoreById.get(best.node.id) ?? 0,
      length: bestLengthById.get(best.node.id) ?? 0,
      edge: best,
    };
    return compareCriticalPathCandidates(candidate, current) > 0 ? edge : best;
  }, null);

  const pathIds = new Set<string>();
  let currentId = terminalEdge?.node.id ?? null;
  while (currentId) {
    pathIds.add(currentId);
    currentId = previousIdById.get(currentId) ?? null;
  }

  return taskEdges.filter((edge) => pathIds.has(edge.node.id));
}

const GET_SCAN_TASKS = gql`
  query GetScanTasks($id: ID!, $firstTasks: Int!, $afterTasks: String) {
    buildScan(id: $id) {
      id
      tasks(first: $firstTasks, after: $afterTasks) {
        edges {
          node {
            id
            dependencies
            taskPath
            className
            outcome
            cacheable
            durationMs
            cacheKey
            cachingDisabledReason
            cachingDisabledExplanation
            upToDateMessages
            originBuildInvocationId
            originExecutionTime
            cacheOperations {
              id
              operationType
              succeeded
              archiveSize
              cacheKey
              durationMs
            }
          }
          cursor
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
`;

@Component({
  selector: "app-scan-tasks-tab",
  imports: [
    CacheBreakdownComponent,
    TaskDependencyGraphComponent,
    TasksTableComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="space-y-6">
      <h2 class="text-2xl font-bold">Tasks</h2>

      @if (loading()) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          @if (taskEdges().length === 0) {
            Loading tasks…
          } @else {
            Loading remaining tasks ({{ taskEdges().length }} of
            {{ taskCount() }})…
          }
        </div>
      } @else if (taskEdges().length > 0) {
        <app-cache-breakdown
          [taskEdges]="taskEdges()"
          [taskCount]="taskCount()"
          [loading]="loading()"
        />

        @if (showDependencyGraph()) {
          <app-task-dependency-graph
            [taskEdges]="displayedGraphTaskEdges()"
            [title]="dependencyGraphTitle()"
            [description]="dependencyGraphDescription()"
          />
        }

        @if (hasLargeTaskGraph()) {
          <div
            class="rounded-md border border-base-300 bg-base-200 p-4 text-sm"
          >
            <div class="font-semibold">Showing critical path only</div>
            <p class="mt-1 opacity-70">
              This scan has {{ taskCount() }} tasks. The full dependency graph
              can stall the browser, so the graph above shows the critical path
              by default.
            </p>
          </div>
        }

        <app-tasks-table [taskEdges]="taskEdges()" [taskCount]="taskCount()" />
      } @else if (!loading()) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          No tasks recorded for this scan.
        </div>
      }
    </div>
  `,
})
export class ScanTasksTabComponent implements OnInit {
  scanId = input.required<string>();
  taskCount = input.required<number>();

  private apollo = inject(Apollo);
  private destroyRef = inject(DestroyRef);
  private scanId$ = toObservable(this.scanId);

  taskEdges = signal<TaskEdge[]>([]);
  loading = signal(true);
  hasLargeTaskGraph = computed(
    () => this.taskCount() > LARGE_GRAPH_TASK_THRESHOLD,
  );
  showDependencyGraph = computed(
    () => !this.loading() && this.taskEdges().length > 0,
  );
  displayedGraphTaskEdges = computed(() =>
    this.hasLargeTaskGraph()
      ? selectCriticalPathTaskEdges(this.taskEdges())
      : this.taskEdges(),
  );
  dependencyGraphTitle = computed(() =>
    this.hasLargeTaskGraph()
      ? "Critical Path"
      : "Task Dependencies",
  );
  dependencyGraphDescription = computed(() =>
    this.hasLargeTaskGraph()
      ? "Showing the longest weighted dependency chain."
      : "Static graph of task prerequisites.",
  );

  private loadRemainingPages(
    queryRef: QueryRef<GetScanTasksData, GetScanTasksVariables>,
    id: string,
    afterTasks: string,
  ): Promise<void> {
    return queryRef
      .fetchMore({
        variables: {
          id,
          firstTasks: 100,
          afterTasks,
        },
      })
      .then(({ data }) => {
        const scan = data?.buildScan;
        if (!scan) return;
        const edges = scan.tasks?.edges ?? [];
        const pageInfo = scan.tasks?.pageInfo;
        this.taskEdges.update((currentEdges) => [...currentEdges, ...edges]);
        if (pageInfo?.hasNextPage && pageInfo.endCursor) {
          return this.loadRemainingPages(queryRef, id, pageInfo.endCursor);
        }
        this.loading.set(false);
        return;
      });
  }

  ngOnInit() {
    this.scanId$
      .pipe(
        switchMap((id) => {
          this.taskEdges.set([]);
          if (this.taskCount() === 0) {
            this.loading.set(false);
            return EMPTY;
          }

          this.loading.set(true);

          const queryRef = this.apollo.watchQuery<
            GetScanTasksData,
            GetScanTasksVariables
          >({
            query: GET_SCAN_TASKS,
            variables: {
              id,
              firstTasks: 100,
            },
            errorPolicy: "all",
          });

          return queryRef.valueChanges.pipe(
            map((result) => result.data?.buildScan ?? null),
            filter((scan): scan is PartialTaskScan => !!scan),
            tap((scan) => {
              const edges = scan.tasks?.edges ?? [];
              const pageInfo = scan.tasks?.pageInfo;
              if (this.taskEdges().length === 0) {
                this.taskEdges.set(edges);
                if (pageInfo?.hasNextPage && pageInfo.endCursor) {
                  void this.loadRemainingPages(
                    queryRef,
                    id,
                    pageInfo.endCursor,
                  );
                } else {
                  this.loading.set(false);
                }
              }
            }),
          );
        }),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe();
  }
}
