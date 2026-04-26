import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { TestBed, type ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { ScanTasksTabComponent } from "./scan-tasks-tab.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGFzazox",
      taskPath: ":compileJava",
      className: "JavaCompile",
      outcome: "Success",
      cacheable: true,
      durationMs: 120,
      cacheKey: "abc123",
      cachingDisabledReason: null,
      cachingDisabledExplanation: null,
      upToDateMessages: null,
      originBuildInvocationId: null,
      originExecutionTime: null,
      cacheOperations: [],
      ...overrides,
    },
    cursor: "c1",
  };
}

function buildTaskDependencyGraph(overrides: Record<string, unknown> = {}) {
  return {
    nodes: [{ id: "VGFzazox" }],
    edges: [],
    ...overrides,
  };
}

function buildTaskScan(
  edges: Array<ReturnType<typeof buildTaskEdge>> = [buildTaskEdge()],
  pageInfo: Record<string, unknown> = { hasNextPage: false, endCursor: null },
  taskDependencyGraph: Record<
    string,
    unknown
  > | null = buildTaskDependencyGraph(),
) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
    taskDependencyGraph,
    tasks: {
      edges,
      pageInfo,
    },
  };
}

describe("ScanTasksTabComponent", () => {
  let fixture: ComponentFixture<ScanTasksTabComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanTasksTabComponent],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanTasksTabComponent);
    fixture.componentRef.setInput("scanId", "123");
    fixture.componentRef.setInput("taskCount", 1);
    fixture.detectChanges();
  });

  afterEach(() => {
    fixture.destroy();
  });

  it("shows a loading state before the first result and then renders the task views", () => {
    const pending = controller.expectOne("GetScanTasks");
    const queryText = pending.operation.query.loc?.source.body ?? "";
    expect(pending.operation.variables).toEqual({
      id: "123",
      firstTasks: 100,
    });
    expect(queryText).toContain("taskDependencyGraph");
    expect(queryText).not.toContain("startTimestamp");
    expect(queryText).not.toContain("finishTimestamp");
    expect(fixture.nativeElement.textContent).toContain("Loading tasks…");

    pending.flushData({
      buildScan: buildTaskScan(
        [buildTaskEdge()],
        { hasNextPage: false, endCursor: null },
        buildTaskDependencyGraph({
          nodes: [{ id: "VGFzazox" }],
          edges: [],
        }),
      ),
    });
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector("app-cache-breakdown"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-task-timeline"),
    ).toBeTruthy();
    expect(fixture.nativeElement.textContent).toContain("Task Dependencies");
    expect(fixture.nativeElement.querySelector("app-tasks-table")).toBeTruthy();
    expect(fixture.nativeElement.querySelectorAll("tbody tr").length).toBe(1);
  });

  it("shows the graph component empty state when the dependency graph payload has no nodes", () => {
    const pending = controller.expectOne("GetScanTasks");

    pending.flushData({
      buildScan: buildTaskScan(
        [buildTaskEdge()],
        { hasNextPage: false, endCursor: null },
        buildTaskDependencyGraph({ nodes: [], edges: [] }),
      ),
    });
    fixture.detectChanges();

    expect(fixture.nativeElement.textContent).toContain(
      "No task dependency graph available.",
    );
  });

  it("continues loading additional task pages when pageInfo hasNextPage is true", async () => {
    const first = controller.expectOne("GetScanTasks");
    first.flushData({
      buildScan: buildTaskScan([buildTaskEdge()], {
        hasNextPage: true,
        endCursor: "c1",
      }),
    });
    fixture.detectChanges();

    const second = controller.expectOne("GetScanTasks");
    expect(second.operation.variables).toEqual({
      id: "123",
      firstTasks: 100,
      afterTasks: "c1",
    });

    second.flushData({
      buildScan: buildTaskScan(
        [
          buildTaskEdge({
            id: "VGFzazo2",
            taskPath: ":test",
            className: "TestTask",
            outcome: "FromCache",
            cacheable: true,
            durationMs: 55,
            cacheKey: "def456",
          }),
        ],
        { hasNextPage: false, endCursor: "c2" },
      ),
    });
    await new Promise((resolve) => setTimeout(resolve, 50));
    await fixture.whenStable();
    fixture.detectChanges();

    const rows = fixture.nativeElement.querySelectorAll("tbody tr");
    expect(rows.length).toBe(2);
    expect(fixture.nativeElement.textContent).toContain("Tasks (1)");
    expect(fixture.nativeElement.querySelector("app-tasks-table")).toBeTruthy();
  });

  it("skips querying when the overview already reports zero tasks", () => {
    const zeroFixture = TestBed.createComponent(ScanTasksTabComponent);
    zeroFixture.componentRef.setInput("scanId", "123");
    zeroFixture.componentRef.setInput("taskCount", 0);
    zeroFixture.detectChanges();

    controller
      .expectOne("GetScanTasks")
      .flushData({ buildScan: buildTaskScan() });
    fixture.detectChanges();

    expect(controller.match("GetScanTasks")).toHaveLength(0);
    expect(zeroFixture.nativeElement.textContent).toContain(
      "No tasks recorded for this scan.",
    );

    zeroFixture.destroy();
  });
});
