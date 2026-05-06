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
import { ScanSidebarComponent } from "./scan-sidebar/scan-sidebar.component";
import { ScanTasksTabComponent } from "./scan-tasks-tab.component";
import { ScanTestsTabComponent } from "./scan-tests-tab.component";

type ScanTab = "overview" | "tasks" | "tests";

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
                [requestedTasks]="scan.requestedTasks ?? []"
              />
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
        .watchQuery<any>({
          query: GET_BUILD_SCAN_OVERVIEW,
          variables: { id },
          errorPolicy: "all",
        })
        .valueChanges.pipe(
          filter((result) => !!result.data),
          map((result) => result.data.buildScan),
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
