import { Component, ChangeDetectionStrategy, input } from "@angular/core";

@Component({
  selector: "app-tasks-table",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <h3 class="text-xl font-bold mb-4">Tasks ({{ taskCount() }})</h3>

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
          @for (edge of taskEdges(); track edge.node.id) {
            <tr>
              <td class="font-mono text-sm">{{ edge.node.taskPath }}</td>
              <td>
                <span
                  class="badge badge-sm"
                  [class.badge-success]="
                    edge.node.outcome === 'Success' ||
                    edge.node.outcome === 'UpToDate'
                  "
                  [class.badge-error]="edge.node.outcome === 'Failed'"
                  [class.badge-info]="edge.node.outcome === 'FromCache'"
                  [class.badge-warning]="edge.node.outcome === 'Skipped'"
                >
                  {{ edge.node.outcome }}
                </span>
              </td>
              <td>
                {{
                  edge.node.durationMs != null
                    ? edge.node.durationMs + "ms"
                    : "—"
                }}
              </td>
              <td>{{ edge.node.cacheable ? "Yes" : "No" }}</td>
              <td class="text-xs">
                @if (edge.node.outcome === "FromCache") {
                  <span class="badge badge-sm badge-info">Cache Hit</span>
                } @else if (edge.node.outcome === "UpToDate") {
                  <span class="badge badge-sm badge-success">Up-to-date</span>
                } @else if (edge.node.cachingDisabledReason) {
                  <span
                    class="tooltip"
                    [attr.data-tip]="edge.node.cachingDisabledExplanation || ''"
                  >
                    <span class="badge badge-sm badge-warning">{{
                      edge.node.cachingDisabledReason
                    }}</span>
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
  `,
})
export class TasksTableComponent {
  taskEdges = input.required<any[]>();
  taskCount = input.required<number>();
}
