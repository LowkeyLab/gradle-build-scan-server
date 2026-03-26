import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { TasksTableComponent } from "./tasks-table.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: crypto.randomUUID(),
      taskPath: ":app:compileJava",
      outcome: "Success",
      cacheable: true,
      durationMs: 1000,
      className: "JavaCompile",
      cachingDisabledReason: null,
      cachingDisabledExplanation: null,
      upToDateMessages: null,
      originBuildInvocationId: null,
      originExecutionTime: null,
      cacheKey: null,
      ...overrides,
    },
    cursor: "c1",
  };
}

describe("TasksTableComponent", () => {
  let fixture: ComponentFixture<TasksTableComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TasksTableComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(TasksTableComponent);
  });

  function render(edges: any[] = [buildTaskEdge()]) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.componentRef.setInput("taskCount", edges.length);
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  it("should display taskCount in the heading", () => {
    const edges = [buildTaskEdge()];
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.componentRef.setInput("taskCount", 42);
    fixture.detectChanges();
    const heading = fixture.nativeElement.querySelector("h3");
    expect(heading.textContent).toContain("Tasks (42)");
  });

  it("should show Cache Hit badge for FromCache outcome", () => {
    const el = render([buildTaskEdge({ outcome: "FromCache" })]);
    const table = el.querySelector("table");
    const cells = table!.querySelectorAll("tbody td");
    expect(cells[4].textContent!.trim()).toBe("Cache Hit");
    expect(cells[4].querySelector(".badge-info")).toBeTruthy();
  });

  it("should show Up-to-date badge for UpToDate outcome", () => {
    const el = render([buildTaskEdge({ outcome: "UpToDate" })]);
    const table = el.querySelector("table");
    const cells = table!.querySelectorAll("tbody td");
    expect(cells[4].textContent!.trim()).toBe("Up-to-date");
    expect(cells[4].querySelector(".badge-success")).toBeTruthy();
  });

  it("should show caching disabled reason as warning badge with tooltip", () => {
    const el = render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: false,
        cachingDisabledReason: "NOT_ENABLED_FOR_TASK",
        cachingDisabledExplanation: "Not worth caching",
      }),
    ]);
    const table = el.querySelector("table");
    const cells = table!.querySelectorAll("tbody td");
    expect(cells[4].textContent!.trim()).toBe("NOT_ENABLED_FOR_TASK");
    expect(cells[4].querySelector(".badge-warning")).toBeTruthy();
    const tooltip = cells[4].querySelector(".tooltip");
    expect(tooltip!.getAttribute("data-tip")).toBe("Not worth caching");
  });

  it("should show Executed badge for cacheable task without disabled reason", () => {
    const el = render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: true,
        cachingDisabledReason: null,
      }),
    ]);
    const table = el.querySelector("table");
    const cells = table!.querySelectorAll("tbody td");
    expect(cells[4].textContent!.trim()).toBe("Executed");
    expect(cells[4].querySelector(".badge-ghost")).toBeTruthy();
  });

  it("should show dash for non-cacheable task without disabled reason", () => {
    const el = render([
      buildTaskEdge({
        outcome: "Success",
        cacheable: false,
        cachingDisabledReason: null,
      }),
    ]);
    const table = el.querySelector("table");
    const cells = table!.querySelectorAll("tbody td");
    expect(cells[4].textContent!.trim()).toBe("—");
  });

  it("does not show detail row by default", () => {
    const el = render();
    expect(el.querySelector("app-task-cache-detail")).toBeNull();
  });

  it("shows detail row when row is clicked", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.click();
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });

  it("hides detail row when clicked again", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.click();
    fixture.detectChanges();
    row.click();
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).toBeNull();
  });

  it("sets aria-expanded on clicked row", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    expect(row.getAttribute("aria-expanded")).toBe("false");
    row.click();
    fixture.detectChanges();
    expect(row.getAttribute("aria-expanded")).toBe("true");
  });

  it("expands row on Enter keydown", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });

  it("expands row on Space keydown", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.dispatchEvent(
      new KeyboardEvent("keydown", { key: " ", bubbles: true }),
    );
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });
});
