import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { TestBed, type ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { ScanTasksTabComponent } from "./scan-tasks-tab.component";

vi.mock("@antv/g6", () => ({
  CanvasEvent: { CLICK: "canvas:click" },
  Graph: class MockGraph {
    destroy = vi.fn((): void => undefined);
    setData = vi.fn((_data: unknown): void => undefined);

    render(): Promise<void> {
      return Promise.resolve();
    }
    fitView(): Promise<void> {
      return Promise.resolve();
    }
    setElementState(_states: unknown): Promise<void> {
      return Promise.resolve();
    }
    on(): MockGraph {
      return this;
    }
  },
  NodeEvent: {
    CLICK: "node:click",
    POINTER_ENTER: "node:pointerenter",
    POINTER_LEAVE: "node:pointerleave",
  },
}));

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGFzazox",
      dependencies: [],
      taskPath: ":compileJava",
      className: "JavaCompile",
      outcome: "Success",
      cacheable: true,
      startTimestamp: 1000,
      finishTimestamp: 1120,
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
    expect(queryText).toContain("startTimestamp");
    expect(queryText).toContain("finishTimestamp");
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

  it("hides the dependency graph by default for very large scans", () => {
    const largeFixture = createFixture(1000);

    const pending = controller.expectOne("GetScanTasks");
    pending.flushData({
      buildScan: buildTaskScan([buildTaskEdge()], {
        hasNextPage: false,
        endCursor: null,
      }),
    });
    largeFixture.detectChanges();

    expect(
      largeFixture.nativeElement.querySelector("app-task-dependency-graph"),
    ).toBeNull();
    expect(largeFixture.nativeElement.textContent).toContain(
      "Task dependency graph hidden",
    );

    const buttons = Array.from(
      (largeFixture.nativeElement as HTMLElement).querySelectorAll("button"),
    ) as HTMLButtonElement[];

    const button = buttons.find((candidate) =>
      candidate.textContent?.includes("Render graph anyway"),
    );

    expect(button).toBeTruthy();
    button?.click();
    largeFixture.detectChanges();

    expect(
      largeFixture.nativeElement.querySelector("app-task-dependency-graph"),
    ).toBeTruthy();

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
