import {
  Component,
  ChangeDetectionStrategy,
  input,
  signal,
} from "@angular/core";
import { TaskCacheDetailComponent } from "../task-cache-detail/task-cache-detail.component";

@Component({
  selector: "app-tasks-table",
  imports: [TaskCacheDetailComponent],
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
            <tr
              (click)="toggleRow(edge.node.id)"
              (keydown.enter)="toggleRow(edge.node.id)"
              (keydown.space)="toggleRow(edge.node.id); $event.preventDefault()"
              tabindex="0"
              [attr.aria-expanded]="expandedRows().has(edge.node.id)"
              class="cursor-pointer hover:bg-base-200"
            >
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
            @if (expandedRows().has(edge.node.id)) {
              <tr>
                <td colspan="6" class="p-0">
                  <app-task-cache-detail [taskNode]="edge.node" />
                </td>
              </tr>
            }
          }
        </tbody>
      </table>
    </div>
  `,
})
export class TasksTableComponent {
  taskEdges = input.required<any[]>();
  taskCount = input.required<number>();

  expandedRows = signal<Set<string>>(new Set());

  toggleRow(id: string) {
    this.expandedRows.update((set) => {
      const next = new Set(set);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }
}
