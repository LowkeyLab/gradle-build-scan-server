import {
  Component,
  ChangeDetectionStrategy,
  input,
  inject,
} from "@angular/core";
import { Apollo, gql } from "apollo-angular";
import { AsyncPipe } from "@angular/common";
import { RouterLink } from "@angular/router";
import { filter, map, switchMap, tap } from "rxjs";
import { toObservable } from "@angular/core/rxjs-interop";
import { BuildMetadataComponent } from "./build-metadata/build-metadata.component";
import { TaskTimelineComponent } from "./task-timeline/task-timeline.component";
import { TasksTableComponent } from "./tasks-table/tasks-table.component";
import { TestsTableComponent } from "./tests-table/tests-table.component";
import { CacheBreakdownComponent } from "./cache-breakdown/cache-breakdown.component";

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
      tests(first: $firstTests, after: $afterTests) @include(if: $includeTests) {
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
    RouterLink,
    BuildMetadataComponent,
    TaskTimelineComponent,
    TasksTableComponent,
    TestsTableComponent,
    CacheBreakdownComponent,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="container mx-auto p-6">
      @if (scan$ | async; as scan) {
        <div class="mb-4">
          <a routerLink="/scans" class="link link-primary">&larr; All Scans</a>
        </div>

        <app-build-metadata [scan]="scan" />
        <app-cache-breakdown
          [taskEdges]="scan.tasks.edges"
          [taskCount]="scan.taskCount"
        />
        <app-task-timeline [taskEdges]="scan.tasks.edges" />
        <app-tasks-table
          [taskEdges]="scan.tasks.edges"
          [taskCount]="scan.taskCount"
        />
        <app-tests-table
          [testEdges]="scan.tests.edges"
          [testCount]="scan.testCount"
        />
      }
    </div>
  `,
})
export class ScanDetailComponent {
  id = input.required<string>();
  private apollo = inject(Apollo);

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
}
