import { Component, ChangeDetectionStrategy, input } from "@angular/core";

@Component({
  selector: "app-tests-table",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (testCount() > 0) {
      <h3 class="text-xl font-bold mb-4 mt-8">Tests ({{ testCount() }})</h3>

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
            @for (edge of testEdges(); track edge.node.id) {
              <tr>
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
                <td class="text-xs opacity-60">
                  {{ edge.node.executorName || "—" }}
                </td>
              </tr>
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
}
