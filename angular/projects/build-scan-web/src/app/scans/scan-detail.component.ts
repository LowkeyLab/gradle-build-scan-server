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
              <h4 class="font-semibold mb-2">
                Timeline
                <span class="font-normal text-sm opacity-60 ml-2">{{ timeline.lanes.length }} worker{{ timeline.lanes.length !== 1 ? 's' : '' }}</span>
              </h4>
              @for (lane of timeline.lanes; track $index) {
                <div class="flex items-center gap-2 py-0.5">
                  <span class="text-xs w-20 text-right shrink-0 opacity-50">Worker {{ $index + 1 }}</span>
                  <div class="relative flex-1 h-6">
                    @for (item of lane; track item.id) {
                      <div
                        class="absolute top-0 h-full rounded-sm tooltip cursor-default overflow-hidden text-xs text-base-content/80 leading-6 px-1 whitespace-nowrap"
                        [class.bg-success]="item.outcome === 'Success' || item.outcome === 'UpToDate'"
                        [class.bg-info]="item.outcome === 'FromCache'"
                        [class.bg-error]="item.outcome === 'Failed'"
                        [class.bg-warning]="item.outcome === 'Skipped'"
                        [class.bg-neutral]="!isKnownOutcome(item.outcome)"
                        [attr.data-tip]="item.taskPath + ' — ' + formatDuration(item.durationMs)"
                        [style.left.%]="item.leftPct"
                        [style.width.%]="item.widthPct">
                        @if (item.widthPct > 5) {
                          {{ item.taskPath }}
                        }
                      </div>
                    }
                  </div>
                </div>
              }
              <div class="flex items-center gap-2 pt-1">
                <span class="w-20 shrink-0"></span>
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

  getTimeline(edges: any[]): { lanes: any[][]; duration: number } | null {
    const tasks = edges
      .filter((e: any) => e.node.startTimestamp != null && e.node.finishTimestamp != null)
      .map((e: any) => ({
        id: e.node.id,
        taskPath: e.node.taskPath,
        outcome: e.node.outcome,
        start: e.node.startTimestamp as number,
        finish: e.node.finishTimestamp as number,
      }))
      .sort((a, b) => a.start - b.start || a.finish - b.finish);

    if (tasks.length === 0) return null;

    const minStart = tasks[0].start;
    const maxFinish = tasks.reduce((max, t) => Math.max(max, t.finish), -Infinity);
    const duration = maxFinish - minStart;

    // Greedy lane assignment: place each task in the first lane where it fits
    const lanes: any[][] = [];
    const laneEnds: number[] = [];

    for (const task of tasks) {
      const taskDuration = Math.max(task.finish - task.start, 0);
      const leftPct = duration > 0 ? ((task.start - minStart) / duration) * 100 : 0;
      const widthPct = duration > 0
        ? Math.min(Math.max((taskDuration / duration) * 100, 0.3), 100 - leftPct)
        : 100;

      const item = {
        id: task.id,
        taskPath: task.taskPath,
        outcome: task.outcome,
        durationMs: taskDuration,
        leftPct,
        widthPct,
      };

      let placed = false;
      for (let i = 0; i < laneEnds.length; i++) {
        if (task.start >= laneEnds[i]) {
          lanes[i].push(item);
          laneEnds[i] = task.finish;
          placed = true;
          break;
        }
      }
      if (!placed) {
        lanes.push([item]);
        laneEnds.push(task.finish);
      }
    }

    return { lanes, duration };
  }

  isKnownOutcome(outcome: string): boolean {
    return ['Success', 'UpToDate', 'FromCache', 'Failed', 'Skipped'].includes(outcome);
  }

  formatDuration(ms: number): string {
    if (ms < 1000) return ms + 'ms';
    return (ms / 1000).toFixed(1) + 's';
  }
}
