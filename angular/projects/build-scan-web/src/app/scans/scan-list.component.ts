import { Component, ChangeDetectionStrategy, inject, computed } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Apollo, gql } from 'apollo-angular';
import { DatePipe } from '@angular/common';
import { toSignal } from '@angular/core/rxjs-interop';
import { map } from 'rxjs';

const GET_BUILD_SCANS = gql`
  query GetBuildScans($first: Int!, $after: String) {
    buildScans(first: $first, after: $after) {
      edges {
        node {
          id
          scanId
          buildToolType
          buildToolVersion
          outcome
          createdAt
          hostname
          taskCount
        }
        cursor
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

@Component({
  selector: 'app-scan-list',
  imports: [RouterLink, DatePipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="container mx-auto p-6">
      <h1 class="text-3xl font-bold mb-6">Build Scans</h1>

      @if (scans(); as data) {
        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>Time</th>
                <th>Outcome</th>
                <th>Build Tool</th>
                <th>Hostname</th>
                <th>Tasks</th>
              </tr>
            </thead>
            <tbody>
              @for (edge of data.edges; track edge.node.id) {
                <tr class="hover cursor-pointer">
                  <td>
                    <a [routerLink]="['/scans', edge.node.scanId]" class="link link-primary">
                      {{ edge.node.createdAt | date:'medium' }}
                    </a>
                  </td>
                  <td>
                    <span class="badge"
                      [class.badge-success]="edge.node.outcome === 'success'"
                      [class.badge-error]="edge.node.outcome === 'failed'"
                      [class.badge-warning]="edge.node.outcome === 'parse_error'">
                      {{ edge.node.outcome }}
                    </span>
                  </td>
                  <td>{{ edge.node.buildToolType }} {{ edge.node.buildToolVersion }}</td>
                  <td>{{ edge.node.hostname || '—' }}</td>
                  <td>{{ edge.node.taskCount }}</td>
                </tr>
              }
            </tbody>
          </table>
        </div>

        @if (hasNextPage()) {
          <div class="mt-4">
            <button class="btn btn-outline" (click)="loadMore()">Load More</button>
          </div>
        }
      }
    </div>
  `,
})
export class ScanListComponent {
  private apollo = inject(Apollo);
  private queryRef = this.apollo.watchQuery<any>({
    query: GET_BUILD_SCANS,
    variables: { first: 20 },
  });

  scans = toSignal(this.queryRef.valueChanges.pipe(
    map(r => r.data.buildScans)
  ));

  hasNextPage = computed(() => this.scans()?.pageInfo.hasNextPage ?? false);
  endCursor = computed(() => this.scans()?.pageInfo.endCursor ?? null);

  loadMore() {
    const cursor = this.endCursor();
    if (!cursor) return;
    this.queryRef.fetchMore({
      variables: { first: 20, after: cursor },
    });
  }
}
