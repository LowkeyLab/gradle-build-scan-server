import { Component, ChangeDetectionStrategy, input } from "@angular/core";

@Component({
  selector: "app-task-timeline",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (getTimeline(); as timeline) {
      <div class="card bg-base-200 mb-6">
        <div class="card-body p-4">
          <h4 class="font-semibold mb-2">Timeline</h4>
          @for (item of timeline.items; track item.id) {
            <div class="flex items-center gap-2 py-0.5">
              <span
                class="font-mono text-xs w-48 truncate text-right shrink-0"
                [title]="item.taskPath"
                >{{ item.taskPath }}</span
              >
              <div class="relative flex-1 h-5">
                <div
                  class="absolute top-0 h-full rounded tooltip"
                  [class.bg-success]="item.outcome === 'Success'"
                  [class.opacity-60]="item.outcome === 'UpToDate'"
                  [class.bg-info]="item.outcome === 'FromCache'"
                  [class.bg-error]="item.outcome === 'Failed'"
                  [class.bg-warning]="item.outcome === 'Skipped'"
                  [class.bg-neutral]="
                    item.outcome !== 'Success' &&
                    item.outcome !== 'UpToDate' &&
                    item.outcome !== 'FromCache' &&
                    item.outcome !== 'Failed' &&
                    item.outcome !== 'Skipped'
                  "
                  [attr.data-tip]="
                    item.taskPath + ' — ' + formatDuration(item.durationMs)
                  "
                  [style.left.%]="item.leftPct"
                  [style.width.%]="item.widthPct"
                ></div>
              </div>
            </div>
          }
          <div class="flex items-center gap-2 pt-1">
            <span class="w-48 shrink-0"></span>
            <div
              class="relative flex-1 flex justify-between text-xs opacity-50"
            >
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
  `,
})
export class TaskTimelineComponent {
  taskEdges = input.required<any[]>();

  getTimeline(): { items: any[]; duration: number } | null {
    const edges = this.taskEdges();
    const tasks = edges.filter(
      (e: any) =>
        e.node.startTimestamp != null && e.node.finishTimestamp != null,
    );
    if (tasks.length === 0) return null;

    let minStart = Infinity;
    let maxFinish = -Infinity;
    for (const e of tasks) {
      if (e.node.startTimestamp < minStart) minStart = e.node.startTimestamp;
      if (e.node.finishTimestamp > maxFinish)
        maxFinish = e.node.finishTimestamp;
    }
    const duration = maxFinish - minStart;
    const sorted = [...tasks].sort(
      (a: any, b: any) => a.node.startTimestamp - b.node.startTimestamp,
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
        const taskDuration = Math.max(
          e.node.finishTimestamp - e.node.startTimestamp,
          0,
        );
        const leftPct = (start / duration) * 100;
        const widthPct = Math.min(
          Math.max((taskDuration / duration) * 100, 0.5),
          100 - leftPct,
        );
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
    if (ms < 1000) return ms + "ms";
    return (ms / 1000).toFixed(1) + "s";
  }
}
