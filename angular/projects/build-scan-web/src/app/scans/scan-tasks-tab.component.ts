import {
  ChangeDetectionStrategy,
  Component,
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

      @if (loading() && taskEdges().length === 0) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          Loading tasks…
        </div>
      }

      @if (taskEdges().length > 0) {
        <app-cache-breakdown
          [taskEdges]="taskEdges()"
          [taskCount]="taskCount()"
          [loading]="loading()"
        />
        <app-task-dependency-graph [taskEdges]="taskEdges()" />
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
