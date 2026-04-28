import { type ComponentFixture, TestBed } from "@angular/core/testing";
import { select, zoomIdentity, type ZoomBehavior } from "d3";
import { beforeEach, describe, expect, it } from "vitest";
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

describe("TaskDependencyGraphComponent", () => {
  let fixture: ComponentFixture<TaskDependencyGraphComponent>;
  let component: TaskDependencyGraphComponent;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [TaskDependencyGraphComponent],
    });
    fixture = TestBed.createComponent(TaskDependencyGraphComponent);
    component = fixture.componentInstance;
  });

  function render(edges: Array<ReturnType<typeof buildTaskEdge>>) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.detectChanges();
  }

  it("renders custom title and description copy", () => {
    fixture.componentRef.setInput("taskEdges", [buildTaskEdge()]);
    fixture.componentRef.setInput("title", "Critical Path");
    fixture.componentRef.setInput(
      "description",
      "Showing the longest weighted dependency chain.",
    );
    fixture.detectChanges();

    expect(fixture.nativeElement.textContent).toContain("Critical Path");
    expect(fixture.nativeElement.textContent).toContain(
      "Showing the longest weighted dependency chain.",
    );
  });

  describe("graph layout", () => {
    it("builds labeled nodes and directed edges from per-task dependencies", () => {
      render([
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
        }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({
          id: "T3",
          dependencies: ["T1"],
          taskPath: ":test",
        }),
      ]);

      const graph = component.graph();
      expect(graph.nodes.map((node) => node.label)).toEqual([
        ":compileJava",
        ":processResources",
        ":test",
      ]);
      expect(graph.edges).toEqual([
        expect.objectContaining({ sourceId: "T1", targetId: "T2" }),
        expect.objectContaining({ sourceId: "T1", targetId: "T3" }),
      ]);
    });

    it("ignores graph entries that do not map to loaded task labels", () => {
      render([
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
        }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["MISSING"],
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

    it("drops cyclic dependency edges before rendering the graph layout", () => {
      render([
        buildTaskEdge({
          id: "T1",
          dependencies: ["T2"],
          taskPath: ":compileJava",
        }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
      ]);

      const graph = component.graph();
      expect(graph.nodes.map((node) => node.id)).toEqual(["T1", "T2"]);
      expect(graph.edges).toEqual([]);
      expect(component.layout().nodes.map((node) => node.id)).toEqual([
        "T1",
        "T2",
      ]);
    });

    it("orders nodes within a Sugiyama layer to reduce dependency crossings", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":yank" }),
        buildTaskEdge({ id: "T4", dependencies: ["T1"], taskPath: ":zeta" }),
      ]);

      const secondLayerLabels = component
        .layout()
        .nodes.filter((node) => node.layer === 1)
        .sort((left, right) => left.column - right.column)
        .map((node) => node.label);

      expect(secondLayerLabels).toEqual([":yank", ":zeta"]);
    });

    it("renders routed d3-dag link points for sibling edges in a top-to-bottom flow", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      const edges = component.layout().edges;
      expect(edges.every((edge) => edge.points.length >= 2)).toBe(true);
      expect(
        new Set(
          edges.map((edge) => edge.points[edge.points.length - 1]?.x ?? -1),
        ).size,
      ).toBe(2);
      expect(
        edges.every(
          (edge) =>
            (edge.points[0]?.y ?? 0) <
            (edge.points[edge.points.length - 1]?.y ?? 0),
        ),
      ).toBe(true);
      expect(edges.every((edge) => edge.path.startsWith("M "))).toBe(true);
      expect(edges.every((edge) => edge.strokeWidth > 2)).toBe(true);
      expect(
        edges.every((edge) =>
          /L [^ ]+ [^ ]+ L [^ ]+ [^ ]+ L [^ ]+ [^ ]+/.test(edge.path),
        ),
      ).toBe(true);
      const layout = component.layout();
      const sourceNode = layout.nodes.find((node) => node.id === "T1");
      expect(sourceNode).toBeTruthy();
      if (!sourceNode) {
        throw new Error("source node T1 not found");
      }
      expect(
        edges.every(
          (edge) =>
            edge.points[0]?.x === sourceNode.x + sourceNode.width / 2 &&
            edge.points[0]?.y === sourceNode.y + sourceNode.height,
        ),
      ).toBe(true);
      expect(
        edges.every((edge) => {
          const targetNode = layout.nodes.find(
            (node) => node.id === edge.targetId,
          );
          if (!targetNode) {
            throw new Error(`target node ${edge.targetId} not found`);
          }
          return (
            edge.points[edge.points.length - 1]?.x ===
              targetNode.x + targetNode.width / 2 &&
            edge.points[edge.points.length - 1]?.y === targetNode.y
          );
        }),
      ).toBe(true);
    });

    it("keeps isolated nodes by seeding graphConnect with single-node placeholders", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({ id: "T2", taskPath: ":processResources" }),
      ]);

      const layout = component.layout();
      expect(layout.nodes.map((node) => node.id)).toEqual(["T1", "T2"]);
      expect(layout.edges).toEqual([]);
      expect(layout.width).toBeGreaterThan(0);
      expect(layout.height).toBeGreaterThan(0);
    });

    it("derives recursive upstream highlight state for hovered nodes", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      component.setHoveredNode("T4");

      const highlightState = component.highlightState();
      expect(highlightState).toBeTruthy();
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

    it("keeps click selection active until another node or blank space is clicked", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      component.selectNode("T4");
      component.setHoveredNode("T6");

      let highlightState = component.highlightState();
      expect(highlightState?.activeNodeId).toBe("T4");
      expect(highlightState?.mode).toBe("selected");
      expect(
        [...(highlightState?.highlightedEdgeKeys ?? new Set<string>())].sort(),
      ).toEqual(["T1:T2", "T2:T3", "T3:T4"]);

      component.selectNode("T6");
      highlightState = component.highlightState();
      expect(highlightState?.activeNodeId).toBe("T6");
      expect(highlightState?.mode).toBe("selected");
      expect(
        [...(highlightState?.highlightedEdgeKeys ?? new Set<string>())].sort(),
      ).toEqual(["T5:T6"]);

      component.clearSelectedNode();
      expect(component.highlightState()).toBeNull();
    });
  });

  describe("template rendering", () => {
    it("renders a dependency graph card instead of the old timeline", () => {
      render([
        buildTaskEdge({
          id: "T1",
          taskPath: ":compileJava",
        }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({
          id: "T3",
          dependencies: ["T1"],
          taskPath: ":test",
        }),
      ]);

      const card = fixture.nativeElement.querySelector(".card.bg-base-200");
      expect(card).toBeTruthy();
      const heading = card.querySelector("h4");
      expect(heading.textContent.trim()).toBe("Task Dependencies");
      expect(card.textContent).not.toContain("Timeline");
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-dependency-graph"]',
        ),
      ).toBeTruthy();
      expect(
        fixture.nativeElement.querySelector(
          '[data-testid="task-dependency-viewport"]',
        ),
      ).toBeTruthy();
      expect(
        fixture.nativeElement.querySelectorAll(
          '[data-testid="dependency-node"]',
        ).length,
      ).toBe(3);
      expect(
        fixture.nativeElement.querySelectorAll(
          '[data-testid="dependency-edge"]',
        ).length,
      ).toBe(2);
      const legend = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-legend"]',
      );
      expect(legend).toBeTruthy();
      expect(legend.textContent).toContain("Legend");
      expect(legend.textContent).toContain("Success");
      expect(legend.textContent).toContain("From Cache");
      expect(legend.textContent).toContain("Task dependency");
      expect(legend.textContent).not.toContain("Cross-layer dependency");
      expect(legend.textContent).not.toContain("Direct dependency");
      expect(card.textContent).toContain(":compileJava");
      expect(card.textContent).toContain(":processResources");
      expect(card.textContent).toContain(":test");
    });

    it("renders an empty-state message when the dependency graph payload is empty", () => {
      render([]);

      const card = fixture.nativeElement.querySelector(".card.bg-base-200");
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
    });

    it("renders long-span edges with explicit routing attributes", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({
          id: "T3",
          dependencies: ["T1", "T2"],
          taskPath: ":test",
        }),
      ]);

      const longSpanEdge = fixture.nativeElement.querySelector(
        '[data-testid="dependency-edge"][data-edge-span="2"]',
      );

      expect(longSpanEdge).toBeTruthy();
      expect(longSpanEdge.getAttribute("stroke-dasharray")).toBeNull();
      expect(
        Number(longSpanEdge.getAttribute("data-point-count")),
      ).toBeGreaterThan(2);
      expect(longSpanEdge.getAttribute("marker-end")).toBeNull();
      expect(
        fixture.nativeElement.querySelector("#task-dependency-arrow"),
      ).toBeNull();
    });

    it("resets the viewport zoom when the graph layout changes", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
      ]);

      const svg = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as SVGSVGElement;
      const viewport = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-viewport"]',
      ) as SVGGElement;
      const zoomBehavior = (
        component as unknown as {
          zoomBehavior: ZoomBehavior<SVGSVGElement, unknown>;
        }
      ).zoomBehavior;

      select(svg).call(
        zoomBehavior.transform,
        zoomIdentity.translate(24, 36).scale(1.75),
      );

      expect(viewport.getAttribute("transform")).toBe(
        "translate(24,36) scale(1.75)",
      );

      render([
        buildTaskEdge({ id: "T1", taskPath: ":compileJava" }),
        buildTaskEdge({
          id: "T2",
          dependencies: ["T1"],
          taskPath: ":processResources",
        }),
        buildTaskEdge({ id: "T3", dependencies: ["T1"], taskPath: ":test" }),
        buildTaskEdge({ id: "T4", dependencies: ["T2"], taskPath: ":check" }),
      ]);

      const rerenderedSvg = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as SVGSVGElement & {
        __zoom?: { k: number; x: number; y: number };
      };
      const rerenderedViewport = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-viewport"]',
      ) as SVGGElement;

      expect(rerenderedViewport.getAttribute("transform")).toBe(
        "translate(0,0) scale(1)",
      );
      expect(rerenderedSvg.__zoom).toMatchObject({ k: 1, x: 0, y: 0 });
    });

    it("adds hover styling hooks that highlight upstream chains and dim unrelated graph elements", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      const hoveredNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T4"]',
      ) as SVGGElement;
      expect(hoveredNode?.getAttribute("data-highlight-state")).toBe("idle");

      hoveredNode.dispatchEvent(new MouseEvent("mouseenter"));
      fixture.detectChanges();

      const highlightedNodeIds = [
        ...fixture.nativeElement.querySelectorAll(
          '[data-testid="dependency-node"][data-highlight-state="highlighted"]',
        ),
      ]
        .map((node) => node.getAttribute("data-node-id"))
        .sort();
      expect(highlightedNodeIds).toEqual(["T1", "T2", "T3", "T4"]);

      const highlightedEdgeIds = [
        ...fixture.nativeElement.querySelectorAll(
          '[data-testid="dependency-edge"][data-highlight-state="highlighted"]',
        ),
      ]
        .map((edge) => edge.getAttribute("data-edge-id"))
        .sort();
      expect(highlightedEdgeIds).toEqual(["T1:T2", "T2:T3", "T3:T4"]);

      const dimmedNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T6"]',
      ) as SVGGElement;
      const dimmedEdge = fixture.nativeElement.querySelector(
        '[data-testid="dependency-edge"][data-edge-id="T5:T6"]',
      ) as SVGPathElement;
      expect(dimmedNode.getAttribute("data-highlight-state")).toBe("dimmed");
      expect(dimmedNode.getAttribute("opacity")).toBe("0.28");
      expect(dimmedEdge.getAttribute("data-highlight-state")).toBe("dimmed");
      expect(dimmedEdge.getAttribute("stroke-opacity")).toBe("0.12");

      hoveredNode.dispatchEvent(new MouseEvent("mouseleave"));
      fixture.detectChanges();

      expect(hoveredNode.getAttribute("data-highlight-state")).toBe("idle");
      expect(dimmedNode.getAttribute("data-highlight-state")).toBe("idle");
      expect(dimmedEdge.getAttribute("data-highlight-state")).toBe("idle");
    });

    it("persists click-based highlighting and suppresses hover until the selection is cleared", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
        buildTaskEdge({ id: "T5", taskPath: ":epsilon" }),
        buildTaskEdge({ id: "T6", dependencies: ["T5"], taskPath: ":zeta" }),
      ]);

      const graph = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as SVGSVGElement;
      const selectedNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T4"]',
      ) as SVGGElement;
      const otherNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T6"]',
      ) as SVGGElement;

      selectedNode.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(selectedNode.getAttribute("data-highlight-state")).toBe(
        "highlighted",
      );
      expect(otherNode.getAttribute("data-highlight-state")).toBe("dimmed");

      otherNode.dispatchEvent(new MouseEvent("mouseenter", { bubbles: true }));
      fixture.detectChanges();

      expect(selectedNode.getAttribute("data-highlight-state")).toBe(
        "highlighted",
      );
      expect(otherNode.getAttribute("data-highlight-state")).toBe("dimmed");

      otherNode.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();
      expect(otherNode.getAttribute("data-highlight-state")).toBe(
        "highlighted",
      );

      graph.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();
      expect(selectedNode.getAttribute("data-highlight-state")).toBe("idle");
      expect(otherNode.getAttribute("data-highlight-state")).toBe("idle");
    });

    it("clears a persisted selection when the selected node disappears after a graph rerender", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
      ]);

      const initiallySelectedNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T3"]',
      ) as SVGGElement;

      initiallySelectedNode.dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
      );
      fixture.detectChanges();

      expect(component.highlightState()?.activeNodeId).toBe("T3");

      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
      ]);
      fixture.detectChanges();

      expect(
        (
          component as unknown as { selectedNodeId: () => string | null }
        ).selectedNodeId(),
      ).toBeNull();
      expect(component.highlightState()).toBeNull();

      const remainingNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T2"]',
      ) as SVGGElement;

      remainingNode.dispatchEvent(
        new MouseEvent("mouseenter", { bubbles: true }),
      );
      fixture.detectChanges();

      expect(component.highlightState()?.activeNodeId).toBe("T2");
      expect(component.highlightState()?.mode).toBe("hover");
    });

    it("preserves click selection on edge clicks and clears it only on blank svg space", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
        buildTaskEdge({ id: "T4", dependencies: ["T3"], taskPath: ":delta" }),
      ]);

      const graph = fixture.nativeElement.querySelector(
        '[data-testid="task-dependency-graph"]',
      ) as SVGSVGElement;
      const selectedNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T4"]',
      ) as SVGGElement;
      const edge = fixture.nativeElement.querySelector(
        '[data-testid="dependency-edge"][data-edge-id="T3:T4"]',
      ) as SVGPathElement;

      selectedNode.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(component.highlightState()?.activeNodeId).toBe("T4");
      expect(component.highlightState()?.mode).toBe("selected");

      edge.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(component.highlightState()?.activeNodeId).toBe("T4");
      expect(component.highlightState()?.mode).toBe("selected");
      expect(selectedNode.getAttribute("data-highlight-state")).toBe(
        "highlighted",
      );

      graph.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(component.highlightState()).toBeNull();
      expect(selectedNode.getAttribute("data-highlight-state")).toBe("idle");
    });

    it("clears the persisted highlight when the selected node is clicked again", () => {
      render([
        buildTaskEdge({ id: "T1", taskPath: ":alpha" }),
        buildTaskEdge({ id: "T2", dependencies: ["T1"], taskPath: ":beta" }),
        buildTaskEdge({ id: "T3", dependencies: ["T2"], taskPath: ":gamma" }),
      ]);

      const selectedNode = fixture.nativeElement.querySelector(
        '[data-testid="dependency-node"][data-node-id="T3"]',
      ) as SVGGElement;

      selectedNode.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(component.highlightState()?.activeNodeId).toBe("T3");
      expect(component.highlightState()?.mode).toBe("selected");
      expect(selectedNode.getAttribute("data-highlight-state")).toBe(
        "highlighted",
      );

      selectedNode.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      fixture.detectChanges();

      expect(component.highlightState()).toBeNull();
      expect(selectedNode.getAttribute("data-highlight-state")).toBe("idle");
    });
  });
});
