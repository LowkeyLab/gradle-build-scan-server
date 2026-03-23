import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { TaskTimelineComponent } from "./task-timeline.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGFzazox",
      taskPath: ":compileJava",
      outcome: "Success",
      durationMs: 120,
      startTimestamp: 1000,
      finishTimestamp: 1120,
      ...overrides,
    },
    cursor: "c1",
  };
}

describe("TaskTimelineComponent", () => {
  let fixture: ComponentFixture<TaskTimelineComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [TaskTimelineComponent],
    });
    fixture = TestBed.createComponent(TaskTimelineComponent);
  });

  function render(edges: any[]) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.detectChanges();
  }

  it("should render timeline section when tasks have timestamps", () => {
    render([
      buildTaskEdge({
        id: "T1",
        taskPath: ":compileJava",
        startTimestamp: 1000,
        finishTimestamp: 1120,
      }),
      buildTaskEdge({
        id: "T2",
        taskPath: ":processResources",
        startTimestamp: 1050,
        finishTimestamp: 1080,
      }),
    ]);
    const timeline = fixture.nativeElement.querySelector(".card.bg-base-200");
    expect(timeline).toBeTruthy();
    const heading = timeline.querySelector("h4");
    expect(heading.textContent.trim()).toBe("Timeline");
  });

  it("should not render when no tasks have timestamps", () => {
    render([buildTaskEdge({ startTimestamp: null, finishTimestamp: null })]);
    const timeline = fixture.nativeElement.querySelector(".card.bg-base-200");
    expect(timeline).toBeFalsy();
  });

  it("should color-code timeline bars by outcome", () => {
    render([
      buildTaskEdge({
        id: "T1",
        taskPath: ":a",
        outcome: "Success",
        startTimestamp: 0,
        finishTimestamp: 100,
      }),
      buildTaskEdge({
        id: "T2",
        taskPath: ":b",
        outcome: "FromCache",
        startTimestamp: 50,
        finishTimestamp: 150,
      }),
      buildTaskEdge({
        id: "T3",
        taskPath: ":c",
        outcome: "Failed",
        startTimestamp: 100,
        finishTimestamp: 200,
      }),
    ]);
    const timeline = fixture.nativeElement.querySelector(".card.bg-base-200");
    const bars = timeline.querySelectorAll(".rounded");
    expect(bars[0].classList.contains("bg-success")).toBe(true);
    expect(bars[1].classList.contains("bg-info")).toBe(true);
    expect(bars[2].classList.contains("bg-error")).toBe(true);
  });

  it("should sort timeline tasks by start time", () => {
    render([
      buildTaskEdge({
        id: "T3",
        taskPath: ":late",
        startTimestamp: 200,
        finishTimestamp: 300,
      }),
      buildTaskEdge({
        id: "T1",
        taskPath: ":early",
        startTimestamp: 0,
        finishTimestamp: 100,
      }),
      buildTaskEdge({
        id: "T2",
        taskPath: ":middle",
        startTimestamp: 100,
        finishTimestamp: 200,
      }),
    ]);
    const timeline = fixture.nativeElement.querySelector(".card.bg-base-200");
    const labels = timeline.querySelectorAll("span.font-mono");
    expect(labels[0].textContent.trim()).toBe(":early");
    expect(labels[1].textContent.trim()).toBe(":middle");
    expect(labels[2].textContent.trim()).toBe(":late");
  });

  it("should exclude tasks without timestamps from timeline", () => {
    render([
      buildTaskEdge({
        id: "T1",
        taskPath: ":compileJava",
        startTimestamp: 1000,
        finishTimestamp: 1120,
      }),
      buildTaskEdge({
        id: "T2",
        taskPath: ":noTimestamps",
        startTimestamp: null,
        finishTimestamp: null,
      }),
    ]);
    const timeline = fixture.nativeElement.querySelector(".card.bg-base-200");
    expect(timeline).toBeTruthy();
    const bars = timeline.querySelectorAll(".rounded");
    expect(bars.length).toBe(1);
  });
});
