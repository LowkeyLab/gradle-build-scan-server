import { Component, ChangeDetectionStrategy, input, inject } from '@angular/core';
import { Apollo, gql } from 'apollo-angular';
import { AsyncPipe, DatePipe } from '@angular/common';
import { RouterLink } from '@angular/router';
import { filter, map, switchMap } from 'rxjs';
import { toObservable } from '@angular/core/rxjs-interop';

const GET_BUILD_SCAN = gql`
  query GetBuildScan($id: ID!, $firstTasks: Int!, $afterTasks: String, $firstTests: Int!, $afterTests: String) {
    buildScan(id: $id) {
      id
      scanId
      buildToolType
      buildToolVersion
      pluginVersion
      outcome
      createdAt
      hostname
      osName
      osVersion
      jvmVendor
      jvmVersion
      requestedTasks
      taskCount
      testCount
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
          }
          cursor
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
      tests(first: $firstTests, after: $afterTests) {
        edges {
          node {
            id
            className
            methodName
            executorName
            outcome
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
  selector: 'app-scan-detail',
  imports: [AsyncPipe, DatePipe, RouterLink],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="container mx-auto p-6">
      @if (scan$ | async; as scan) {
        <div class="mb-4">
          <a routerLink="/scans" class="link link-primary">&larr; All Scans</a>
        </div>

        <div class="card bg-base-100 shadow-xl mb-6">
          <div class="card-body">
            <h2 class="card-title">
              Build Scan
              <span class="badge"
                [class.badge-success]="scan.outcome === 'success'"
                [class.badge-error]="scan.outcome === 'failed'">
                {{ scan.outcome }}
              </span>
            </h2>

            <div class="grid grid-cols-2 gap-4 mt-4">
              <div>
                <span class="text-sm opacity-60">Created</span>
                <p>{{ scan.createdAt | date:'long' }}</p>
              </div>
              <div>
                <span class="text-sm opacity-60">Build Tool</span>
                <p>{{ scan.buildToolType }} {{ scan.buildToolVersion }}</p>
              </div>
              <div>
                <span class="text-sm opacity-60">Plugin Version</span>
                <p>{{ scan.pluginVersion }}</p>
              </div>
              <div>
                <span class="text-sm opacity-60">Hostname</span>
                <p>{{ scan.hostname || '—' }}</p>
              </div>
              @if (scan.osName) {
                <div>
                  <span class="text-sm opacity-60">OS</span>
                  <p>{{ scan.osName }} {{ scan.osVersion }}</p>
                </div>
              }
              @if (scan.jvmVersion) {
                <div>
                  <span class="text-sm opacity-60">JVM</span>
                  <p>{{ scan.jvmVendor }} {{ scan.jvmVersion }}</p>
                </div>
              }
            </div>

            @if (scan.requestedTasks.length > 0) {
              <div class="mt-4">
                <span class="text-sm opacity-60">Requested Tasks</span>
                <p>{{ scan.requestedTasks.join(' ') }}</p>
              </div>
            }
          </div>
        </div>

        <h3 class="text-xl font-bold mb-4">
          Tasks ({{ scan.taskCount }})
        </h3>

        @if (getTimeline(scan.tasks.edges); as timeline) {
          <div class="card bg-base-200 mb-6">
            <div class="card-body p-4">
              <h4 class="font-semibold mb-2">Timeline</h4>
              @for (item of timeline.items; track item.id) {
                <div class="flex items-center gap-2 py-0.5">
                  <span class="font-mono text-xs w-48 truncate text-right shrink-0" [title]="item.taskPath">{{ item.taskPath }}</span>
                  <div class="relative flex-1 h-5">
                    <div
                      class="absolute top-0 h-full rounded tooltip"
                      [class.bg-success]="item.outcome === 'Success'"
                      [class.opacity-60]="item.outcome === 'UpToDate'"
                      [class.bg-info]="item.outcome === 'FromCache'"
                      [class.bg-error]="item.outcome === 'Failed'"
                      [class.bg-warning]="item.outcome === 'Skipped'"
                      [class.bg-neutral]="item.outcome !== 'Success' && item.outcome !== 'UpToDate' && item.outcome !== 'FromCache' && item.outcome !== 'Failed' && item.outcome !== 'Skipped'"
                      [attr.data-tip]="item.taskPath + ' — ' + formatDuration(item.durationMs)"
                      [style.left.%]="item.leftPct"
                      [style.width.%]="item.widthPct">
                    </div>
                  </div>
                </div>
              }
              <div class="flex items-center gap-2 pt-1">
                <span class="w-48 shrink-0"></span>
                <div class="relative flex-1 flex justify-between text-xs opacity-50">
                  <span>0ms</span>
                  @if (timeline.duration > 0) {
                    <span>{{ formatDuration(timeline.duration / 2) }}</span>
                    <span>{{ formatDuration(timeline.duration) }}</span>
                  }
                </div>
              </div>
            </div>
          </div>
        }

        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>Task Path</th>
                <th>Outcome</th>
                <th>Duration</th>
                <th>Cacheable</th>
                <th>Caching Reason</th>
                <th>Class</th>
              </tr>
            </thead>
            <tbody>
              @for (edge of scan.tasks.edges; track edge.node.id) {
                <tr>
                  <td class="font-mono text-sm">{{ edge.node.taskPath }}</td>
                  <td>
                    <span class="badge badge-sm"
                      [class.badge-success]="edge.node.outcome === 'Success' || edge.node.outcome === 'UpToDate'"
                      [class.badge-error]="edge.node.outcome === 'Failed'"
                      [class.badge-info]="edge.node.outcome === 'FromCache'"
                      [class.badge-warning]="edge.node.outcome === 'Skipped'">
                      {{ edge.node.outcome }}
                    </span>
                  </td>
                  <td>{{ edge.node.durationMs != null ? edge.node.durationMs + 'ms' : '—' }}</td>
                  <td>{{ edge.node.cacheable ? 'Yes' : 'No' }}</td>
                  <td class="text-xs">
                    @if (edge.node.outcome === 'FromCache') {
                      <span class="badge badge-sm badge-info">Cache Hit</span>
                    } @else if (edge.node.outcome === 'UpToDate') {
                      <span class="badge badge-sm badge-success">Up-to-date</span>
                    } @else if (edge.node.cachingDisabledReason) {
                      <span class="tooltip" [attr.data-tip]="edge.node.cachingDisabledExplanation || ''">
                        <span class="badge badge-sm badge-warning">{{ edge.node.cachingDisabledReason }}</span>
                      </span>
                    } @else if (edge.node.cacheable) {
                      <span class="badge badge-sm badge-ghost">Executed</span>
                    } @else {
                      <span class="opacity-40">—</span>
                    }
                  </td>
                  <td class="text-xs opacity-60">{{ edge.node.className }}</td>
                </tr>
              }
            </tbody>
          </table>
        </div>

        @if (scan.testCount > 0) {
          <h3 class="text-xl font-bold mb-4 mt-8">
            Tests ({{ scan.testCount }})
          </h3>

          <div class="overflow-x-auto">
            <table class="table table-zebra w-full">
              <thead>
                <tr>
                  <th>Class Name</th>
                  <th>Method Name</th>
                  <th>Outcome</th>
                  <th>Executor</th>
                </tr>
              </thead>
              <tbody>
                @for (edge of scan.tests.edges; track edge.node.id) {
                  <tr>
                    <td class="font-mono text-sm">{{ edge.node.className }}</td>
                    <td class="font-mono text-sm">{{ edge.node.methodName || '—' }}</td>
                    <td>
                      <span class="badge badge-sm"
                        [class.badge-success]="edge.node.outcome === 'Passed'"
                        [class.badge-error]="edge.node.outcome === 'Failed'"
                        [class.badge-warning]="edge.node.outcome === 'Skipped'">
                        {{ edge.node.outcome || '—' }}
                      </span>
                    </td>
                    <td class="text-xs opacity-60">{{ edge.node.executorName || '—' }}</td>
                  </tr>
                }
              </tbody>
            </table>
          </div>
        }
      }
    </div>
  `,
})
export class ScanDetailComponent {
  id = input.required<string>();
  private apollo = inject(Apollo);

  scan$ = toObservable(this.id).pipe(
    switchMap(id =>
      this.apollo.watchQuery<any>({
        query: GET_BUILD_SCAN,
        variables: { id, firstTasks: 100, firstTests: 100 },
        errorPolicy: 'all',
      }).valueChanges
    ),
    filter(result => !!result.data),
    map(result => result.data.buildScan)
  );

  getTimeline(edges: any[]): { items: any[]; duration: number } | null {
    const tasks = edges.filter(
      (e: any) => e.node.startTimestamp != null && e.node.finishTimestamp != null
    );
    if (tasks.length === 0) return null;

    let minStart = Infinity;
    let maxFinish = -Infinity;
    for (const e of tasks) {
      if (e.node.startTimestamp < minStart) minStart = e.node.startTimestamp;
      if (e.node.finishTimestamp > maxFinish) maxFinish = e.node.finishTimestamp;
    }
    const duration = maxFinish - minStart;
    const sorted = [...tasks].sort(
      (a: any, b: any) => a.node.startTimestamp - b.node.startTimestamp
    );

    if (duration === 0) {
      return {
        duration: 0,
        items: sorted.map((e: any) => ({
          id: e.node.id,
          taskPath: e.node.taskPath,
          outcome: e.node.outcome,
          durationMs: 0,
          leftPct: 0,
          widthPct: 100,
        })),
      };
    }

    return {
      duration,
      items: sorted.map((e: any) => {
        const start = e.node.startTimestamp - minStart;
        const taskDuration = Math.max(e.node.finishTimestamp - e.node.startTimestamp, 0);
        const leftPct = (start / duration) * 100;
        const widthPct = Math.min(Math.max((taskDuration / duration) * 100, 0.5), 100 - leftPct);
        return {
          id: e.node.id,
          taskPath: e.node.taskPath,
          outcome: e.node.outcome,
          durationMs: taskDuration,
          leftPct,
          widthPct,
        };
      }),
    };
  }

  formatDuration(ms: number): string {
    if (ms < 1000) return ms + 'ms';
    return (ms / 1000).toFixed(1) + 's';
  }
}
