import { type ComponentFixture, TestBed } from "@angular/core/testing";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type ElementStateMap = Record<string, string[]>;
type RenderedNodeStyle = Record<string, unknown> & {
  fill?: unknown;
  size?: unknown;
};

const g6Mock = vi.hoisted(() => {
  type HoistedGraphEvent = {
    target: { id: string };
    targetType: "node" | "edge" | "canvas";
  };
  type HoistedGraphEventHandler = (event: HoistedGraphEvent) => void;
  type HoistedGraphData = {
    nodes: Array<{
      id: string;
      data?: Record<string, unknown>;
      style?: Record<string, unknown>;
    }>;
    edges: Array<{
      id: string;
      source: string;
      target: string;
      data?: Record<string, unknown>;
      style?: Record<string, unknown>;
    }>;
  };
  type HoistedGraphOptions = {
    container: HTMLElement;
    data: HoistedGraphData;
    layout?: Record<string, unknown>;
    autoFit?: string;
    node?: Record<string, unknown>;
    edge?: Record<string, unknown>;
    behaviors?: string[];
    plugins?: Array<Record<string, unknown>>;
  };
  type HoistedElementStateMap = Record<string, string[]>;

  class MockGraph {
    static instances: MockGraph[] = [];

    options: HoistedGraphOptions;
    data: HoistedGraphData;
    handlers = new Map<string, HoistedGraphEventHandler>();
    elementStates: HoistedElementStateMap = {};

    render = vi.fn((): Promise<void> => Promise.resolve());
    fitView = vi.fn((_options?: unknown): Promise<void> => Promise.resolve());
    destroy = vi.fn((): void => undefined);
    setData = vi.fn((data: HoistedGraphData): void => {
      this.data = data;
    });
    setLayout = vi.fn((layout: Record<string, unknown>): void => {
      this.options = { ...this.options, layout };
    });
    setElementState = vi.fn((states: HoistedElementStateMap): Promise<void> => {
      this.elementStates = states;
      return Promise.resolve();
    });
    on = vi.fn(
      (eventName: string, handler: HoistedGraphEventHandler): MockGraph => {
        this.handlers.set(eventName, handler);
        return this;
      },
    );

    constructor(options: HoistedGraphOptions) {
      this.options = options;
      this.data = options.data;
      MockGraph.instances.push(this);
    }

    emitNode(eventName: string, nodeId: string): void {
      this.handlers.get(eventName)?.({
        target: { id: nodeId },
        targetType: "node",
      });
    }

    emitCanvasClick(): void {
      this.handlers.get("canvas:click")?.({
        target: { id: "canvas" },
        targetType: "canvas",
      });
    }
  }

  return {
    MockGraph,
    register: vi.fn(),
    ExtensionCategory: { LAYOUT: "layout" },
    NodeEvent: {
      POINTER_ENTER: "node:pointerenter",
      POINTER_LEAVE: "node:pointerleave",
      CLICK: "node:click",
    },
    CanvasEvent: { CLICK: "canvas:click" },
    resetInstances: () => {
      MockGraph.instances = [];
    },
  };
});

const wasmMock = vi.hoisted(() => {
  class AntVDagreLayout {}
  return {
    AntVDagreLayout,
    supportsThreads: vi.fn((): Promise<boolean> => Promise.resolve(true)),
    initThreads: vi.fn(
      (_supported: boolean): Promise<symbol> =>
        Promise.resolve(Symbol.for("layout-wasm-threads")),
    ),
    threadToken: Symbol.for("layout-wasm-threads"),
  };
});

vi.mock("@antv/g6", () => ({
  Graph: g6Mock.MockGraph,
  register: g6Mock.register,
  ExtensionCategory: g6Mock.ExtensionCategory,
  NodeEvent: g6Mock.NodeEvent,
  CanvasEvent: g6Mock.CanvasEvent,
}));

vi.mock("@antv/layout-wasm/dist/index.min.js", () => ({
  AntVDagreLayout: wasmMock.AntVDagreLayout,
  default: {
    AntVDagreLayout: wasmMock.AntVDagreLayout,
    supportsThreads: wasmMock.supportsThreads,
    initThreads: wasmMock.initThreads,
  },
  supportsThreads: wasmMock.supportsThreads,
  initThreads: wasmMock.initThreads,
}));

import { TaskDependencyGraphComponent } from "./task-dependency-graph.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: "VGFzazox",
      dependencies: [],
      taskPath: ":compileJava",
      outcome: "Success",
      durationMs: 120,
      ...overrides,
    },
    cursor: "c1",
  };
}

async function settleAsyncWork(
  fixture: ComponentFixture<unknown>,
): Promise<void> {
  await fixture.whenStable();
  await Promise.resolve();
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  await fixture.whenStable();
  await Promise.resolve();
  await new Promise<void>((resolve) => setTimeout(resolve, 0));
  await fixture.whenStable();
}

describe("TaskDependencyGraphComponent", () => {
  let fixture: ComponentFixture<TaskDependencyGraphComponent>;
  let component: TaskDependencyGraphComponent;

  beforeEach(() => {
    g6Mock.resetInstances();
    wasmMock.supportsThreads.mockClear();
    wasmMock.initThreads.mockClear();
    vi.stubGlobal("Worker", class MockWorker {});

    TestBed.configureTestingModule({
      imports: [TaskDependencyGraphComponent],
    });
    fixture = TestBed.createComponent(TaskDependencyGraphComponent);
    component = fixture.componentInstance;
  });

  afterEach(() => {
    fixture.destroy();
    vi.unstubAllGlobals();
  });

  async function render(edges: Array<ReturnType<typeof buildTaskEdge>>) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.detectChanges();
    await settleAsyncWork(fixture);
    fixture.detectChanges();
    await settleAsyncWork(fixture);
  }

  function graphInstance(): InstanceType<typeof g6Mock.MockGraph> {
    const instance =
      g6Mock.MockGraph.instances[g6Mock.MockGraph.instances.length - 1];
    if (!instance) {
      throw new Error("Expected a G6 graph instance to be created");
    }
    return instance;
  }

  function elementState(
    instance: InstanceType<typeof g6Mock.MockGraph>,
    elementId: string,
  ): string[] | undefined {
    return instance.elementStates[elementId];
  }

  function renderedNode(
    instance: InstanceType<typeof g6Mock.MockGraph>,
    nodeId: string,
  ) {
    const node = instance.data.nodes.find(
      (candidate) => candidate.id === nodeId,
    );
    if (!node) {
      throw new Error(`Expected node ${nodeId} to be rendered`);
    }
    return node;
  }

  function expectOpaqueFill(fill: unknown): void {
    if (typeof fill !== "string") {
      throw new Error("Expected fill to be a CSS color string");
    }
    expect(fill).toMatch(/^oklch\([^/]+\)$/);
  }

  function renderedNodeStyle(
    node: ReturnType<typeof renderedNode>,
  ): RenderedNodeStyle {
    if (!node.style) {
      throw new Error("Expected rendered node to include a style object");
    }
    return node.style;
  }

  function nodeSize(node: ReturnType<typeof renderedNode>): number {
    const size = renderedNodeStyle(node).size;
    if (typeof size !== "number") {
      throw new Error("Expected rendered circle node size to be a number");
    }
    return size;
  }

  function highlightedNodeIds(): string[] {
    return [
      ...(component.highlightState()?.highlightedNodeIds ?? new Set<string>()),
    ].sort();
  }

  function highlightedEdgeKeys(): string[] {
    return [
      ...(component.highlightState()?.highlightedEdgeKeys ?? new Set<string>()),
    ].sort();
  }

  describe("graph data construction", () => {
    it("builds sorted labeled nodes and deduplicated dependency edges", async () => {
      await render([
        buildTaskEdge({
          id: "T3",
          dependencies: ["T1", "T1", "MISSING", "T3"],
          taskPath: ":test",
        }),
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
        }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
          outcome: "FromCache",
        }),
      ]);

      const graph = component.graph();
      expect(graph.nodes.map((node) => node.label)).toEqual([
        ":compileJava",
        ":processResources",
        ":test",
      ]);
      expect(graph.nodes.map((node) => node.displayLabel)).toEqual([
        ":compileJava",
        ":processResources",
        ":test",
      ]);
      expect(graph.edges).toEqual([
        { sourceId: "T1", targetId: "T2" },
        { sourceId: "T1", targetId: "T3" },
      ]);
    });

    it("builds a deterministic edge order independent of task input order", async () => {
      await render([
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T4", dependencies: ["T1"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
      ]);

      expect(component.graph().edges).toEqual([
        { sourceId: "T1", targetId: "T2" },
        { sourceId: "T1", targetId: "T4" },
        { sourceId: "T2", targetId: "T3" },
      ]);
    });

    it("keeps isolated nodes while filtering missing dependencies and self edges", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["MISSING", "T2"],
          taskPath: ":processResources",
        }),
      ]);

      const graph = component.graph();
      expect(graph.nodes).toEqual([
        expect.objectContaining({ id: "T1", label: ":compileJava" }),
        expect.objectContaining({ id: "T2", label: ":processResources" }),
      ]);
      expect(graph.edges).toEqual([]);
    });

    it("derives recursive upstream highlight state from selected dependencies", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      component.selectNode("T4");

      const highlightState = component.highlightState();
      expect(highlightState?.activeNodeId).toBe("T4");
      expect(highlightState?.mode).toBe("selected");
      expect(
        [...(highlightState?.highlightedNodeIds ?? new Set<string>())].sort(),
      ).toEqual(["T1", "T2", "T3", "T4"]);
      expect(
        [...(highlightState?.highlightedEdgeKeys ?? new Set<string>())].sort(),
      ).toEqual(["T1:T2", "T2:T3", "T3:T4"]);
      expect(component.nodeHighlightState("T6")).toBe("dimmed");
      expect(
        component.edgeHighlightState({ sourceId: "T5", targetId: "T6" }),
      ).toBe("dimmed");

      component.selectNode("T4");
      expect(component.highlightState()).toBeNull();
    });
  });

  describe("critical path state", () => {
    it("derives terminal task options from sink nodes only", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build" }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":compileJava",
        }),
        buildTaskEdge({ id: "C", dependencies: ["B"], taskPath: ":test" }),
      ]);

      expect(component.graph().nodes.map((node) => node.id)).toEqual([
        "A",
        "B",
        "C",
      ]);
      expect(component.graph().edges).toEqual([
        { sourceId: "A", targetId: "B" },
        { sourceId: "B", targetId: "C" },
      ]);
      expect(component.terminalOptions().map((option) => option.label)).toEqual(
        [":test"],
      );
    });

    it("treats the selected terminal target as the critical-path anchor and keeps invalid typed values unhighlighted", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":compileJava", durationMs: 10 }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":test",
          durationMs: 20,
        }),
      ]);

      component.selectCriticalPathTarget(":test");
      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.targetInputValue()).toBe(":test");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");
      expect(component.criticalPathWarning()).toBeNull();
      expect(component.highlightState()?.mode).toBe("critical");
      expect(highlightedNodeIds()).toEqual(["A", "B"]);
      expect(highlightedEdgeKeys()).toEqual(["A:B"]);

      component.selectCriticalPathTarget(":missing");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.targetInputValue()).toBe(":missing");
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();
      expect(component.highlightState()).toBeNull();
      expect(component.criticalPathWarning()).toContain("terminal task");
    });

    it("treats duplicate terminal labels as ambiguous target values", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 10 }),
        buildTaskEdge({ id: "B", taskPath: ":build", durationMs: 20 }),
      ]);

      expect(component.terminalOptions().map((option) => option.label)).toEqual([
        ":build",
        ":build",
      ]);

      component.selectCriticalPathTarget(":build");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.targetInputValue()).toBe(":build");
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();
      expect(component.criticalPathTargetInputInvalid()).toBe(true);
      expect(component.criticalPathTargetHelpText()).toContain(
        "exactly one terminal task",
      );

      component.showCriticalPath.set(true);

      expect(component.highlightState()).toBeNull();
      expect(component.criticalPathWarning()).toContain(
        "exactly one terminal task",
      );
    });

    it("handles zero, multiple, missing, non-terminal, and terminal requested-task preselection cases", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build" }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":compileJava",
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", []);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();

      fixture.componentRef.setInput("requestedTasks", [":build", ":test"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();

      fixture.componentRef.setInput("requestedTasks", [":missing"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();

      fixture.componentRef.setInput("requestedTasks", [":build"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();
      expect(component.targetInputValue()).toBe("");

      fixture.componentRef.setInput("requestedTasks", [":compileJava"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.terminalOptions().map((option) => option.label)).toEqual(
        [":compileJava"],
      );
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");
      expect(component.targetInputValue()).toBe(":compileJava");
    });

    it("preselects a terminal requested task and keeps a user-modified target across requestedTasks changes", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build" }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":compileJava",
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":compileJava"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");

      component.selectCriticalPathTarget(":compileJava");
      expect(component.targetInputValue()).toBe(":compileJava");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");

      fixture.componentRef.setInput("requestedTasks", [":missing"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");
    });

    it("reconciles stale targets on graph changes without overriding user-modified targets", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "B", dependencies: ["A"], taskPath: ":test" }),
      ]);

      component.selectCriticalPathTarget(":test");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");

      await render([
        buildTaskEdge({ id: "A", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "C", dependencies: ["A"], taskPath: ":check" }),
      ]);

      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();

      component.selectCriticalPathTarget(":check");
      await render([
        buildTaskEdge({ id: "A", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "C", dependencies: ["A"], taskPath: ":check" }),
        buildTaskEdge({ id: "D", dependencies: ["C"], taskPath: ":assemble" }),
      ]);

      expect(component.targetInputValue()).toBe(":check");
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();
    });

    it("computes a critical path through dependency edge direction", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":a", durationMs: 80 }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":b",
          durationMs: 20,
        }),
        buildTaskEdge({
          id: "C",
          dependencies: ["B"],
          taskPath: ":c",
          durationMs: 30,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":c"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(component.highlightState()?.mode).toBe("critical");
      expect(highlightedNodeIds()).toEqual(["A", "B", "C"]);
      expect(highlightedEdgeKeys()).toEqual(["A:B", "B:C"]);
      expect(component.criticalPathHasCycle()).toBe(false);
    });

    it("highlights only the requested terminal path in disconnected graphs", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":a", durationMs: 80 }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":b",
          durationMs: 1,
        }),
        buildTaskEdge({ id: "C", taskPath: ":c", durationMs: 80 }),
        buildTaskEdge({
          id: "D",
          dependencies: ["C"],
          taskPath: ":d",
          durationMs: 10,
        }),
        buildTaskEdge({
          id: "E",
          dependencies: ["D"],
          taskPath: ":e",
          durationMs: 10,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":b"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(highlightedNodeIds()).toEqual(["A", "B"]);
      expect(highlightedEdgeKeys()).toEqual(["A:B"]);
    });

    it("breaks equal-score ties by the lexicographically smallest terminal path", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":alpha", durationMs: 4 }),
        buildTaskEdge({ id: "B", taskPath: ":beta", durationMs: 4 }),
        buildTaskEdge({ id: "C", taskPath: ":gamma", durationMs: 8 }),
        buildTaskEdge({
          id: "D",
          dependencies: ["A"],
          taskPath: ":delta",
          durationMs: 6,
        }),
        buildTaskEdge({
          id: "E",
          dependencies: ["B"],
          taskPath: ":epsilon",
          durationMs: 6,
        }),
        buildTaskEdge({
          id: "F",
          dependencies: ["C", "D", "E"],
          taskPath: ":zeta",
          durationMs: 2,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":zeta"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(highlightedNodeIds()).toEqual(["A", "D", "F"]);
      expect(highlightedEdgeKeys()).toEqual(["A:D", "D:F"]);
    });

    it("treats null durations as zero in terminal critical-path scoring", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":alpha", durationMs: null }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":beta",
          durationMs: 5,
        }),
        buildTaskEdge({ id: "C", taskPath: ":gamma", durationMs: 4 }),
        buildTaskEdge({
          id: "D",
          dependencies: ["C"],
          taskPath: ":delta",
          durationMs: 3,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":beta"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(highlightedNodeIds()).toEqual(["A", "B"]);
      expect(highlightedEdgeKeys()).toEqual(["A:B"]);
    });

    it("resolves a single exact taskPath match without normalization", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":app:build", durationMs: 10 }),
        buildTaskEdge({ id: "B", taskPath: ":build", durationMs: 9 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":app:build"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(highlightedNodeIds()).toEqual(["A"]);
      expect(highlightedEdgeKeys()).toEqual([]);
    });

    it("normalizes a root task request to a single matching node", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 11 }),
        buildTaskEdge({ id: "B", taskPath: ":test", durationMs: 2 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", ["build"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);

      expect(highlightedNodeIds()).toEqual(["A"]);
      expect(highlightedEdgeKeys()).toEqual([]);
    });

    it("returns no critical highlight when multiple tasks are requested", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 11 }),
        buildTaskEdge({ id: "B", taskPath: ":test", durationMs: 2 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", ["build", "test"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.criticalPathWarning()).toBeNull();
      component.showCriticalPath.set(true);

      expect(component.highlightState()).toBeNull();
      expect(component.criticalPathHasCycle()).toBe(false);
      expect(component.criticalPathWarning()).toContain(
        "choose a terminal task from the selector",
      );
    });

    it("returns no critical highlight when terminal resolution is missing or ambiguous", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 11 }),
        buildTaskEdge({ id: "B", taskPath: ":build", durationMs: 2 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", ["test"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.criticalPathWarning()).toBeNull();
      component.showCriticalPath.set(true);

      expect(component.highlightState()).toBeNull();
      expect(highlightedNodeIds()).toEqual([]);
      expect(highlightedEdgeKeys()).toEqual([]);
      expect(component.criticalPathWarning()).toContain(
        "choose a terminal task from the selector",
      );

      fixture.componentRef.setInput("requestedTasks", ["build"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.criticalPathWarning()).toContain(
        "choose a terminal task from the selector",
      );
    });

    it("renders the cycle warning and hides terminal text when both are possible", async () => {
      await render([
        buildTaskEdge({
          id: "A",
          dependencies: ["B"],
          taskPath: ":build",
          durationMs: 10,
        }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":build",
          durationMs: 20,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", ["build"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      const warning = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-warning"]',
      ) as HTMLElement;
      expect(warning).toBeTruthy();
      expect(warning.textContent).toContain("dependency cycle");
      expect(warning.textContent).not.toContain("requested task");
    });
  });

  describe("G6 rendering", () => {
    it("renders with an antv-dagre layered layout", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      expect(g6Mock.register).not.toHaveBeenCalled();
      expect(wasmMock.supportsThreads).not.toHaveBeenCalled();
      expect(wasmMock.initThreads).not.toHaveBeenCalled();
      expect(graphInstance().options.layout).toEqual({
        type: "antv-dagre",
        rankdir: "LR",
        align: "DL",
        nodesep: 56,
        ranksep: 96,
      });
      expect(graphInstance().fitView).toHaveBeenCalledTimes(1);
    });

    it("lets the layout position nodes instead of passing explicit coordinates", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", taskPath: ":processResources" }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      for (const node of graphInstance().data.nodes) {
        expect(node.style).not.toHaveProperty("x");
        expect(node.style).not.toHaveProperty("y");
      }
      expect(graphInstance().data.edges).toEqual([
        expect.objectContaining({ id: "T1:T3", source: "T1", target: "T3" }),
      ]);
    });

    it("maps task graph data into opaque G6 circle nodes and cubic horizontal edges", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
          outcome: "FromCache",
        }),
        buildTaskEdge({ id: "T3", taskPath: ":test" }),
        buildTaskEdge({
          id: "T4",
          dependencies: ["T1", "T2", "T3"],
          taskPath: ":verify",
          outcome: "Failed",
        }),
      ]);

      const instance = graphInstance();
      expect(instance.options.autoFit).toBe("view");
      expect(instance.options.node).toMatchObject({ type: "circle" });
      expect(instance.options.edge).toMatchObject({
        type: "cubic-horizontal",
        style: {
          endArrow: false,
        },
      });
      expect(instance.options.node).toMatchObject({
        state: {
          highlighted: expect.objectContaining({ lineWidth: 3 }),
          selected: expect.objectContaining({ lineWidth: 4 }),
          critical: expect.objectContaining({
            lineWidth: 5,
            stroke: expect.any(String),
            shadowBlur: 18,
          }),
          dimmed: expect.objectContaining({ opacity: 0.28 }),
        },
      });
      expect(instance.options.edge).toMatchObject({
        state: {
          highlighted: expect.objectContaining({ lineWidth: 3 }),
          critical: expect.objectContaining({
            lineWidth: 5,
            stroke: expect.any(String),
            opacity: 1,
          }),
          dimmed: expect.objectContaining({ opacity: 0.12 }),
        },
      });
      expect(instance.options.behaviors).toEqual([
        "drag-canvas",
        "zoom-canvas",
      ]);
      expect(instance.options.plugins).toEqual([
        {
          type: "minimap",
          key: "task-graph-minimap",
          size: [240, 160],
          position: "right-bottom",
        },
      ]);
      expect(instance.data.nodes).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            id: "T1",
            data: expect.objectContaining({
              label: ":compileJava",
              displayLabel: ":compileJava",
              outcome: "Success",
              incomingDependencyCount: 0,
            }),
          }),
          expect.objectContaining({
            id: "T2",
            data: expect.objectContaining({
              label: ":processResources",
              displayLabel: ":processResources",
              outcome: "FromCache",
              incomingDependencyCount: 1,
            }),
          }),
          expect.objectContaining({
            id: "T3",
            data: expect.objectContaining({
              label: ":test",
              incomingDependencyCount: 0,
            }),
          }),
          expect.objectContaining({
            id: "T4",
            data: expect.objectContaining({
              label: ":verify",
              incomingDependencyCount: 3,
            }),
          }),
        ]),
      );
      expect(instance.data.nodes).toHaveLength(4);
      for (const node of instance.data.nodes) {
        const style = renderedNodeStyle(node);
        expect(style).not.toHaveProperty("radius");
        expectOpaqueFill(style.fill);
        expect(style).toHaveProperty("labelFontSize", 16);
      }

      const sourceNode = renderedNode(instance, "T1");
      const oneIncomingNode = renderedNode(instance, "T2");
      const threeIncomingNode = renderedNode(instance, "T4");
      expect(nodeSize(sourceNode)).toBe(72);
      expect(nodeSize(oneIncomingNode)).toBeGreaterThan(nodeSize(sourceNode));
      expect(nodeSize(threeIncomingNode)).toBeGreaterThan(
        nodeSize(oneIncomingNode),
      );
      expect(instance.data.edges).toEqual([
        expect.objectContaining({ id: "T1:T2", source: "T1", target: "T2" }),
        expect.objectContaining({ id: "T1:T4", source: "T1", target: "T4" }),
        expect.objectContaining({ id: "T2:T4", source: "T2", target: "T4" }),
        expect.objectContaining({ id: "T3:T4", source: "T3", target: "T4" }),
      ]);
    });

    it("updates existing G6 data and refits the viewport when inputs change", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":test" }),
      ]);
      const instance = graphInstance();

      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":test" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":check" }),
      ]);

      expect(g6Mock.MockGraph.instances).toHaveLength(1);
      expect(instance.setData).toHaveBeenCalledWith(
        expect.objectContaining({
          nodes: expect.arrayContaining([
            expect.objectContaining({ id: "T3" }),
          ]),
          edges: expect.arrayContaining([
            expect.objectContaining({
              id: "T2:T3",
              source: "T2",
              target: "T3",
            }),
          ]),
        }),
      );
      expect(instance.setLayout).toHaveBeenCalledWith({
        type: "antv-dagre",
        rankdir: "LR",
        align: "DL",
        nodesep: 56,
        ranksep: 96,
      });
    });

    it("destroys the G6 graph when the component is destroyed", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":test" }),
      ]);
      const instance = graphInstance();

      fixture.destroy();

      expect(instance.destroy).toHaveBeenCalledTimes(1);
    });

    it("pushes click-only upstream highlighting into G6 element states", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);
      const instance = graphInstance();

      expect(instance.handlers.has("node:pointerenter")).toBe(false);
      expect(instance.handlers.has("node:pointerleave")).toBe(false);

      instance.emitNode("node:pointerenter", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(instance.elementStates).toEqual({});

      instance.emitNode("node:pointerleave", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(instance.elementStates).toEqual({});

      instance.emitNode("node:click", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("selected");
      expect(instance.elementStates).toMatchObject({
        T1: ["highlighted"],
        T2: ["highlighted"],
        T3: ["highlighted"],
        T4: ["highlighted", "selected"],
        T6: ["dimmed"],
        "T1:T2": ["highlighted"],
        "T2:T3": ["highlighted"],
        "T3:T4": ["highlighted"],
        "T5:T6": ["dimmed"],
      } satisfies ElementStateMap);

      instance.emitNode("node:pointerenter", "T6");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.activeNodeId).toBe("T4");
      expect(elementState(instance, "T6")).toEqual(["dimmed"]);

      instance.emitNode("node:click", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(elementState(instance, "T4")).toEqual([]);
      expect(elementState(instance, "T6")).toEqual([]);

      instance.emitNode("node:click", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.activeNodeId).toBe("T4");

      instance.emitCanvasClick();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(elementState(instance, "T4")).toEqual([]);
      expect(elementState(instance, "T6")).toEqual([]);
    });

    it("pushes critical path states into G6 and restores selected states when disabled", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha", durationMs: 10 }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":beta",
          durationMs: 20,
        }),
        buildTaskEdge({ id: "T3", taskPath: ":gamma", durationMs: 1 }),
        buildTaskEdge({
          id: "T4",
          dependencies: ["T2"],
          taskPath: ":delta",
          durationMs: 30,
        }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon", durationMs: 2 }),
      ]);
      const instance = graphInstance();

      fixture.componentRef.setInput("requestedTasks", [":delta"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");
      expect(elementState(instance, "T4")).toEqual(["critical"]);
      expect(elementState(instance, "T5")).toEqual(["dimmed"]);

      component.toggleCriticalPath();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual([]);

      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");

      instance.emitNode("node:click", "T5");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual(["dimmed"]);

      instance.emitCanvasClick();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual(["dimmed"]);

      component.toggleCriticalPath();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual([]);

      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");

      instance.emitNode("node:click", "T5");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual(["dimmed"]);

      component.toggleCriticalPath();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(instance.elementStates).toMatchObject({
        T1: [],
        T2: [],
        T4: [],
        T3: [],
        T5: [],
        "T1:T2": [],
        "T2:T4": [],
      } satisfies ElementStateMap);
      expect(elementState(instance, "T5")).not.toContain("selected");

      component.toggleCriticalPath();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("critical");
      expect(component.targetInputValue()).toBe(":delta");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("T4");
      expect(elementState(instance, "T5")).toEqual(["dimmed"]);
      expect(elementState(instance, "T1")).toEqual(["critical"]);
      expect(elementState(instance, "T1:T2")).toEqual(["critical"]);
    });
  });

  describe("template rendering", () => {
    it("keeps the graph card, counts, legend, and G6 container", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      const card = fixture.nativeElement.querySelector(
        ".card.bg-base-200",
      ) as HTMLElement;
      expect(card).toBeTruthy();
      expect(card.querySelector("h4")?.textContent?.trim()).toBe(
        "Task Dependencies",
      );
      expect(card.textContent).toContain("3 nodes · 2 edges");
      expect(card.textContent).not.toContain("Timeline");
      expect(card.querySelector("svg")).toBeNull();

      const toggle = card.querySelector(
        '[data-testid="task-critical-path-toggle"]',
      ) as HTMLButtonElement;
      expect(toggle).toBeTruthy();
      expect(toggle.getAttribute("aria-pressed")).toBe("false");
      expect(toggle.textContent?.trim()).toBe("Highlight critical path");
      expect(
        card.querySelector('[data-testid="task-critical-path-warning"]'),
      ).toBeFalsy();

      toggle.click();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();
      expect(toggle.getAttribute("aria-pressed")).toBe("true");
      expect(toggle.textContent?.trim()).toBe("Critical path highlighted");

      const container = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as HTMLElement;
      expect(container).toBeTruthy();
      expect(container.className).toContain("task-dependency-g6");
      expect(container.className).toContain("h-[36rem]");
      expect(container.className).toContain("min-h-[32rem]");

      const legend = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-legend"]',
      ) as HTMLElement;
      expect(legend).toBeTruthy();
      expect(legend.textContent).toContain("Legend");
      expect(legend.textContent).toContain("Success");
      expect(legend.textContent).toContain("From Cache");
      expect(legend.textContent).toContain("Task dependency");
      for (const item of component.legendNodeItems) {
        expectOpaqueFill(item.fillColor);
      }
    });

    it("renders a labeled native terminal target input wired to datalist options", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build" }),
        buildTaskEdge({ id: "B", dependencies: ["A"], taskPath: ":test" }),
        buildTaskEdge({ id: "C", dependencies: ["A"], taskPath: ":check" }),
      ]);

      const label = fixture.nativeElement.querySelector(
        'label[for="task-critical-path-target-input"]',
      ) as HTMLLabelElement;
      const input = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-input"]',
      ) as HTMLInputElement;
      const datalist = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-options"]',
      ) as HTMLDataListElement;
      const help = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-help"]',
      ) as HTMLElement;

      expect(label).toBeTruthy();
      expect(label.textContent?.trim()).toBe("Critical path target");
      expect(label.htmlFor).toBe(input.id);
      expect(input.getAttribute("list")).toBe(datalist.id);
      expect(input.value).toBe(component.targetInputValue());
      expect(input.disabled).toBe(false);
      expect(help.textContent).toContain("Choose a terminal task");
      expect(
        Array.from(datalist.querySelectorAll("option")).map(
          (option) => option.value,
        ),
      ).toEqual([":check", ":test"]);

      input.value = ":test";
      input.dispatchEvent(new Event("input"));
      fixture.detectChanges();

      expect(component.targetInputValue()).toBe(":test");
      expect(component.effectiveCriticalPathTargetNodeId()).toBe("B");
      expect(component.showCriticalPath()).toBe(false);
    });

    it("disables the terminal target input when no terminal tasks are available", async () => {
      await render([
        buildTaskEdge({ id: "A", dependencies: ["B"], taskPath: ":alpha" }),
        buildTaskEdge({ id: "B", dependencies: ["A"], taskPath: ":beta" }),
      ]);

      const input = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-input"]',
      ) as HTMLInputElement;
      const datalist = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-options"]',
      ) as HTMLDataListElement;
      const help = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-help"]',
      ) as HTMLElement;

      expect(component.terminalOptions()).toEqual([]);
      expect(input.disabled).toBe(true);
      expect(input.getAttribute("list")).toBe(datalist.id);
      expect(datalist.querySelectorAll("option")).toHaveLength(0);
      expect(help.textContent).toContain("No terminal tasks are available");
    });

    it("shows invalid target helper text and avoids highlighting invalid values", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 10 }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":test",
          durationMs: 20,
        }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":test"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      const input = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-input"]',
      ) as HTMLInputElement;
      input.value = ":missing";
      input.dispatchEvent(new Event("change"));
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      const help = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-target-help"]',
      ) as HTMLElement;
      expect(help.textContent).toContain(
        "Value must match exactly one terminal task",
      );
      expect(component.effectiveCriticalPathTargetNodeId()).toBeNull();

      component.toggleCriticalPath();
      fixture.detectChanges();
      await settleAsyncWork(fixture);

      expect(component.highlightState()).toBeNull();
    });

    it("renders an empty-state message without creating a G6 graph", async () => {
      await render([]);

      const card = fixture.nativeElement.querySelector(
        ".card.bg-base-200",
      ) as HTMLElement;
      expect(card).toBeTruthy();
      expect(card.textContent).toContain("No task dependency graph available.");
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-dependency-graph"]',
        ),
      ).toBeFalsy();
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-dependency-legend"]',
        ),
      ).toBeFalsy();
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-critical-path-toggle"]',
        ),
      ).toBeFalsy();
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-critical-path-warning"]',
        ),
      ).toBeFalsy();
      expect(g6Mock.MockGraph.instances).toHaveLength(0);
    });

    it("renders a terminal warning only after critical path mode is enabled", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 10 }),
        buildTaskEdge({ id: "B", taskPath: ":test", durationMs: 20 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":missing"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-critical-path-warning"]',
        ),
      ).toBeFalsy();

      const toggle = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-toggle"]',
      ) as HTMLButtonElement;
      toggle.click();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      const warning = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-warning"]',
      ) as HTMLElement;
      expect(warning).toBeTruthy();
      expect(warning.textContent).toContain(
        "choose a terminal task from the selector",
      );
      expect(warning.textContent).not.toContain("dependency cycle");
    });

    it("keeps cycle warnings ahead of target warnings", async () => {
      await render([
        buildTaskEdge({
          id: "A",
          dependencies: ["B"],
          taskPath: ":build",
          durationMs: 10,
        }),
        buildTaskEdge({
          id: "B",
          dependencies: ["A"],
          taskPath: ":test",
          durationMs: 20,
        }),
      ]);

      component.selectCriticalPathTarget(":missing");
      component.showCriticalPath.set(true);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(component.criticalPathHasCycle()).toBe(true);
      expect(component.highlightState()).toBeNull();
      expect(component.criticalPathWarning()).toContain("dependency cycle");
      expect(component.criticalPathWarning()).not.toContain("terminal task");
    });

    it("renders a multiple-request warning after critical path mode is enabled", async () => {
      await render([
        buildTaskEdge({ id: "A", taskPath: ":build", durationMs: 10 }),
        buildTaskEdge({ id: "B", taskPath: ":test", durationMs: 20 }),
      ]);

      fixture.componentRef.setInput("requestedTasks", [":build", ":test"]);
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-critical-path-warning"]',
        ),
      ).toBeFalsy();

      const toggle = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-toggle"]',
      ) as HTMLButtonElement;
      toggle.click();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      fixture.detectChanges();

      const warning = fixture.nativeElement.querySelector(
        '[data-testid="task-critical-path-warning"]',
      ) as HTMLElement;
      expect(warning).toBeTruthy();
      expect(warning.textContent).toContain(
        "choose a terminal task from the selector",
      );
    });
  });
});
