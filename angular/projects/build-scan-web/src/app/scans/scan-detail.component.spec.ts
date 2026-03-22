import { describe, it, expect, beforeEach } from 'vitest';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { ApolloTestingModule, ApolloTestingController } from 'apollo-angular/testing';
import { provideRouter } from '@angular/router';
import { ScanDetailComponent } from './scan-detail.component';

function buildMockScan(overrides: Record<string, unknown> = {}) {
  return {
    id: 'QnVpbGRTY2FuOjEyMw==',
    scanId: '123',
    buildToolType: 'Gradle',
    buildToolVersion: '8.0',
    pluginVersion: '3.0',
    outcome: 'success',
    createdAt: '2026-01-15T10:00:00Z',
    hostname: 'ci-host',
    osName: 'Linux',
    osVersion: '6.0',
    jvmVendor: 'Eclipse',
    jvmVersion: '21',
    requestedTasks: ['build'],
    taskCount: 1,
    testCount: 0,
    tasks: {
      edges: [
        {
          node: {
            id: 'VGFzazox',
            taskPath: ':compileJava',
            className: 'JavaCompile',
            outcome: 'Success',
            cacheable: true,
            durationMs: 120,
            startTimestamp: 1000,
            finishTimestamp: 1120,
            cacheKey: 'abc123',
            cachingDisabledReason: null,
            cachingDisabledExplanation: null,
          },
          cursor: 'c1',
        },
      ],
      pageInfo: { hasNextPage: false, endCursor: null },
    },
    tests: {
      edges: [],
      pageInfo: { hasNextPage: false, endCursor: null },
    },
    ...overrides,
  };
}

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: 'VGFzazox',
      taskPath: ':compileJava',
      className: 'JavaCompile',
      outcome: 'Success',
      cacheable: true,
      durationMs: 120,
      startTimestamp: 1000,
      finishTimestamp: 1120,
      cacheKey: 'abc123',
      cachingDisabledReason: null,
      cachingDisabledExplanation: null,
      ...overrides,
    },
    cursor: 'c1',
  };
}

function buildTestEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: 'VGVzdDox',
      className: 'com.example.FooTest',
      methodName: 'testSomething',
      executorName: 'Gradle Test Executor 1',
      outcome: 'Passed',
      ...overrides,
    },
    cursor: 'tc1',
  };
}

describe('ScanDetailComponent', () => {
  let fixture: ComponentFixture<ScanDetailComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanDetailComponent],
      providers: [provideRouter([])],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanDetailComponent);
    fixture.componentRef.setInput('id', '123');
    fixture.detectChanges();
  });

  function flushQuery(scanOverrides: Record<string, unknown> = {}) {
    const op = controller.expectOne('GetBuildScan');
    op.flushData({ buildScan: buildMockScan(scanOverrides) });
    fixture.detectChanges();
  }

  it('should display taskCount in the tasks heading', () => {
    flushQuery({ taskCount: 42 });
    const heading = fixture.nativeElement.querySelector('h3');
    expect(heading.textContent).toContain('Tasks (42)');
  });

  it('should hide tests section when testCount is 0', () => {
    flushQuery({ testCount: 0 });
    const headings = fixture.nativeElement.querySelectorAll('h3');
    const texts = Array.from(headings).map((h: any) => h.textContent);
    expect(texts.some((t: string) => t.includes('Tests'))).toBe(false);
  });

  it('should show tests section when testCount > 0', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge()],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const headings = fixture.nativeElement.querySelectorAll('h3');
    const texts = Array.from(headings).map((h: any) => h.textContent);
    expect(texts.some((t: string) => t.includes('Tests (1)'))).toBe(true);
  });

  it('should render test table columns', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge()],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const testsTable = tables[1];
    const headers = testsTable.querySelectorAll('th');
    expect(Array.from(headers).map((h: any) => h.textContent.trim())).toEqual([
      'Class Name',
      'Method Name',
      'Outcome',
      'Executor',
    ]);
  });

  it('should render test row data', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge()],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const testsTable = tables[1];
    const cells = testsTable.querySelectorAll('tbody td');
    expect(cells[0].textContent.trim()).toBe('com.example.FooTest');
    expect(cells[1].textContent.trim()).toBe('testSomething');
    expect(cells[2].textContent).toContain('Passed');
    expect(cells[3].textContent.trim()).toBe('Gradle Test Executor 1');
  });

  it('should apply badge-success for Passed outcome', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge({ outcome: 'Passed' })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const badge = tables[1].querySelector('.badge');
    expect(badge.classList.contains('badge-success')).toBe(true);
  });

  it('should apply badge-error for Failed outcome', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge({ outcome: 'Failed' })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const badge = tables[1].querySelector('.badge');
    expect(badge.classList.contains('badge-error')).toBe(true);
  });

  it('should apply badge-warning for Skipped outcome', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge({ outcome: 'Skipped' })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const badge = tables[1].querySelector('.badge');
    expect(badge.classList.contains('badge-warning')).toBe(true);
  });

  it('should show dash for null methodName', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge({ methodName: null })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const cells = tables[1].querySelectorAll('tbody td');
    expect(cells[1].textContent.trim()).toBe('—');
  });

  it('should show dash for null executorName', () => {
    flushQuery({
      testCount: 1,
      tests: {
        edges: [buildTestEdge({ executorName: null })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const cells = tables[1].querySelectorAll('tbody td');
    expect(cells[3].textContent.trim()).toBe('—');
  });

  it('should show Cache Hit badge for FromCache outcome', () => {
    flushQuery({
      tasks: {
        edges: [buildTaskEdge({ outcome: 'FromCache' })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const table = fixture.nativeElement.querySelector('table');
    const cells = table.querySelectorAll('tbody td');
    expect(cells[4].textContent.trim()).toBe('Cache Hit');
    expect(cells[4].querySelector('.badge-info')).toBeTruthy();
  });

  it('should show Up-to-date badge for UpToDate outcome', () => {
    flushQuery({
      tasks: {
        edges: [buildTaskEdge({ outcome: 'UpToDate' })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const table = fixture.nativeElement.querySelector('table');
    const cells = table.querySelectorAll('tbody td');
    expect(cells[4].textContent.trim()).toBe('Up-to-date');
    expect(cells[4].querySelector('.badge-success')).toBeTruthy();
  });

  it('should show caching disabled reason as warning badge with tooltip', () => {
    flushQuery({
      tasks: {
        edges: [buildTaskEdge({
          outcome: 'Success',
          cacheable: false,
          cachingDisabledReason: 'NOT_ENABLED_FOR_TASK',
          cachingDisabledExplanation: 'Not worth caching',
        })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const table = fixture.nativeElement.querySelector('table');
    const cells = table.querySelectorAll('tbody td');
    expect(cells[4].textContent.trim()).toBe('NOT_ENABLED_FOR_TASK');
    expect(cells[4].querySelector('.badge-warning')).toBeTruthy();
    const tooltip = cells[4].querySelector('.tooltip');
    expect(tooltip.getAttribute('data-tip')).toBe('Not worth caching');
  });

  it('should show Executed badge for cacheable task without disabled reason', () => {
    flushQuery({
      tasks: {
        edges: [buildTaskEdge({ outcome: 'Success', cacheable: true, cachingDisabledReason: null })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const table = fixture.nativeElement.querySelector('table');
    const cells = table.querySelectorAll('tbody td');
    expect(cells[4].textContent.trim()).toBe('Executed');
    expect(cells[4].querySelector('.badge-ghost')).toBeTruthy();
  });

  it('should show dash for non-cacheable task without disabled reason', () => {
    flushQuery({
      tasks: {
        edges: [buildTaskEdge({ outcome: 'Success', cacheable: false, cachingDisabledReason: null })],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const table = fixture.nativeElement.querySelector('table');
    const cells = table.querySelectorAll('tbody td');
    expect(cells[4].textContent.trim()).toBe('—');
  });

  it('should render multiple test rows', () => {
    flushQuery({
      testCount: 2,
      tests: {
        edges: [
          buildTestEdge(),
          {
            node: {
              id: 'VGVzdDoy',
              className: 'com.example.BarTest',
              methodName: 'testOther',
              executorName: null,
              outcome: 'Failed',
            },
            cursor: 'tc2',
          },
        ],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const tables = fixture.nativeElement.querySelectorAll('table');
    const rows = tables[1].querySelectorAll('tbody tr');
    expect(rows.length).toBe(2);
  });

  it('should render timeline with worker lanes for overlapping tasks', () => {
    flushQuery({
      taskCount: 2,
      tasks: {
        edges: [
          buildTaskEdge({ id: 'T1', taskPath: ':compileJava', startTimestamp: 1000, finishTimestamp: 1120 }),
          buildTaskEdge({ id: 'T2', taskPath: ':processResources', startTimestamp: 1050, finishTimestamp: 1080 }),
        ],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const timeline = fixture.nativeElement.querySelector('.card.bg-base-200');
    expect(timeline).toBeTruthy();
    const heading = timeline.querySelector('h4');
    expect(heading.textContent).toContain('Timeline');
    expect(heading.textContent).toContain('2 workers');
    // Overlapping tasks should be in separate lanes
    const workerLabels = timeline.querySelectorAll('.opacity-50');
    // First two .opacity-50 spans are worker labels
    expect(workerLabels[0].textContent.trim()).toBe('Worker 1');
    expect(workerLabels[1].textContent.trim()).toBe('Worker 2');
  });

  it('should assign sequential tasks to the same lane', () => {
    flushQuery({
      taskCount: 2,
      tasks: {
        edges: [
          buildTaskEdge({ id: 'T1', taskPath: ':a', startTimestamp: 0, finishTimestamp: 100 }),
          buildTaskEdge({ id: 'T2', taskPath: ':b', startTimestamp: 100, finishTimestamp: 200 }),
        ],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const timeline = fixture.nativeElement.querySelector('.card.bg-base-200');
    expect(timeline.textContent).toContain('1 worker');
    // Both tasks in one lane = one row with relative positioning
    const bars = timeline.querySelectorAll('.rounded-sm');
    expect(bars.length).toBe(2);
  });

  it('should color-code timeline bars by outcome', () => {
    // T1 and T2 overlap, so they go to separate lanes.
    // T3 starts when T1 ends, so it joins lane 1.
    // Lane 1: [T1(Success), T3(Failed)], Lane 2: [T2(FromCache)]
    flushQuery({
      taskCount: 3,
      tasks: {
        edges: [
          buildTaskEdge({ id: 'T1', taskPath: ':a', outcome: 'Success', startTimestamp: 0, finishTimestamp: 100 }),
          buildTaskEdge({ id: 'T2', taskPath: ':b', outcome: 'FromCache', startTimestamp: 50, finishTimestamp: 150 }),
          buildTaskEdge({ id: 'T3', taskPath: ':c', outcome: 'Failed', startTimestamp: 100, finishTimestamp: 200 }),
        ],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const timeline = fixture.nativeElement.querySelector('.card.bg-base-200');
    const bars = timeline.querySelectorAll('.rounded-sm');
    // DOM order: lane 1 bars first (T1, T3), then lane 2 (T2)
    expect(bars[0].classList.contains('bg-success')).toBe(true);  // T1
    expect(bars[1].classList.contains('bg-error')).toBe(true);    // T3
    expect(bars[2].classList.contains('bg-info')).toBe(true);     // T2
  });

  it('should exclude tasks without timestamps from timeline', () => {
    flushQuery({
      taskCount: 2,
      tasks: {
        edges: [
          buildTaskEdge({ id: 'T1', taskPath: ':compileJava', startTimestamp: 1000, finishTimestamp: 1120 }),
          buildTaskEdge({ id: 'T2', taskPath: ':noTimestamps', startTimestamp: null, finishTimestamp: null }),
        ],
        pageInfo: { hasNextPage: false, endCursor: null },
      },
    });
    const timeline = fixture.nativeElement.querySelector('.card.bg-base-200');
    expect(timeline).toBeTruthy();
    const bars = timeline.querySelectorAll('.rounded-sm');
    expect(bars.length).toBe(1);
  });
});
