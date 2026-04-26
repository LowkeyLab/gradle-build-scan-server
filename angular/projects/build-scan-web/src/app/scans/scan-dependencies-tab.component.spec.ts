import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { TestBed, type ComponentFixture } from "@angular/core/testing";
import {
  ApolloTestingController,
  ApolloTestingModule,
} from "apollo-angular/testing";
import { ScanDependenciesTabComponent } from "./scan-dependencies-tab.component";

function buildConfigurationSummary(overrides: Record<string, unknown> = {}) {
  return {
    id: "6648731359144961106",
    displayName: ":app:build",
    details: [":list:build", ":utilities:build"],
    ...overrides,
  };
}

function buildConfigurationGraph(overrides: Record<string, unknown> = {}) {
  return {
    nodes: [
      { id: "configuration:6648731359144961106", label: ":app:build" },
      {
        id: "dependency:6648731359144961106:0",
        label: "junit-jupiter-api-5.12.1.jar",
      },
    ],
    edges: [
      {
        sourceId: "configuration:6648731359144961106",
        targetId: "dependency:6648731359144961106:0",
      },
    ],
    ...overrides,
  };
}

describe("ScanDependenciesTabComponent", () => {
  let fixture: ComponentFixture<ScanDependenciesTabComponent>;
  let controller: ApolloTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ApolloTestingModule, ScanDependenciesTabComponent],
    });
    controller = TestBed.inject(ApolloTestingController);
    fixture = TestBed.createComponent(ScanDependenciesTabComponent);
    fixture.componentRef.setInput("scanId", "QnVpbGRTY2FuOjEyMw==");
    fixture.detectChanges();
  });

  afterEach(() => {
    fixture.destroy();
    controller.match("GetScanConfigurationDependencies");
    controller.match("GetScanConfigurationDependencyGraph");
    controller.verify();
  });

  it("loads configuration summaries when the tab mounts", () => {
    const pending = controller.expectOne("GetScanConfigurationDependencies");
    expect(pending.operation.variables).toEqual({
      id: "QnVpbGRTY2FuOjEyMw==",
    });
    expect(fixture.nativeElement.textContent).toContain(
      "Loading configurations…",
    );

    pending.flushData({
      buildScan: {
        id: "QnVpbGRTY2FuOjEyMw==",
        configurationDependencies: [buildConfigurationSummary()],
      },
    });
    fixture.detectChanges();

    expect(fixture.nativeElement.textContent).toContain("Configurations");
    expect(fixture.nativeElement.textContent).toContain(":app:build");
    expect(
      controller.match("GetScanConfigurationDependencyGraph"),
    ).toHaveLength(0);
  });

  it("fetches the selected configuration graph only after a click", async () => {
    controller.expectOne("GetScanConfigurationDependencies").flushData({
      buildScan: {
        id: "QnVpbGRTY2FuOjEyMw==",
        configurationDependencies: [buildConfigurationSummary()],
      },
    });
    fixture.detectChanges();

    const button = fixture.nativeElement.querySelector(
      "aside button",
    ) as HTMLButtonElement | null;
    expect(button).toBeTruthy();

    if (!button) {
      throw new Error("Configuration button not found");
    }
    button.click();
    fixture.detectChanges();

    const pendingGraph = controller.expectOne(
      "GetScanConfigurationDependencyGraph",
    );
    expect(pendingGraph.operation.variables).toEqual({
      id: "QnVpbGRTY2FuOjEyMw==",
      configurationId: "6648731359144961106",
    });
    expect(fixture.nativeElement.textContent).toContain(
      "Loading dependency graph for :app:build…",
    );

    pendingGraph.flushData({
      buildScan: {
        id: "QnVpbGRTY2FuOjEyMw==",
        configurationDependencyGraph: buildConfigurationGraph(),
      },
    });
    for (const refetch of controller.match(
      "GetScanConfigurationDependencies",
    )) {
      refetch.flushData({
        buildScan: {
          id: "QnVpbGRTY2FuOjEyMw==",
          configurationDependencies: [buildConfigurationSummary()],
        },
      });
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
    await fixture.whenStable();
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector(
        '[data-testid="configuration-dependency-graph"]',
      ),
    ).toBeTruthy();
    expect(fixture.nativeElement.textContent).toContain(
      "junit-jupiter-api-5.12.1.jar",
    );
  });
});
