import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { provideRouter } from "@angular/router";
import { ScanDetailComponent } from "./scan-detail.component";

function buildOverviewScan(overrides: Record<string, unknown> = {}) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
    scanId: "123",
    buildToolType: "Gradle",
    buildToolVersion: "8.0",
    pluginVersion: "3.0",
    outcome: "success",
    createdAt: "2026-01-15T10:00:00Z",
    hostname: "ci-host",
    osName: "Linux",
    osVersion: "6.0",
    jvmVendor: "Eclipse",
    jvmVersion: "21",
    requestedTasks: ["build"],
    taskCount: 1,
    testCount: 1,
    ...overrides,
  };
}

function buildTaskScan(overrides: Record<string, unknown> = {}) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
    tasks: {
      edges: [
        {
          node: {
            id: "VGFzazox",
            dependencies: [],
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
          },
          cursor: "c1",
        },
      ],
      pageInfo: { hasNextPage: false, endCursor: null },
    },
    ...overrides,
  };
}

function buildTestScan(overrides: Record<string, unknown> = {}) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
    testCount: 1,
    testSummary: {
      passed: 1,
      failed: 0,
      skipped: 0,
      totalDurationMs: 1234,
    },
    tests: {
      edges: [
        {
          node: {
            id: "VGVzdDox",
            className: "com.example.FooTest",
            methodName: "testSomething",
            executorName: "Gradle Test Executor 1",
            outcome: "Passed",
            durationMs: 123,
            failureMessage: null,
            failureStacktrace: null,
          },
          cursor: "tc1",
        },
      ],
      pageInfo: { hasNextPage: false, endCursor: null },
    },
    ...overrides,
  };
}

describe("ScanDetailComponent", () => {
  let fixture: ComponentFixture<ScanDetailComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanDetailComponent],
      providers: [provideRouter([])],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanDetailComponent);
    fixture.componentRef.setInput("id", "123");
    fixture.detectChanges();
  });

  afterEach(() => {
    fixture.destroy();
    controller.verify();
  });

  function flushOverview(scanOverrides: Record<string, unknown> = {}) {
    const op = controller.expectOne("GetBuildScanOverview");
    const queryText = op.operation.query.loc?.source.body ?? "";
    expect(queryText).not.toContain("tasks(");
    expect(queryText).not.toContain("tests(");
    expect(queryText).not.toContain("testSummary");
    expect(op.operation.variables).toEqual({ id: "123" });
    op.flushData({ buildScan: buildOverviewScan(scanOverrides) });
    fixture.detectChanges();
    return op;
  }

  function clickTab(label: string) {
    const buttons = Array.from(
      fixture.nativeElement.querySelectorAll("nav button"),
    ) as HTMLButtonElement[];
    const button = buttons.find((candidate) =>
      candidate.textContent?.includes(label),
    );
    expect(button, `button ${label}`).toBeTruthy();
    button!.click();
    fixture.detectChanges();
  }

  it("renders the overview tab without eager task or test fields", () => {
    const op = controller.expectOne("GetBuildScanOverview");
    const queryText = op.operation.query.loc?.source.body ?? "";
    expect(queryText).not.toContain("tasks(");
    expect(queryText).not.toContain("tests(");
    expect(queryText).not.toContain("testSummary");
    expect(op.operation.variables).toEqual({ id: "123" });

    op.flushData({ buildScan: buildOverviewScan() });
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector("app-build-metadata"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-scan-sidebar"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-scan-tasks-tab"),
    ).toBeNull();
    expect(
      fixture.nativeElement.querySelector("app-scan-tests-tab"),
    ).toBeNull();
    expect(fixture.componentInstance.selectedTab()).toBe("overview");
    expect(fixture.componentInstance.isTabMounted("tasks")).toBe(false);
    expect(fixture.componentInstance.isTabMounted("tests")).toBe(false);
  });

  it("loads the Tasks tab on demand and keeps it mounted for revisit within the same scan", () => {
    flushOverview();

    clickTab("Tasks");

    const pendingTasks = controller.expectOne("GetScanTasks");
    expect(pendingTasks.operation.variables).toEqual({
      id: "QnVpbGRTY2FuOjEyMw==",
      firstTasks: 100,
    });
    expect(fixture.nativeElement.textContent).toContain("Loading tasks…");

    pendingTasks.flushData({ buildScan: buildTaskScan() });
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector("app-cache-breakdown"),
    ).toBeTruthy();
    expect(
      fixture.nativeElement.querySelector("app-task-timeline"),
    ).toBeTruthy();
    expect(fixture.nativeElement.querySelector("app-tasks-table")).toBeTruthy();
    expect(fixture.componentInstance.selectedTab()).toBe("tasks");
    expect(fixture.componentInstance.isTabMounted("tasks")).toBe(true);

    clickTab("Overview");
    expect(fixture.componentInstance.selectedTab()).toBe("overview");

    clickTab("Tasks");
    expect(
      controller.match((op) => op.operationName === "GetScanTasks"),
    ).toHaveLength(0);
    expect(fixture.componentInstance.selectedTab()).toBe("tasks");
  });

  it("loads the Tests tab on demand", () => {
    flushOverview();

    clickTab("Tests");

    const pendingTests = controller.expectOne("GetScanTests");
    expect(pendingTests.operation.variables).toEqual({
      id: "QnVpbGRTY2FuOjEyMw==",
      firstTests: 100,
    });
    expect(fixture.nativeElement.textContent).toContain("Loading tests…");

    pendingTests.flushData({ buildScan: buildTestScan() });
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector("app-tests-table")).toBeTruthy();
    expect(fixture.nativeElement.querySelector("h3")?.textContent).toContain(
      "Tests (1)",
    );
    expect(fixture.componentInstance.selectedTab()).toBe("tests");
    expect(fixture.componentInstance.isTabMounted("tests")).toBe(true);

    clickTab("Overview");
    expect(fixture.componentInstance.selectedTab()).toBe("overview");

    clickTab("Tests");
    expect(
      controller.match((op) => op.operationName === "GetScanTests"),
    ).toHaveLength(0);
    expect(fixture.componentInstance.selectedTab()).toBe("tests");
  });

  it("resets tab state when a different scan id arrives", () => {
    flushOverview();

    clickTab("Tasks");
    const firstTasks = controller.expectOne("GetScanTasks");
    firstTasks.flushData({ buildScan: buildTaskScan() });
    fixture.detectChanges();

    fixture.componentRef.setInput("id", "456");
    fixture.detectChanges();

    const secondOverview = controller.expectOne("GetBuildScanOverview");
    expect(secondOverview.operation.variables).toEqual({ id: "456" });
    secondOverview.flushData({
      buildScan: buildOverviewScan({
        id: "QnVpbGRTY2FuOjQ1Ng==",
        scanId: "456",
        taskCount: 0,
        testCount: 0,
      }),
    });
    fixture.detectChanges();

    expect(fixture.componentInstance.selectedTab()).toBe("overview");
    expect(fixture.componentInstance.isTabMounted("tasks")).toBe(false);
    expect(fixture.componentInstance.isTabMounted("tests")).toBe(false);

    clickTab("Tasks");
    expect(
      controller.match((op) => op.operationName === "GetScanTasks"),
    ).toHaveLength(0);
    expect(fixture.nativeElement.textContent).toContain(
      "No tasks recorded for this scan.",
    );
  });
});
