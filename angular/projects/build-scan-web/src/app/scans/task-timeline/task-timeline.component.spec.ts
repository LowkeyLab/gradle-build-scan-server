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
  let component: TaskTimelineComponent;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [TaskTimelineComponent],
    });
    fixture = TestBed.createComponent(TaskTimelineComponent);
    component = fixture.componentInstance;
  });

  function render(edges: any[]) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.detectChanges();
  }

  describe("tasks computed signal", () => {
    it("should filter tasks with timestamps and normalize to relative time", () => {
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

      const tasks = component.tasks();
      expect(tasks.length).toBe(2);
      expect(tasks[0].start).toBe(0);
      expect(tasks[0].end).toBe(120);
      expect(tasks[0].durationMs).toBe(120);
      expect(tasks[1].start).toBe(50);
      expect(tasks[1].end).toBe(80);
    });

    it("should return empty array when no tasks have timestamps", () => {
      render([buildTaskEdge({ startTimestamp: null, finishTimestamp: null })]);
      expect(component.tasks()).toEqual([]);
    });

    it("should sort tasks by start time", () => {
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

      const tasks = component.tasks();
      expect(tasks[0].taskPath).toBe(":early");
      expect(tasks[1].taskPath).toBe(":middle");
      expect(tasks[2].taskPath).toBe(":late");
    });

    it("should exclude tasks without timestamps", () => {
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

      expect(component.tasks().length).toBe(1);
      expect(component.tasks()[0].taskPath).toBe(":compileJava");
    });

    it("should preserve outcome values for color mapping", () => {
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

      const tasks = component.tasks();
      expect(tasks[0].outcome).toBe("Success");
      expect(tasks[1].outcome).toBe("FromCache");
      expect(tasks[2].outcome).toBe("Failed");
    });
  });

  describe("template rendering", () => {
    it("should render card with chart container when tasks have timestamps", () => {
      render([
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
          startTimestamp: 1000,
          finishTimestamp: 1120,
        }),
      ]);
      const card = fixture.nativeElement.querySelector(".card.bg-base-200");
      expect(card).toBeTruthy();
      const heading = card.querySelector("h4");
      expect(heading.textContent.trim()).toBe("Timeline");
    });

    it("should not render when no tasks have timestamps", () => {
      render([buildTaskEdge({ startTimestamp: null, finishTimestamp: null })]);
      const card = fixture.nativeElement.querySelector(".card.bg-base-200");
      expect(card).toBeFalsy();
    });

    it("should render legend entries", () => {
      render([
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
          startTimestamp: 1000,
          finishTimestamp: 1120,
        }),
      ]);
      const legendItems = fixture.nativeElement.querySelectorAll(
        ".text-xs .opacity-70",
      );
      const labels = Array.from(legendItems).map((el: any) =>
        el.textContent.trim(),
      );
      expect(labels).toContain("Success");
      expect(labels).toContain("Cache Hit");
      expect(labels).toContain("Failed");
    });
  });
});
