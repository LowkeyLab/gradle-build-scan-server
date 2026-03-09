import { describe, it, expect, beforeEach } from 'vitest';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { ApolloTestingModule, ApolloTestingController } from 'apollo-angular/testing';
import { provideRouter } from '@angular/router';
import { ScanListComponent } from './scan-list.component';

function buildMockEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: 'QnVpbGRTY2FuOjE=',
      scanId: 'scan-1',
      buildToolType: 'Gradle',
      buildToolVersion: '8.0',
      outcome: 'success',
      createdAt: '2026-01-15T10:00:00Z',
      hostname: 'ci-host',
      taskCount: 5,
      testCount: 12,
      ...overrides,
    },
    cursor: 'c1',
  };
}

describe('ScanListComponent', () => {
  let fixture: ComponentFixture<ScanListComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanListComponent],
      providers: [provideRouter([])],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanListComponent);
    fixture.detectChanges();
  });

  function flushQuery(edges = [buildMockEdge()]) {
    const op = controller.expectOne('GetBuildScans');
    op.flushData({
      buildScans: {
        edges,
        pageInfo: { hasNextPage: false, endCursor: null },
        totalCount: edges.length,
      },
    });
    fixture.detectChanges();
  }

  it('should render Tests column header', () => {
    flushQuery();
    const headers = fixture.nativeElement.querySelectorAll('th');
    const texts = Array.from(headers).map((h: any) => h.textContent.trim());
    expect(texts).toContain('Tests');
  });

  it('should display testCount in the Tests column', () => {
    flushQuery([buildMockEdge({ testCount: 42 })]);
    const cells = fixture.nativeElement.querySelectorAll('tbody td');
    const cellTexts = Array.from(cells).map((c: any) => c.textContent.trim());
    expect(cellTexts).toContain('42');
  });

  it('should display taskCount and testCount side by side', () => {
    flushQuery([buildMockEdge({ taskCount: 5, testCount: 12 })]);
    const cells = fixture.nativeElement.querySelectorAll('tbody td');
    const cellTexts = Array.from(cells).map((c: any) => c.textContent.trim());
    expect(cellTexts).toContain('5');
    expect(cellTexts).toContain('12');
  });

  it('should show 0 for testCount when no tests', () => {
    flushQuery([buildMockEdge({ testCount: 0 })]);
    const row = fixture.nativeElement.querySelector('tbody tr');
    const cells = row.querySelectorAll('td');
    // testCount is the last column (index 5: Time, Outcome, Build Tool, Hostname, Tasks, Tests)
    expect(cells[5].textContent.trim()).toBe('0');
  });
});
