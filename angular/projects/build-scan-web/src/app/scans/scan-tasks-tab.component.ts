import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  OnInit,
  inject,
  input,
  signal,
} from "@angular/core";
import { Apollo, gql } from "apollo-angular";
import { filter, map, switchMap, tap } from "rxjs";
import { takeUntilDestroyed, toObservable } from "@angular/core/rxjs-interop";
import { CacheBreakdownComponent } from "./cache-breakdown/cache-breakdown.component";
import { TaskTimelineComponent } from "./task-timeline/task-timeline.component";
import { TasksTableComponent } from "./tasks-table/tasks-table.component";

const GET_SCAN_TASKS = gql`
  query GetScanTasks($id: ID!, $firstTasks: Int!, $afterTasks: String) {
    buildScan(id: $id) {
      id
      tasks(first: $firstTasks, after: $afterTasks) {
        edges {
          node {
            id
            taskPath
            className
            outcome
            cacheable
            durationMs
            startTimestamp
            finishTimestamp
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
    TaskTimelineComponent,
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
        <app-task-timeline [taskEdges]="taskEdges()" />
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

  taskEdges = signal<any[]>([]);
  loading = signal(true);

  private loadRemainingPages(
    queryRef: any,
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
        updateQuery: (previous: any, { fetchMoreResult }: any) => {
          if (!fetchMoreResult?.buildScan) return previous;
          const previousEdges = previous.buildScan?.tasks?.edges ?? [];
          const nextEdges = fetchMoreResult.buildScan.tasks?.edges ?? [];
          return {
            buildScan: {
              ...previous.buildScan,
              ...fetchMoreResult.buildScan,
              tasks: {
                ...fetchMoreResult.buildScan.tasks,
                edges: [...previousEdges, ...nextEdges],
              },
            },
          };
        },
      })
      .then(({ data }: any) => {
        const scan = data?.buildScan;
        if (!scan) return;
        this.taskEdges.update((edges) => [...edges, ...scan.tasks.edges]);
        if (scan.tasks.pageInfo.hasNextPage) {
          return this.loadRemainingPages(
            queryRef,
            id,
            scan.tasks.pageInfo.endCursor,
          );
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
          this.loading.set(true);

          const queryRef = this.apollo.watchQuery<any>({
            query: GET_SCAN_TASKS,
            variables: {
              id,
              firstTasks: 100,
            },
            errorPolicy: "all",
          });

          return queryRef.valueChanges.pipe(
            filter((result) => !!result.data),
            map((result) => result.data.buildScan),
            tap((scan) => {
              if (this.taskEdges().length === 0) {
                this.taskEdges.set(scan.tasks.edges);
                if (scan.tasks.pageInfo.hasNextPage) {
                  void this.loadRemainingPages(
                    queryRef,
                    id,
                    scan.tasks.pageInfo.endCursor,
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
