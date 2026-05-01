import { type ComponentFixture, TestBed } from "@angular/core/testing";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type ElementStateMap = Record<string, string[]>;

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
        { sourceId: "T1", targetId: "T3" },
        { sourceId: "T1", targetId: "T2" },
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

    it("derives recursive upstream highlight state from dependencies", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      component.setHoveredNode("T4");

      const highlightState = component.highlightState();
      expect(highlightState?.activeNodeId).toBe("T4");
      expect(highlightState?.mode).toBe("hover");
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

      component.clearHoveredNode();
      expect(component.highlightState()).toBeNull();
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
        controlPoints: true,
      });
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

    it("maps task graph data into G6 rect nodes and polyline edges", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
          outcome: "FromCache",
        }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      const instance = graphInstance();
      expect(instance.options.autoFit).toBe("view");
      expect(instance.options.node).toMatchObject({ type: "rect" });
      expect(instance.options.edge).toMatchObject({ type: "polyline" });
      expect(instance.options.behaviors).toEqual([
        "drag-canvas",
        "zoom-canvas",
      ]);
      expect(instance.data.nodes).toEqual([
        expect.objectContaining({
          id: "T1",
          data: expect.objectContaining({
            label: ":compileJava",
            displayLabel: ":compileJava",
            outcome: "Success",
          }),
        }),
        expect.objectContaining({
          id: "T2",
          data: expect.objectContaining({
            label: ":processResources",
            displayLabel: ":processResources",
            outcome: "FromCache",
          }),
        }),
        expect.objectContaining({
          id: "T3",
          data: expect.objectContaining({ label: ":test" }),
        }),
      ]);
      expect(instance.data.edges).toEqual([
        expect.objectContaining({ id: "T1:T2", source: "T1", target: "T2" }),
        expect.objectContaining({ id: "T1:T3", source: "T1", target: "T3" }),
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
        controlPoints: true,
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

    it("pushes upstream hover and click highlighting into G6 element states", async () => {
      await render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);
      const instance = graphInstance();

      instance.emitNode("node:pointerenter", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);

      expect(instance.elementStates).toMatchObject({
        T1: ["highlighted"],
        T2: ["highlighted"],
        T3: ["highlighted"],
        T4: ["highlighted"],
        T6: ["dimmed"],
        "T1:T2": ["highlighted"],
        "T2:T3": ["highlighted"],
        "T3:T4": ["highlighted"],
        "T5:T6": ["dimmed"],
      } satisfies ElementStateMap);

      instance.emitNode("node:pointerleave", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(instance.elementStates["T6"]).toEqual([]);

      instance.emitNode("node:click", "T4");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.mode).toBe("selected");
      expect(instance.elementStates["T4"]).toEqual(["highlighted", "selected"]);

      instance.emitNode("node:pointerenter", "T6");
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()?.activeNodeId).toBe("T4");
      expect(instance.elementStates["T6"]).toEqual(["dimmed"]);

      instance.emitCanvasClick();
      fixture.detectChanges();
      await settleAsyncWork(fixture);
      expect(component.highlightState()).toBeNull();
      expect(instance.elementStates["T4"]).toEqual([]);
      expect(instance.elementStates["T6"]).toEqual([]);
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

      const container = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as HTMLElement;
      expect(container).toBeTruthy();
      expect(container.className).toContain("task-dependency-g6");

      const legend = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-legend"]',
      ) as HTMLElement;
      expect(legend).toBeTruthy();
      expect(legend.textContent).toContain("Legend");
      expect(legend.textContent).toContain("Success");
      expect(legend.textContent).toContain("From Cache");
      expect(legend.textContent).toContain("Task dependency");
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
      expect(g6Mock.MockGraph.instances).toHaveLength(0);
    });
  });
});
