import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  OnInit,
  inject,
  input,
  signal,
} from "@angular/core";
import { Apollo, gql } from "apollo-angular";
import { EMPTY, filter, map, switchMap, tap } from "rxjs";
import { takeUntilDestroyed, toObservable } from "@angular/core/rxjs-interop";
import { TestsTableComponent } from "./tests-table/tests-table.component";

const GET_SCAN_TESTS = gql`
  query GetScanTests($id: ID!, $firstTests: Int!) {
    buildScan(id: $id) {
      id
      testCount
      testSummary {
        passed
        failed
        skipped
        totalDurationMs
      }
      tests(first: $firstTests) {
        edges {
          node {
            id
            className
            methodName
            executorName
            outcome
            durationMs
            failureMessage
            failureStacktrace
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
  selector: "app-scan-tests-tab",
  imports: [TestsTableComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="space-y-6">
      <h2 class="text-2xl font-bold">Tests</h2>

      @if (loading()) {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          Loading tests…
        </div>
      } @else if (testCount() > 0) {
        <app-tests-table
          [testEdges]="testEdges()"
          [testCount]="testCount()"
          [testSummary]="testSummary()"
        />
      } @else {
        <div
          class="rounded-md border border-base-300 bg-base-200 p-4 text-sm opacity-70"
        >
          No tests recorded for this scan.
        </div>
      }
    </div>
  `,
})
export class ScanTestsTabComponent implements OnInit {
  scanId = input.required<string>();
  testCount = input.required<number>();

  private apollo = inject(Apollo);
  private destroyRef = inject(DestroyRef);
  private scanId$ = toObservable(this.scanId);

  testEdges = signal<any[]>([]);
  testSummary = signal<any | null>(null);
  loading = signal(true);

  ngOnInit() {
    this.scanId$
      .pipe(
        switchMap((id) => {
          this.testEdges.set([]);
          this.testSummary.set(null);
          if (this.testCount() === 0) {
            this.loading.set(false);
            return EMPTY;
          }

          this.loading.set(true);

          return this.apollo
            .watchQuery<any>({
              query: GET_SCAN_TESTS,
              variables: {
                id,
                firstTests: 100,
              },
              errorPolicy: "all",
            })
            .valueChanges.pipe(
              filter((result) => !!result.data),
              map((result) => result.data.buildScan),
              tap((scan) => {
                this.testEdges.set(scan.tests.edges);
                this.testSummary.set(scan.testSummary ?? null);
                this.loading.set(false);
              }),
            );
        }),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe();
  }
}
