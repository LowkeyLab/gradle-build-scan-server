import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
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
      startTimestamp: 1000,
      finishTimestamp: 1120,
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
  edges: any[] = [buildTaskEdge()],
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
    fixture = TestBed.createComponent(ScanTasksTabComponent);
    fixture.componentRef.setInput("scanId", "123");
    fixture.componentRef.setInput("taskCount", 1);
    fixture.detectChanges();
  });

  afterEach(() => {
    controller.verify();
  });

  it("shows a loading state before the first result and then renders the task views", () => {
    const pending = controller.expectOne("GetScanTasks");
    expect(pending.operation.variables).toEqual({
      id: "123",
      firstTasks: 100,
    });
    expect(fixture.nativeElement.textContent).toContain("Loading tasks…");

    pending.flushData({ buildScan: buildTaskScan() });
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector("app-cache-breakdown"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-task-timeline"),
    ).toBeTruthy();
    expect(fixture.nativeElement.querySelector("app-tasks-table")).toBeTruthy();
    expect(fixture.nativeElement.querySelectorAll("tbody tr").length).toBe(1);
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
});
