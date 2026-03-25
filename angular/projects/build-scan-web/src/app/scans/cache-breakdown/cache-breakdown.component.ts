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

interface CacheCategory {
  label: string;
  count: number;
  color: string;
}

const AVOIDED_LABELS = ["Cache Hit", "Up-to-date", "No Source", "Avoided"];

const COLOR_MAP: Record<string, string> = {
  "Cache Hit": "oklch(72% 0.15 230)",
  "Up-to-date": "oklch(72% 0.17 150)",
  "No Source": "oklch(72% 0.12 200)",
  Avoided: "oklch(68% 0.10 210)",
  Executed: "oklch(55% 0.04 260)",
  Failed: "oklch(62% 0.2 25)",
  Skipped: "oklch(75% 0.15 75)",
};

const OUTCOME_TO_CATEGORY: Record<string, string> = {
  FromCache: "Cache Hit",
  UpToDate: "Up-to-date",
  NoSource: "No Source",
  AvoidedForUnknownReason: "Avoided",
  Success: "Executed",
  Failed: "Failed",
  Skipped: "Skipped",
};

const CATEGORY_ORDER = [
  "Cache Hit",
  "Up-to-date",
  "No Source",
  "Avoided",
  "Executed",
  "Failed",
  "Skipped",
];

@Component({
  selector: "app-cache-breakdown",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="card bg-base-200 mb-6">
      <div class="card-body p-4">
        <div class="flex items-baseline justify-between mb-3">
          <h4 class="font-semibold">Cache Breakdown</h4>
          <div class="font-mono text-xl font-bold">
            {{ avoidedPct() }}%
            <span class="text-sm font-normal opacity-50">avoided</span>
          </div>
        </div>
        @if (categories().length > 0) {
          <div #chartContainer class="w-full mb-3"></div>
          <div class="flex flex-wrap gap-x-5 gap-y-1">
            @for (cat of categories(); track cat.label) {
              <div class="flex items-center gap-1.5 text-sm">
                <span
                  class="inline-block w-2.5 h-2.5 rounded-full shrink-0"
                  [style.background]="cat.color"
                ></span>
                <span class="opacity-70">{{ cat.label }}</span>
                <span class="font-mono font-semibold">{{ cat.count }}</span>
              </div>
            }
          </div>
        } @else {
          <p class="text-sm opacity-50">No task data available</p>
        }
      </div>
    </div>
  `,
})
export class CacheBreakdownComponent {
  taskEdges = input.required<any[]>();
  taskCount = input.required<number>();

  private chartEl = viewChild<ElementRef<HTMLElement>>("chartContainer");

  categories = computed<CacheCategory[]>(() => {
    const edges = this.taskEdges();
    const counts: Record<string, number> = {};

    for (const edge of edges) {
      const outcome = edge.node.outcome;
      const category = OUTCOME_TO_CATEGORY[outcome] ?? "Executed";
      counts[category] = (counts[category] ?? 0) + 1;
    }

    return Object.entries(counts)
      .filter(([, count]) => count > 0)
      .map(([label, count]) => ({
        label,
        count,
        color: COLOR_MAP[label] ?? COLOR_MAP["Executed"],
      }))
      .sort(
        (a, b) =>
          CATEGORY_ORDER.indexOf(a.label) - CATEGORY_ORDER.indexOf(b.label),
      );
  });

  avoidedPct = computed(() => {
    const cats = this.categories();
    const total = cats.reduce((sum, c) => sum + c.count, 0);
    const avoided = cats
      .filter((c) => AVOIDED_LABELS.includes(c.label))
      .reduce((sum, c) => sum + c.count, 0);
    return total > 0 ? Math.round((avoided / total) * 100) : 0;
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
    const cats = this.categories();
    if (cats.length === 0) return;

    const domain = cats.map((c) => c.label);
    const range = cats.map((c) => c.color);

    const plot = Plot.plot({
      axis: null,
      label: null,
      width: el.clientWidth || 600,
      height: 32,
      margin: 0,
      color: { domain, range },
      x: { axis: null },
      marks: [
        Plot.barX(cats, Plot.stackX({ x: "count", fill: "label", rx: 6 })),
      ],
    });

    plot.style.width = "100%";
    el.replaceChildren(plot);
  }
}
