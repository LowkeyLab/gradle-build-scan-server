import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { TestBed, type ComponentFixture } from "@angular/core/testing";
import { By } from "@angular/platform-browser";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { ScanTasksTabComponent } from "./scan-tasks-tab.component";
import { TaskDependencyGraphComponent } from "./task-dependency-graph/task-dependency-graph.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGFzazox",
      dependencies: [],
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

function buildTaskScan(
  edges: Array<ReturnType<typeof buildTaskEdge>> = [buildTaskEdge()],
  pageInfo: Record<string, unknown> = { hasNextPage: false, endCursor: null },
) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
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
  });

  function createFixture(taskCount = 1) {
    fixture = TestBed.createComponent(ScanTasksTabComponent);
    fixture.componentRef.setInput("scanId", "123");
    fixture.componentRef.setInput("taskCount", taskCount);
    fixture.detectChanges();
    return fixture;
  }

  afterEach(() => {
    fixture?.destroy();
  });

  it("shows a loading state before the first result and then renders the task views", () => {
    createFixture();
    const pending = controller.expectOne("GetScanTasks");
    const queryText = pending.operation.query.loc?.source.body ?? "";
    expect(pending.operation.variables).toEqual({
      id: "123",
      firstTasks: 100,
    });
    expect(queryText).toContain("dependencies");
    expect(queryText).not.toContain("taskDependencyGraph");
    expect(queryText).not.toContain("startTimestamp");
    expect(queryText).not.toContain("finishTimestamp");
    expect(fixture.nativeElement.textContent).toContain("Loading tasks…");

    pending.flushData({
      buildScan: buildTaskScan([buildTaskEdge()], {
        hasNextPage: false,
        endCursor: null,
      }),
    });
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector("app-cache-breakdown"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-task-dependency-graph"),
    ).toBeTruthy();
    expect(fixture.nativeElement.textContent).toContain("Task Dependencies");
    expect(fixture.nativeElement.querySelector("app-tasks-table")).toBeTruthy();
    expect(fixture.nativeElement.querySelectorAll("tbody tr").length).toBe(1);
  });

  it("passes per-task dependencies through to the task timeline", () => {
    createFixture();
    const pending = controller.expectOne("GetScanTasks");

    pending.flushData({
      buildScan: buildTaskScan(
        [
          buildTaskEdge({
            id: "VGFzazoy",
            dependencies: ["VGFzazox"],
          }),
        ],
        { hasNextPage: false, endCursor: null },
      ),
    });
    fixture.detectChanges();

    const timeline = fixture.componentInstance.taskEdges();
    expect(timeline[0]?.node.dependencies).toEqual(["VGFzazox"]);
  });

  it("continues loading additional task pages when pageInfo hasNextPage is true", async () => {
    createFixture();
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

  it("renders only the critical path by default for very large scans", () => {
    const largeFixture = createFixture(1000);

    const pending = controller.expectOne("GetScanTasks");
    pending.flushData({
      buildScan: buildTaskScan(
        [
          buildTaskEdge({
            id: "ROOT",
            taskPath: ":root",
            durationMs: 10,
          }),
          buildTaskEdge({
            id: "FAST",
            dependencies: ["ROOT"],
            taskPath: ":fast",
            durationMs: 20,
          }),
          buildTaskEdge({
            id: "SLOW",
            dependencies: ["ROOT"],
            taskPath: ":slow",
            durationMs: 80,
          }),
          buildTaskEdge({
            id: "END",
            dependencies: ["SLOW"],
            taskPath: ":end",
            durationMs: 30,
          }),
        ],
        { hasNextPage: false, endCursor: null },
      ),
    });
    largeFixture.detectChanges();

    const graphDebugElement = largeFixture.debugElement.query(
      By.directive(TaskDependencyGraphComponent),
    );

    expect(graphDebugElement).toBeTruthy();
    const graphComponent =
      graphDebugElement.componentInstance as TaskDependencyGraphComponent;
    expect(graphComponent.taskEdges().map((edge) => edge.node.id)).toEqual([
      "ROOT",
      "SLOW",
      "END",
    ]);
    expect(largeFixture.nativeElement.textContent).toContain("Critical Path");
    expect(largeFixture.nativeElement.textContent).toContain(
      "Showing the longest weighted dependency chain",
    );

    const buttons = Array.from(
      (largeFixture.nativeElement as HTMLElement).querySelectorAll("button"),
    ) as HTMLButtonElement[];

    const button = buttons.find((candidate) =>
      candidate.textContent?.includes("Render full graph anyway"),
    );

    expect(button).toBeUndefined();

    largeFixture.destroy();
  });

  it("uses origin execution time when duration is unavailable for critical path selection", () => {
    const largeFixture = createFixture(1000);

    const pending = controller.expectOne("GetScanTasks");
    pending.flushData({
      buildScan: buildTaskScan(
        [
          buildTaskEdge({
            id: "ROOT",
            taskPath: ":root",
            durationMs: 10,
          }),
          buildTaskEdge({
            id: "DURATION_PATH",
            dependencies: ["ROOT"],
            taskPath: ":durationPath",
            durationMs: 20,
            originExecutionTime: 20,
          }),
          buildTaskEdge({
            id: "ORIGIN_PATH",
            dependencies: ["ROOT"],
            taskPath: ":originPath",
            durationMs: null,
            originExecutionTime: 90,
          }),
        ],
        { hasNextPage: false, endCursor: null },
      ),
    });
    largeFixture.detectChanges();

    const graphComponent = largeFixture.debugElement.query(
      By.directive(TaskDependencyGraphComponent),
    ).componentInstance as TaskDependencyGraphComponent;

    expect(graphComponent.taskEdges().map((edge) => edge.node.id)).toEqual([
      "ROOT",
      "ORIGIN_PATH",
    ]);

    largeFixture.destroy();
  });

  it("skips querying when the overview already reports zero tasks", () => {
    const zeroFixture = createFixture(0);
    zeroFixture.detectChanges();

    expect(controller.match("GetScanTasks")).toHaveLength(0);
    expect(zeroFixture.nativeElement.textContent).toContain(
      "No tasks recorded for this scan.",
    );

    zeroFixture.destroy();
  });
});
