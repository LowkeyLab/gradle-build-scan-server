import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingModule,
  ApolloTestingController,
} from "apollo-angular/testing";
import { provideRouter } from "@angular/router";
import { ScanDetailComponent } from "./scan-detail.component";

function buildMockScan(overrides: Record<string, unknown> = {}) {
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
    testCount: 0,
    tasks: {
      edges: [
        {
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
          },
          cursor: "c1",
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

  function flushQuery(scanOverrides: Record<string, unknown> = {}) {
    const op = controller.expectOne("GetBuildScan");
    op.flushData({ buildScan: buildMockScan(scanOverrides) });
    fixture.detectChanges();
  }

  it("should render sidebar component", () => {
    flushQuery();
    const sidebar = fixture.nativeElement.querySelector("app-scan-sidebar");
    expect(sidebar).toBeTruthy();
  });

  it("should render task timeline component", () => {
    flushQuery();
    const timeline = fixture.nativeElement.querySelector("app-task-timeline");
    expect(timeline).toBeTruthy();
  });

  it("should render tasks table component", () => {
    flushQuery();
    const table = fixture.nativeElement.querySelector("app-tasks-table");
    expect(table).toBeTruthy();
  });

  it("should render tests table component", () => {
    flushQuery();
    const table = fixture.nativeElement.querySelector("app-tests-table");
    expect(table).toBeTruthy();
  });

  it("should render back link in sidebar", () => {
    flushQuery();
    const link = fixture.nativeElement.querySelector("a.back-link");
    expect(link.textContent).toContain("All Scans");
  });

  it("should render section navigation", () => {
    flushQuery();
    const navItems = fixture.nativeElement.querySelectorAll("nav button");
    expect(navItems.length).toBeGreaterThanOrEqual(3);
  });

  it("should update activeSection on scrollToSection", () => {
    flushQuery();
    // jsdom doesn't implement scrollIntoView; stub it on the target element
    const section = fixture.nativeElement.querySelector("#task-timeline");
    section.scrollIntoView = vi.fn();

    fixture.componentInstance.scrollToSection("task-timeline");

    expect(fixture.componentInstance.activeSection()).toBe("task-timeline");
    expect(section.scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "start",
    });
  });
});
