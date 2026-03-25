import {
  Component,
  ChangeDetectionStrategy,
  input,
  inject,
  signal,
} from "@angular/core";
import { Apollo, gql } from "apollo-angular";
import { AsyncPipe, DOCUMENT } from "@angular/common";
import { filter, map, switchMap, tap } from "rxjs";
import { toObservable } from "@angular/core/rxjs-interop";
import { TaskTimelineComponent } from "./task-timeline/task-timeline.component";
import { TasksTableComponent } from "./tasks-table/tasks-table.component";
import { TestsTableComponent } from "./tests-table/tests-table.component";
import { CacheBreakdownComponent } from "./cache-breakdown/cache-breakdown.component";
import { ScanSidebarComponent } from "./scan-sidebar/scan-sidebar.component";

const GET_BUILD_SCAN = gql`
  query GetBuildScan(
    $id: ID!
    $firstTasks: Int!
    $afterTasks: String
    $includeTests: Boolean!
    $firstTests: Int!
    $afterTests: String
  ) {
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
      tasks(first: $firstTasks, after: $afterTasks) {
        edges {
          node {
            id
            taskPath
            className
            outcome
            cacheable
            durationMs
            startTimestamp
            finishTimestamp
            cacheKey
            cachingDisabledReason
            cachingDisabledExplanation
          }
          cursor
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
      tests(first: $firstTests, after: $afterTests)
        @include(if: $includeTests) {
        edges {
          node {
            id
            className
            methodName
            executorName
            outcome
          }
          cursor
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
`;

@Component({
  selector: "app-scan-detail",
  imports: [
    AsyncPipe,
    TaskTimelineComponent,
    TasksTableComponent,
    TestsTableComponent,
    CacheBreakdownComponent,
    ScanSidebarComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (scan$ | async; as scan) {
      <div class="grid grid-cols-[260px_1fr] h-screen">
        <app-scan-sidebar
          [scan]="scan"
          [activeSection]="activeSection()"
          (sectionClicked)="scrollToSection($event)"
        />
        <main class="overflow-y-auto p-6 scroll-smooth">
          <section id="cache-breakdown" class="scroll-mt-4">
            <app-cache-breakdown
              [taskEdges]="scan.tasks.edges"
              [taskCount]="scan.taskCount"
            />
          </section>
          <section id="task-timeline" class="scroll-mt-4">
            <app-task-timeline [taskEdges]="scan.tasks.edges" />
          </section>
          <section id="tasks-table" class="scroll-mt-4">
            <app-tasks-table
              [taskEdges]="scan.tasks.edges"
              [taskCount]="scan.taskCount"
            />
          </section>
          @if (scan.tests) {
            <section id="tests-table" class="scroll-mt-4">
              <app-tests-table
                [testEdges]="scan.tests.edges"
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
  private apollo = inject(Apollo);
  private doc = inject(DOCUMENT);
  activeSection = signal("cache-breakdown");

  scan$ = toObservable(this.id).pipe(
    switchMap((id) => {
      const queryRef = this.apollo.watchQuery<any>({
        query: GET_BUILD_SCAN,
        variables: {
          id,
          firstTasks: 100,
          firstTests: 100,
          includeTests: true,
        },
        errorPolicy: "all",
      });

      return queryRef.valueChanges.pipe(
        filter((result) => !!result.data),
        map((result) => result.data.buildScan),
        tap((scan) => {
          if (scan.tasks.pageInfo.hasNextPage) {
            queryRef.fetchMore({
              variables: {
                id,
                firstTasks: 100,
                afterTasks: scan.tasks.pageInfo.endCursor,
                includeTests: false,
                firstTests: 0,
              },
            });
          }
        }),
      );
    }),
  );

  scrollToSection(sectionId: string) {
    this.activeSection.set(sectionId);
    const el = this.doc.getElementById(sectionId);
    el?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
}
