import {
  Component,
  ChangeDetectionStrategy,
  input,
  computed,
} from "@angular/core";

interface TaskNode {
  outcome: string | null;
  cacheable: boolean | null;
  cachingDisabledReason: string | null;
  cachingDisabledExplanation: string | null;
  upToDateMessages: string[] | null;
  originBuildInvocationId: string | null;
  originExecutionTime: number | null;
  cacheKey: string | null;
  durationMs: number | null;
  cacheOperations: CacheOperation[] | null;
}

interface CacheOperation {
  id: string;
  operationType: string;
  succeeded: boolean;
  archiveSize: number | null;
  cacheKey: string | null;
}

@Component({
  selector: "app-task-cache-detail",
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="px-6 py-3 bg-base-200/50 text-sm">
      @switch (scenario()) {
        @case ("up-to-date") {
          <div class="flex items-center gap-2">
            <span class="text-info">&#10003;</span>
            @for (msg of taskNode().upToDateMessages ?? []; track msg) {
              <span class="text-info">{{ msg }}</span>
            }
          </div>
        }
        @case ("from-cache") {
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-success">&#10003; Cache hit</span>
            @if (taskNode().originExecutionTime; as time) {
              <span class="text-base-content/50">&bull;</span>
              <span class="text-success">Saved {{ formatMs(time) }}</span>
            }
          </div>
          <div class="text-base-content/50 text-xs mt-1">
            @if (taskNode().originBuildInvocationId; as origin) {
              Origin: <span class="font-mono">{{ origin }}</span>
            }
            @if (taskNode().cacheKey; as key) {
              &bull; Key: <span class="font-mono">{{ key }}</span>
            }
          </div>
        }
        @case ("not-cacheable") {
          <div class="flex items-center gap-2">
            <span class="text-warning"
              >&#9940; Caching disabled:
              {{ taskNode().cachingDisabledReason }}</span
            >
          </div>
          @if (taskNode().cachingDisabledExplanation; as explanation) {
            <div class="text-base-content/50 text-xs mt-1">
              {{ explanation }}
            </div>
          }
        }
        @case ("executed") {
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-amber-400">&#9881; Executed</span>
            @if (taskNode().durationMs; as dur) {
              <span class="text-amber-400">({{ formatMs(dur) }})</span>
            }
            @if (taskNode().cacheKey; as key) {
              <span class="text-base-content/50">&bull;</span>
              <span class="text-base-content/50 text-xs font-mono"
                >Key: {{ key }}</span
              >
            }
          </div>
        }
        @default {
          <span class="text-base-content/40">No cache detail available</span>
        }
      }
      @if ((taskNode().cacheOperations ?? []).length > 0) {
        <div class="mt-3 pt-3 border-t border-base-300">
          <h4
            class="text-xs font-semibold uppercase tracking-wide opacity-60 mb-2"
          >
            Cache Operations
          </h4>
          <div class="flex items-center gap-2 text-sm flex-wrap">
            @for (
              op of taskNode().cacheOperations ?? [];
              track op.id;
              let last = $last
            ) {
              <span
                class="badge badge-sm"
                [class.badge-success]="op.succeeded"
                [class.badge-error]="!op.succeeded"
              >
                {{ formatOp(op) }}
              </span>
              @if (!last) {
                <span class="opacity-30">→</span>
              }
            }
          </div>
        </div>
      }
    </div>
  `,
})
export class TaskCacheDetailComponent {
  taskNode = input.required<TaskNode>();

  scenario = computed(() => {
    const node = this.taskNode();
    if (node.outcome === "UpToDate" && node.upToDateMessages?.length)
      return "up-to-date";
    if (node.outcome === "FromCache") return "from-cache";
    if (node.cacheable === false && node.cachingDisabledReason)
      return "not-cacheable";
    if (
      node.cacheable === true &&
      (node.outcome === "Success" || node.outcome === "Failed") &&
      (node.durationMs != null || node.cacheKey != null)
    )
      return "executed";
    return "fallback";
  });

  formatMs(ms: number): string {
    if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
    return ms + "ms";
  }

  formatOp(op: CacheOperation): string {
    const prefix = op.succeeded ? "✓" : "✗";
    const labels: Record<string, string> = {
      LocalLoad: "Local lookup",
      RemoteLoad: "Remote lookup",
      Pack: "Pack",
      Unpack: "Unpack",
      LocalStore: "Local store",
      RemoteStore: "Remote store",
    };
    const label = labels[op.operationType] ?? op.operationType;
    if (op.archiveSize && op.archiveSize > 0) {
      return prefix + " " + label + " (" + formatBytes(op.archiveSize) + ")";
    }
    return prefix + " " + label;
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}
