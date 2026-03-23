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

const COLOR_MAP: Record<string, string> = {
  "Cache Hit": "oklch(72% 0.15 230)",
  "Up-to-date": "oklch(72% 0.17 150)",
  Executed: "oklch(55% 0.04 260)",
  Failed: "oklch(62% 0.2 25)",
  Skipped: "oklch(75% 0.15 75)",
};

@Component({
  selector: "app-cache-breakdown",
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="card bg-base-200 mb-6">
      <div class="card-body p-4">
        <h4 class="font-semibold mb-3">Cache Breakdown</h4>
        @if (categories().length > 0) {
          <div class="flex items-center gap-6">
            <div #chartContainer class="shrink-0"></div>
            <div class="flex flex-col gap-2">
              <div class="font-mono text-2xl font-bold">
                {{ avoidedPct() }}%
                <span class="text-sm font-normal opacity-50">avoided</span>
              </div>
              @for (cat of categories(); track cat.label) {
                <div class="flex items-center gap-2 text-sm">
                  <span
                    class="inline-block w-3 h-3 rounded-sm shrink-0"
                    [style.background]="cat.color"
                  ></span>
                  <span class="opacity-70">{{ cat.label }}</span>
                  <span class="font-mono font-bold">{{ cat.count }}</span>
                </div>
              }
            </div>
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

  private chartEl = viewChild<ElementRef<HTMLElement>>("chartContainer");

  categories = computed<CacheCategory[]>(() => {
    const edges = this.taskEdges();
    const counts: Record<string, number> = {
      "Cache Hit": 0,
      "Up-to-date": 0,
      Executed: 0,
      Failed: 0,
      Skipped: 0,
    };

    for (const edge of edges) {
      const outcome = edge.node.outcome;
      if (outcome === "FromCache") counts["Cache Hit"]++;
      else if (outcome === "UpToDate") counts["Up-to-date"]++;
      else if (outcome === "Failed") counts["Failed"]++;
      else if (outcome === "Skipped") counts["Skipped"]++;
      else counts["Executed"]++;
    }

    return Object.entries(counts)
      .filter(([, count]) => count > 0)
      .map(([label, count]) => ({
        label,
        count,
        color: COLOR_MAP[label],
      }));
  });

  avoidedPct = computed(() => {
    const cats = this.categories();
    const total = cats.reduce((sum, c) => sum + c.count, 0);
    const avoided = cats
      .filter((c) => c.label === "Cache Hit" || c.label === "Up-to-date")
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
      width: 160,
      height: 160,
      margin: 0,
      color: { domain, range },
      marks: [
        Plot.waffleY(cats, {
          y: "count",
          fill: "label",
          rx: 3,
          gap: 2,
          unit: 1,
          round: true,
        }),
      ],
    });

    el.replaceChildren(plot);
  }
}
