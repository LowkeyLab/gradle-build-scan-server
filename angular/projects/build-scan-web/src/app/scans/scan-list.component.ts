import { Component, ChangeDetectionStrategy, inject } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Apollo, gql } from 'apollo-angular';
import { AsyncPipe, DatePipe } from '@angular/common';
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
  imports: [RouterLink, AsyncPipe, DatePipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="container mx-auto p-6">
      <h1 class="text-3xl font-bold mb-6">Build Scans</h1>

      @if (scans$ | async; as data) {
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

        @if (hasNextPage) {
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

  scans$ = this.queryRef.valueChanges.pipe(
    map(result => result.data.buildScans)
  );

  hasNextPage = false;
  endCursor: string | null = null;

  constructor() {
    this.queryRef.valueChanges.subscribe(result => {
      this.hasNextPage = result.data.buildScans.pageInfo.hasNextPage;
      this.endCursor = result.data.buildScans.pageInfo.endCursor;
    });
  }

  loadMore() {
    if (!this.endCursor) return;
    this.queryRef.fetchMore({
      variables: { first: 20, after: this.endCursor },
    });
  }
}
