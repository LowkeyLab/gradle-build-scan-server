import {
  Component,
  ChangeDetectionStrategy,
  input,
  computed,
  output,
} from "@angular/core";
import { DatePipe } from "@angular/common";
import { RouterLink } from "@angular/router";

interface SectionNav {
  id: string;
  label: string;
  icon: string;
}

@Component({
  selector: "app-scan-sidebar",
  imports: [DatePipe, RouterLink],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (scan(); as scan) {
      <aside
        class="flex flex-col h-screen p-5 bg-base-200 border-r border-base-300 overflow-y-auto"
      >
        <!-- Build identity -->
        <div class="flex flex-col gap-1">
          <div
            class="text-[0.625rem] font-bold tracking-widest uppercase opacity-50"
          >
            Scan
          </div>
          <div class="font-mono text-sm opacity-75 break-all leading-tight">
            {{ scan.scanId }}
          </div>
          <div class="mt-1">
            <span
              class="badge badge-sm gap-1"
              [class.badge-success]="scan.outcome === 'success'"
              [class.badge-error]="scan.outcome === 'failed'"
            >
              {{ scan.outcome }}
            </span>
          </div>
          <time class="text-xs opacity-50 mt-0.5">{{
            scan.createdAt | date: "MMM d, y HH:mm"
          }}</time>
        </div>

        <div class="divider my-2"></div>

        <!-- Quick stats -->
        <div class="grid grid-cols-2 gap-2">
          <div class="flex flex-col p-2 rounded-md bg-base-300">
            <span class="font-mono text-xl font-bold">{{
              scan.taskCount
            }}</span>
            <span
              class="text-[0.625rem] font-semibold tracking-wider uppercase opacity-50"
              >Tasks</span
            >
          </div>
          <div class="flex flex-col p-2 rounded-md bg-base-300">
            <span class="font-mono text-xl font-bold">{{
              scan.testCount
            }}</span>
            <span
              class="text-[0.625rem] font-semibold tracking-wider uppercase opacity-50"
              >Tests</span
            >
          </div>
          <div class="flex flex-col p-2 rounded-md bg-base-300 col-span-2">
            <span class="font-mono text-xl font-bold text-info">{{
              cacheAvoidanceRate()
            }}</span>
            <span
              class="text-[0.625rem] font-semibold tracking-wider uppercase opacity-50"
              >Cache Avoidance</span
            >
          </div>
        </div>

        <div class="divider my-2"></div>

        <!-- Environment -->
        <div class="flex flex-col gap-1.5">
          <div
            class="text-[0.625rem] font-bold tracking-widest uppercase opacity-50 mb-0.5"
          >
            Environment
          </div>
          <dl class="flex flex-col gap-1">
            <div class="flex justify-between items-baseline gap-2">
              <dt class="text-xs opacity-50 shrink-0">Build Tool</dt>
              <dd class="text-xs text-right truncate">
                {{ scan.buildToolType }} {{ scan.buildToolVersion }}
              </dd>
            </div>
            <div class="flex justify-between items-baseline gap-2">
              <dt class="text-xs opacity-50 shrink-0">Plugin</dt>
              <dd class="text-xs text-right truncate">
                {{ scan.pluginVersion }}
              </dd>
            </div>
            @if (scan.hostname) {
              <div class="flex justify-between items-baseline gap-2">
                <dt class="text-xs opacity-50 shrink-0">Host</dt>
                <dd class="text-xs text-right truncate font-mono">
                  {{ scan.hostname }}
                </dd>
              </div>
            }
            @if (scan.osName) {
              <div class="flex justify-between items-baseline gap-2">
                <dt class="text-xs opacity-50 shrink-0">OS</dt>
                <dd class="text-xs text-right truncate">
                  {{ scan.osName }} {{ scan.osVersion }}
                </dd>
              </div>
            }
            @if (scan.jvmVersion) {
              <div class="flex justify-between items-baseline gap-2">
                <dt class="text-xs opacity-50 shrink-0">JVM</dt>
                <dd class="text-xs text-right truncate">
                  {{ scan.jvmVendor }} {{ scan.jvmVersion }}
                </dd>
              </div>
            }
          </dl>
        </div>

        @if (scan.requestedTasks.length > 0) {
          <div class="divider my-2"></div>
          <div class="flex flex-col gap-1.5">
            <div
              class="text-[0.625rem] font-bold tracking-widest uppercase opacity-50 mb-0.5"
            >
              Requested Tasks
            </div>
            <div class="flex flex-wrap gap-1">
              @for (task of scan.requestedTasks; track task) {
                <code
                  class="badge badge-sm badge-outline font-mono text-[0.65rem]"
                  >{{ task }}</code
                >
              }
            </div>
          </div>
        }

        <div class="divider my-2"></div>

        <!-- Section navigation -->
        <nav class="flex flex-col gap-0.5">
          <div
            class="text-[0.625rem] font-bold tracking-widest uppercase opacity-50 mb-1"
          >
            Sections
          </div>
          @for (section of sections(); track section.id) {
            <button
              type="button"
              class="flex items-center gap-2 px-2.5 py-2 rounded-md text-sm cursor-pointer transition-all duration-150"
              [class]="
                activeSection() === section.id
                  ? 'bg-primary/15 text-primary'
                  : 'opacity-60 hover:bg-base-300 hover:opacity-100'
              "
              (click)="navigateToSection(section.id)"
            >
              <span class="w-5 text-center" aria-hidden="true">{{
                section.icon
              }}</span>
              <span>{{ section.label }}</span>
            </button>
          }
        </nav>

        <!-- Back link -->
        <div class="mt-auto pt-3 border-t border-base-300">
          <a
            class="back-link link link-hover text-xs opacity-50 hover:opacity-80"
            routerLink="/scans"
            >&larr; All Scans</a
          >
        </div>
      </aside>
    }
  `,
})
export class ScanSidebarComponent {
  scan = input.required<any>();
  activeSection = input<string>("cache-breakdown");
  sectionClicked = output<string>();

  sections = computed<SectionNav[]>(() => {
    const scan = this.scan();
    const items: SectionNav[] = [
      { id: "cache-breakdown", label: "Cache Performance", icon: "\u25A4" },
      { id: "task-timeline", label: "Timeline", icon: "\u2590" },
      { id: "tasks-table", label: "Tasks", icon: "\u2630" },
    ];
    if (scan.testCount > 0) {
      items.push({ id: "tests-table", label: "Tests", icon: "\u2713" });
    }
    return items;
  });

  cacheAvoidanceRate = computed(() => {
    const scan = this.scan();
    const edges = scan.tasks?.edges ?? [];
    if (edges.length === 0) return "\u2014";
    const avoided = edges.filter(
      (e: any) =>
        e.node.outcome === "FromCache" ||
        e.node.outcome === "UpToDate" ||
        e.node.outcome === "NoSource" ||
        e.node.outcome === "AvoidedForUnknownReason",
    ).length;
    const pct = Math.round((avoided / edges.length) * 100);
    return `${pct}%`;
  });

  navigateToSection(sectionId: string) {
    this.sectionClicked.emit(sectionId);
  }
}
