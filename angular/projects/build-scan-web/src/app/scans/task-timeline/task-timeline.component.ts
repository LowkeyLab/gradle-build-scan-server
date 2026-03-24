import {
  Component,
  ChangeDetectionStrategy,
  input,
  computed,
  viewChild,
  ElementRef,
  afterRenderEffect,
} from "@angular/core";
import * as Plot from "@observablehq/plot";

interface TimelineTask {
  taskPath: string;
  outcome: string;
  start: number;
  end: number;
  durationMs: number;
}

const OUTCOME_COLORS: Record<string, string> = {
  Success: "oklch(72% 0.17 150)",
  UpToDate: "oklch(72% 0.17 150 / 0.6)",
  FromCache: "oklch(72% 0.15 230)",
  Failed: "oklch(62% 0.2 25)",
  Skipped: "oklch(75% 0.15 75)",
};

const FALLBACK_COLOR = "oklch(55% 0.04 260)";

function formatDuration(ms: number): string {
  if (ms < 1000) return ms + "ms";
  return (ms / 1000).toFixed(1) + "s";
}

@Component({
  selector: "app-task-timeline",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (tasks().length > 0) {
      <div class="card bg-base-200 mb-6">
        <div class="card-body p-4">
          <h4 class="font-semibold mb-2">Timeline</h4>
          <div #chartContainer></div>
          <div class="flex items-center gap-3 mt-2 flex-wrap">
            @for (entry of legendEntries; track entry.label) {
              <div class="flex items-center gap-1.5 text-xs">
                <span
                  class="inline-block w-3 h-3 rounded-sm"
                  [style.background]="entry.color"
                ></span>
                <span class="opacity-70">{{ entry.label }}</span>
              </div>
            }
          </div>
        </div>
      </div>
    }
  `,
})
export class TaskTimelineComponent {
  taskEdges = input.required<any[]>();

  private chartEl = viewChild<ElementRef<HTMLElement>>("chartContainer");

  readonly legendEntries = [
    { label: "Success", color: OUTCOME_COLORS["Success"] },
    { label: "Up-to-date", color: OUTCOME_COLORS["UpToDate"] },
    { label: "Cache Hit", color: OUTCOME_COLORS["FromCache"] },
    { label: "Failed", color: OUTCOME_COLORS["Failed"] },
    { label: "Skipped", color: OUTCOME_COLORS["Skipped"] },
  ];

  tasks = computed<TimelineTask[]>(() => {
    const edges = this.taskEdges();
    const valid = edges.filter(
      (e: any) =>
        e.node.startTimestamp != null && e.node.finishTimestamp != null,
    );
    if (valid.length === 0) return [];

    let minStart = Infinity;
    for (const e of valid) {
      if (e.node.startTimestamp < minStart) minStart = e.node.startTimestamp;
    }

    return [...valid]
      .sort((a: any, b: any) => a.node.startTimestamp - b.node.startTimestamp)
      .map((e: any) => ({
        taskPath: e.node.taskPath,
        outcome: e.node.outcome,
        start: e.node.startTimestamp - minStart,
        end: e.node.finishTimestamp - minStart,
        durationMs: Math.max(e.node.finishTimestamp - e.node.startTimestamp, 0),
      }));
  });

  constructor() {
    afterRenderEffect(() => {
      this.renderChart();
    });
  }

  private renderChart(): void {
    const ref = this.chartEl();
    if (!ref) return;
    const el = ref.nativeElement;
    const data = this.tasks();
    if (data.length === 0) return;

    const plot = Plot.plot({
      marginLeft: 180,
      marginRight: 20,
      marginTop: 4,
      marginBottom: 30,
      width: 900,
      height: Math.max(data.length * 24 + 34, 80),
      x: {
        label: null,
        tickFormat: (d: number) => formatDuration(d),
      },
      y: {
        label: null,
        domain: data.map((d) => d.taskPath),
        padding: 0.2,
      },
      color: {
        domain: Object.keys(OUTCOME_COLORS),
        range: Object.values(OUTCOME_COLORS),
        unknown: FALLBACK_COLOR,
      },
      marks: [
        Plot.barX(data, {
          x1: "start",
          x2: "end",
          y: "taskPath",
          fill: "outcome",
          rx: 3,
          tip: {
            format: {
              x1: false,
              x2: false,
              y: false,
              fill: false,
            },
            channels: {
              Task: { value: "taskPath" },
              Duration: {
                value: (d: TimelineTask) => formatDuration(d.durationMs),
              },
              Outcome: { value: "outcome" },
            },
          },
        }),
        Plot.ruleX([0]),
      ],
    });

    el.replaceChildren(plot);
  }
}
