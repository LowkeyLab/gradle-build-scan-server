import {
  Component,
  ChangeDetectionStrategy,
  input,
  signal,
} from "@angular/core";

@Component({
  selector: "app-tests-table",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (testCount() > 0) {
      <h3 class="text-xl font-bold mb-4 mt-8">Tests ({{ testCount() }})</h3>

      @if (testSummary(); as summary) {
        <div class="flex gap-2 mb-4 flex-wrap">
          <span class="badge badge-success badge-lg">
            {{ summary.passed }} passed
          </span>
          <span class="badge badge-error badge-lg">
            {{ summary.failed }} failed
          </span>
          <span class="badge badge-warning badge-lg">
            {{ summary.skipped }} skipped
          </span>
          @if (summary.totalDurationMs != null) {
            <span class="badge badge-neutral badge-lg">
              {{ formatDuration(summary.totalDurationMs) }} total
            </span>
          }
        </div>
      }

      <div class="overflow-x-auto">
        <table class="table table-zebra w-full">
          <thead>
            <tr>
              <th>Class Name</th>
              <th>Method Name</th>
              <th>Outcome</th>
              <th>Duration</th>
              <th>Executor</th>
            </tr>
          </thead>
          <tbody>
            @for (edge of testEdges(); track edge.node.id) {
              <tr
                [class.cursor-pointer]="edge.node.outcome === 'Failed' && edge.node.failureMessage"
                [attr.tabindex]="edge.node.outcome === 'Failed' && edge.node.failureMessage ? '0' : null"
                [attr.aria-expanded]="edge.node.outcome === 'Failed' && edge.node.failureMessage ? expandedIds().has(edge.node.id) : null"
                (click)="edge.node.outcome === 'Failed' && edge.node.failureMessage ? toggleExpanded(edge.node.id) : null"
                (keydown.enter)="edge.node.outcome === 'Failed' && edge.node.failureMessage ? toggleExpanded(edge.node.id) : null"
                (keydown.space)="edge.node.outcome === 'Failed' && edge.node.failureMessage ? toggleExpanded(edge.node.id) : null"
              >
                <td class="font-mono text-sm">{{ edge.node.className }}</td>
                <td class="font-mono text-sm">
                  {{ edge.node.methodName || "—" }}
                </td>
                <td>
                  <span
                    class="badge badge-sm"
                    [class.badge-success]="edge.node.outcome === 'Passed'"
                    [class.badge-error]="edge.node.outcome === 'Failed'"
                    [class.badge-warning]="edge.node.outcome === 'Skipped'"
                  >
                    {{ edge.node.outcome || "—" }}
                  </span>
                </td>
                <td class="text-sm tabular-nums">
                  {{ edge.node.durationMs != null ? formatDuration(edge.node.durationMs) : "—" }}
                </td>
                <td class="text-xs opacity-60">
                  {{ edge.node.executorName || "—" }}
                </td>
              </tr>
              @if (expandedIds().has(edge.node.id) && edge.node.failureMessage) {
                <tr>
                  <td colspan="5" class="bg-base-200 p-4">
                    @if (edge.node.failureMessage) {
                      <div class="mb-2">
                        <span class="font-semibold text-error">Failure:</span>
                        <span class="ml-2">{{ edge.node.failureMessage }}</span>
                      </div>
                    }
                    @if (edge.node.failureStacktrace) {
                      <pre
                        class="text-xs overflow-x-auto max-h-64 overflow-y-auto bg-base-300 p-3 rounded"
                      >{{ edge.node.failureStacktrace }}</pre>
                    }
                  </td>
                </tr>
              }
            }
          </tbody>
        </table>
      </div>
    }
  `,
})
export class TestsTableComponent {
  testEdges = input.required<any[]>();
  testCount = input.required<number>();
  testSummary = input<any>();

  expandedIds = signal<Set<string>>(new Set());

  toggleExpanded(id: string) {
    const current = this.expandedIds();
    const next = new Set(current);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    this.expandedIds.set(next);
  }

  formatDuration(ms: number): string {
    if (ms < 1) return "< 1ms";
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  }
}
