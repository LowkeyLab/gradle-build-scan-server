import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { CacheBreakdownComponent } from "./cache-breakdown.component";

function buildTaskEdge(outcome: string) {
  return {
    node: {
      id: `task-${Math.random()}`,
      taskPath: `:task-${outcome}`,
      outcome,
      cacheable: true,
    },
    cursor: "c1",
  };
}

describe("CacheBreakdownComponent", () => {
  let fixture: ComponentFixture<CacheBreakdownComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [CacheBreakdownComponent],
    });
    fixture = TestBed.createComponent(CacheBreakdownComponent);
  });

  function render(edges: any[], taskCount?: number) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.componentRef.setInput("taskCount", taskCount ?? edges.length);
    fixture.detectChanges();
  }

  it("should show empty message when no tasks", () => {
    render([]);
    expect(fixture.nativeElement.textContent).toContain(
      "No task data available",
    );
  });

  it("should render chart container when tasks exist", () => {
    render([buildTaskEdge("Success"), buildTaskEdge("FromCache")]);
    const container = fixture.nativeElement.querySelector("div");
    expect(container).toBeTruthy();
  });

  it("should display avoided percentage", () => {
    render([
      buildTaskEdge("FromCache"),
      buildTaskEdge("FromCache"),
      buildTaskEdge("Success"),
      buildTaskEdge("Success"),
    ]);
    expect(fixture.nativeElement.textContent).toContain("50%");
    expect(fixture.nativeElement.textContent).toContain("avoided");
  });

  it("should count UpToDate as avoided work", () => {
    render([
      buildTaskEdge("UpToDate"),
      buildTaskEdge("UpToDate"),
      buildTaskEdge("UpToDate"),
      buildTaskEdge("Success"),
    ]);
    expect(fixture.nativeElement.textContent).toContain("75%");
  });

  it("should show legend entries for present categories only", () => {
    render([buildTaskEdge("FromCache"), buildTaskEdge("UpToDate")]);
    const legendItems = fixture.nativeElement.querySelectorAll(
      ".flex.flex-col .flex.items-center",
    );
    expect(legendItems.length).toBe(2);
    const labels = Array.from(legendItems).map((el: any) =>
      el.textContent.trim(),
    );
    expect(labels.some((l: string) => l.includes("Cache Hit"))).toBe(true);
    expect(labels.some((l: string) => l.includes("Up-to-date"))).toBe(true);
  });

  it("should map FromCache to Cache Hit category", () => {
    render([buildTaskEdge("FromCache")]);
    expect(fixture.nativeElement.textContent).toContain("Cache Hit");
    expect(fixture.nativeElement.textContent).not.toContain("FromCache");
  });

  it("should map UpToDate to Up-to-date category", () => {
    render([buildTaskEdge("UpToDate")]);
    expect(fixture.nativeElement.textContent).toContain("Up-to-date");
    expect(fixture.nativeElement.textContent).not.toContain("UpToDate");
  });

  it("should map NoSource to No Source category and count as avoided", () => {
    render([buildTaskEdge("NoSource"), buildTaskEdge("Success")]);
    expect(fixture.nativeElement.textContent).toContain("No Source");
    expect(fixture.nativeElement.textContent).toContain("50%");
  });

  it("should map AvoidedForUnknownReason to Avoided category", () => {
    render([
      buildTaskEdge("AvoidedForUnknownReason"),
      buildTaskEdge("Success"),
    ]);
    expect(fixture.nativeElement.textContent).toContain("Avoided");
    expect(fixture.nativeElement.textContent).toContain("50%");
  });

  it("should show all seven outcome types when present", () => {
    render([
      buildTaskEdge("FromCache"),
      buildTaskEdge("UpToDate"),
      buildTaskEdge("Success"),
      buildTaskEdge("Failed"),
      buildTaskEdge("Skipped"),
      buildTaskEdge("NoSource"),
      buildTaskEdge("AvoidedForUnknownReason"),
    ]);
    const text = fixture.nativeElement.textContent;
    expect(text).toContain("Cache Hit");
    expect(text).toContain("Up-to-date");
    expect(text).toContain("Executed");
    expect(text).toContain("Failed");
    expect(text).toContain("Skipped");
    expect(text).toContain("No Source");
    expect(text).toContain("Avoided");
  });

  it("should show partial data indicator when taskCount exceeds edges", () => {
    render([buildTaskEdge("Success"), buildTaskEdge("FromCache")], 200);
    const text = fixture.nativeElement.textContent;
    expect(text).toContain("first 2 of 200 tasks");
  });

  it("should not show partial indicator when all tasks are loaded", () => {
    const edges = [buildTaskEdge("Success"), buildTaskEdge("FromCache")];
    render(edges, edges.length);
    const text = fixture.nativeElement.textContent;
    expect(text).not.toContain("first");
    expect(text).not.toContain("of");
  });
});
