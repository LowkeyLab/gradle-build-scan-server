import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { ScanTestsTabComponent } from "./scan-tests-tab.component";

function buildTestEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGVzdDox",
      className: "com.example.FooTest",
      methodName: "testSomething",
      executorName: "Gradle Test Executor 1",
      outcome: "Passed",
      durationMs: 123,
      failureMessage: null,
      failureStacktrace: null,
      ...overrides,
    },
    cursor: "tc1",
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
      edges: [buildTestEdge()],
      pageInfo: { hasNextPage: false, endCursor: null },
      ...overrides,
    },
  };
}

describe("ScanTestsTabComponent", () => {
  let fixture: ComponentFixture<ScanTestsTabComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanTestsTabComponent],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanTestsTabComponent);
    fixture.componentRef.setInput("scanId", "123");
    fixture.componentRef.setInput("testCount", 1);
    fixture.detectChanges();
  });

  afterEach(() => {
    controller.verify();
  });

  it("shows a loading state before the first result and then renders the tests table", () => {
    const pending = controller.expectOne("GetScanTests");
    expect(pending.operation.variables).toEqual({
      id: "123",
      firstTests: 100,
    });
    expect(fixture.nativeElement.textContent).toContain("Loading tests…");

    pending.flushData({ buildScan: buildTestScan() });
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector("app-tests-table")).toBeTruthy();
    expect(fixture.nativeElement.querySelector("h3")?.textContent).toContain(
      "Tests (1)",
    );
    expect(fixture.nativeElement.querySelectorAll(".badge-lg").length).toBe(4);
  });

  it("reuses the mounted tab state without issuing a second first-load query when hidden and shown again", () => {
    const pending = controller.expectOne("GetScanTests");
    pending.flushData({ buildScan: buildTestScan() });
    fixture.detectChanges();

    fixture.nativeElement.hidden = true;
    fixture.detectChanges();
    fixture.nativeElement.hidden = false;
    fixture.detectChanges();

    expect(controller.match("GetScanTests")).toHaveLength(0);
    expect(fixture.nativeElement.querySelector("app-tests-table")).toBeTruthy();
  });
});
