import {
  Component,
  ChangeDetectionStrategy,
  effect,
  input,
  inject,
  signal,
} from "@angular/core";
import { AsyncPipe } from "@angular/common";
import { Apollo, gql } from "apollo-angular";
import { filter, map, switchMap } from "rxjs";
import { toObservable } from "@angular/core/rxjs-interop";
import { BuildMetadataComponent } from "./build-metadata/build-metadata.component";
import { ScanDependenciesTabComponent } from "./scan-dependencies-tab.component";
import {
  ScanSidebarComponent,
  type ScanTab,
} from "./scan-sidebar/scan-sidebar.component";
import { ScanTasksTabComponent } from "./scan-tasks-tab.component";
import { ScanTestsTabComponent } from "./scan-tests-tab.component";

interface BuildScanOverview {
  id: string;
  scanId: string;
  buildToolType: string;
  buildToolVersion: string;
  pluginVersion: string;
  outcome: string | null;
  createdAt: string;
  hostname: string | null;
  osName: string | null;
  osVersion: string | null;
  jvmVendor: string | null;
  jvmVersion: string | null;
  requestedTasks: string[];
  taskCount: number;
  testCount: number;
}

interface GetBuildScanOverviewData {
  buildScan: BuildScanOverview | null;
}

interface GetBuildScanOverviewVariables {
  id: string;
}

const GET_BUILD_SCAN_OVERVIEW = gql`
  query GetBuildScanOverview($id: ID!) {
    buildScan(id: $id) {
      id
      scanId
      buildToolType
      buildToolVersion
      pluginVersion
      outcome
      createdAt
      hostname
      osName
      osVersion
      jvmVendor
      jvmVersion
      requestedTasks
      taskCount
      testCount
    }
  }
`;

@Component({
  selector: "app-scan-detail",
  imports: [
    AsyncPipe,
    BuildMetadataComponent,
    ScanDependenciesTabComponent,
    ScanSidebarComponent,
    ScanTasksTabComponent,
    ScanTestsTabComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (scan$ | async; as scan) {
      <div class="grid grid-cols-[260px_1fr] h-screen">
        <app-scan-sidebar
          [scan]="scan"
          [activeTab]="selectedTab()"
          (tabClicked)="selectTab($event)"
        />

        <main class="overflow-y-auto p-6">
          @if (selectedTab() === "overview") {
            <section>
              <app-build-metadata [scan]="scan" />
            </section>
          }

          @if (isTabMounted("tasks")) {
            <section [hidden]="selectedTab() !== 'tasks'">
              <app-scan-tasks-tab
                [scanId]="scan.id"
                [taskCount]="scan.taskCount"
              />
            </section>
          }

          @if (isTabMounted("dependencies")) {
            <section [hidden]="selectedTab() !== 'dependencies'">
              <app-scan-dependencies-tab [scanId]="scan.id" />
            </section>
          }

          @if (isTabMounted("tests")) {
            <section [hidden]="selectedTab() !== 'tests'">
              <app-scan-tests-tab
                [scanId]="scan.id"
                [testCount]="scan.testCount"
              />
            </section>
          }
        </main>
      </div>
    }
  `,
  host: { class: "block h-screen overflow-hidden" },
})
export class ScanDetailComponent {
  id = input.required<string>();
  selectedTab = signal<ScanTab>("overview");
  visitedTabs = signal<Set<ScanTab>>(new Set<ScanTab>(["overview"]));

  private apollo = inject(Apollo);
  private activeScanId = signal<string | null>(null);

  scan$ = toObservable(this.id).pipe(
    switchMap((id) =>
      this.apollo
        .watchQuery<GetBuildScanOverviewData, GetBuildScanOverviewVariables>({
          query: GET_BUILD_SCAN_OVERVIEW,
          variables: { id },
          errorPolicy: "all",
        })
        .valueChanges.pipe(
          filter((result) => !!result.data),
          map((result) => result.data.buildScan),
          filter((scan): scan is BuildScanOverview => scan !== null),
        ),
    ),
  );

  constructor() {
    effect(() => {
      const id = this.id();
      if (this.activeScanId() === id) return;
      this.activeScanId.set(id);
      this.selectedTab.set("overview");
      this.visitedTabs.set(new Set<ScanTab>(["overview"]));
    });
  }

  selectTab(tab: ScanTab) {
    this.selectedTab.set(tab);
    this.visitedTabs.update((tabs) => new Set(tabs).add(tab));
  }

  isTabMounted(tab: ScanTab) {
    return this.visitedTabs().has(tab);
  }
}
