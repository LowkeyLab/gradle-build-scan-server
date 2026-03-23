import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { TestsTableComponent } from "./tests-table.component";

function buildTestEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGVzdDox",
      className: "com.example.FooTest",
      methodName: "testSomething",
      executorName: "Gradle Test Executor 1",
      outcome: "Passed",
      ...overrides,
    },
    cursor: "tc1",
  };
}

describe("TestsTableComponent", () => {
  let fixture: ComponentFixture<TestsTableComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [TestsTableComponent],
    });
    fixture = TestBed.createComponent(TestsTableComponent);
  });

  function render(edges: any[] = [buildTestEdge()], testCount?: number) {
    fixture.componentRef.setInput("testEdges", edges);
    fixture.componentRef.setInput("testCount", testCount ?? edges.length);
    fixture.detectChanges();
  }

  it("should hide when testCount is 0", () => {
    render([], 0);
    const headings = fixture.nativeElement.querySelectorAll("h3");
    expect(headings.length).toBe(0);
  });

  it("should show heading with count when testCount > 0", () => {
    render([buildTestEdge()], 1);
    const heading = fixture.nativeElement.querySelector("h3");
    expect(heading.textContent).toContain("Tests (1)");
  });

  it("should render test table columns", () => {
    render();
    const headers = fixture.nativeElement.querySelectorAll("th");
    expect(Array.from(headers).map((h: any) => h.textContent.trim())).toEqual([
      "Class Name",
      "Method Name",
      "Outcome",
      "Executor",
    ]);
  });

  it("should render test row data", () => {
    render();
    const cells = fixture.nativeElement.querySelectorAll("tbody td");
    expect(cells[0].textContent.trim()).toBe("com.example.FooTest");
    expect(cells[1].textContent.trim()).toBe("testSomething");
    expect(cells[2].textContent).toContain("Passed");
    expect(cells[3].textContent.trim()).toBe("Gradle Test Executor 1");
  });

  it("should apply badge-success for Passed outcome", () => {
    render([buildTestEdge({ outcome: "Passed" })]);
    const badge = fixture.nativeElement.querySelector(".badge");
    expect(badge.classList.contains("badge-success")).toBe(true);
  });

  it("should apply badge-error for Failed outcome", () => {
    render([buildTestEdge({ outcome: "Failed" })]);
    const badge = fixture.nativeElement.querySelector(".badge");
    expect(badge.classList.contains("badge-error")).toBe(true);
  });

  it("should apply badge-warning for Skipped outcome", () => {
    render([buildTestEdge({ outcome: "Skipped" })]);
    const badge = fixture.nativeElement.querySelector(".badge");
    expect(badge.classList.contains("badge-warning")).toBe(true);
  });

  it("should show dash for null methodName", () => {
    render([buildTestEdge({ methodName: null })]);
    const cells = fixture.nativeElement.querySelectorAll("tbody td");
    expect(cells[1].textContent.trim()).toBe("—");
  });

  it("should show dash for null executorName", () => {
    render([buildTestEdge({ executorName: null })]);
    const cells = fixture.nativeElement.querySelectorAll("tbody td");
    expect(cells[3].textContent.trim()).toBe("—");
  });

  it("should render multiple test rows", () => {
    render([
      buildTestEdge(),
      {
        node: {
          id: "VGVzdDoy",
          className: "com.example.BarTest",
          methodName: "testOther",
          executorName: null,
          outcome: "Failed",
        },
        cursor: "tc2",
      },
    ]);
    const rows = fixture.nativeElement.querySelectorAll("tbody tr");
    expect(rows.length).toBe(2);
  });
});
