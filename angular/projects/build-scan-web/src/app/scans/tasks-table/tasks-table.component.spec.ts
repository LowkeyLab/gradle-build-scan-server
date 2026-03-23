import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { TasksTableComponent } from "./tasks-table.component";

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
      ...overrides,
    },
    cursor: "c1",
  };
}

describe("TasksTableComponent", () => {
  let fixture: ComponentFixture<TasksTableComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [TasksTableComponent],
    });
    fixture = TestBed.createComponent(TasksTableComponent);
  });

  function render(
    edges: any[] = [buildTaskEdge()],
    taskCount: number = edges.length,
  ) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.componentRef.setInput("taskCount", taskCount);
    fixture.detectChanges();
  }

  it("should display taskCount in the heading", () => {
    render([buildTaskEdge()], 42);
    const heading = fixture.nativeElement.querySelector("h3");
    expect(heading.textContent).toContain("Tasks (42)");
  });

  it("should show Cache Hit badge for FromCache outcome", () => {
    render([buildTaskEdge({ outcome: "FromCache" })]);
    const table = fixture.nativeElement.querySelector("table");
    const cells = table.querySelectorAll("tbody td");
    expect(cells[4].textContent.trim()).toBe("Cache Hit");
    expect(cells[4].querySelector(".badge-info")).toBeTruthy();
  });

  it("should show Up-to-date badge for UpToDate outcome", () => {
    render([buildTaskEdge({ outcome: "UpToDate" })]);
    const table = fixture.nativeElement.querySelector("table");
    const cells = table.querySelectorAll("tbody td");
    expect(cells[4].textContent.trim()).toBe("Up-to-date");
    expect(cells[4].querySelector(".badge-success")).toBeTruthy();
  });

  it("should show caching disabled reason as warning badge with tooltip", () => {
    render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: false,
        cachingDisabledReason: "NOT_ENABLED_FOR_TASK",
        cachingDisabledExplanation: "Not worth caching",
      }),
    ]);
    const table = fixture.nativeElement.querySelector("table");
    const cells = table.querySelectorAll("tbody td");
    expect(cells[4].textContent.trim()).toBe("NOT_ENABLED_FOR_TASK");
    expect(cells[4].querySelector(".badge-warning")).toBeTruthy();
    const tooltip = cells[4].querySelector(".tooltip");
    expect(tooltip.getAttribute("data-tip")).toBe("Not worth caching");
  });

  it("should show Executed badge for cacheable task without disabled reason", () => {
    render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: true,
        cachingDisabledReason: null,
      }),
    ]);
    const table = fixture.nativeElement.querySelector("table");
    const cells = table.querySelectorAll("tbody td");
    expect(cells[4].textContent.trim()).toBe("Executed");
    expect(cells[4].querySelector(".badge-ghost")).toBeTruthy();
  });

  it("should show dash for non-cacheable task without disabled reason", () => {
    render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: false,
        cachingDisabledReason: null,
      }),
    ]);
    const table = fixture.nativeElement.querySelector("table");
    const cells = table.querySelectorAll("tbody td");
    expect(cells[4].textContent.trim()).toBe("—");
  });
});
