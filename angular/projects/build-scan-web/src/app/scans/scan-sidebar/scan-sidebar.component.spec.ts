import { describe, it, expect, beforeEach } from "vitest";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { ScanSidebarComponent } from "./scan-sidebar.component";

function buildScan(overrides: Record<string, unknown> = {}) {
  return {
    id: "QnVpbGRTY2FuOjEyMw==",
    scanId: "123",
    buildToolType: "Gradle",
    buildToolVersion: "8.0",
    pluginVersion: "3.0",
    outcome: "success",
    createdAt: "2026-01-15T10:00:00Z",
    hostname: "ci-host",
    osName: "Linux",
    osVersion: "6.0",
    jvmVendor: "Eclipse",
    jvmVersion: "21",
    requestedTasks: ["build"],
    taskCount: 3,
    testCount: 7,
    ...overrides,
  };
}

describe("ScanSidebarComponent", () => {
  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [ScanSidebarComponent],
      providers: [provideRouter([])],
    });
  });

  function render() {
    const fixture = TestBed.createComponent(ScanSidebarComponent);
    fixture.componentRef.setInput("scan", buildScan());
    fixture.detectChanges();
    return fixture;
  }

  it("renders the Overview, Tasks, Dependencies, and Tests tab controls", () => {
    const fixture = render();
    const buttons = Array.from(
      fixture.nativeElement.querySelectorAll("nav button"),
    ) as HTMLButtonElement[];
    const labels = buttons.map((button) => button.textContent?.trim());

    expect(labels).toEqual(["Overview", "Tasks", "Dependencies", "Tests"]);
    expect(fixture.nativeElement.textContent).not.toContain("Cache Avoidance");
  });

  it("emits tab changes when a tab is clicked", () => {
    const fixture = render();
    let clicked: string | undefined;
    fixture.componentInstance.tabClicked.subscribe((tab) => {
      clicked = tab;
    });

    const tasksButton = Array.from(
      fixture.nativeElement.querySelectorAll("nav button"),
    ).find((button) =>
      (button as HTMLElement).textContent?.includes("Tasks"),
    ) as HTMLElement | undefined;
    expect(tasksButton).toBeTruthy();

    if (!tasksButton) {
      throw new Error("Tasks button not found");
    }
    tasksButton.click();
    expect(clicked).toBe("tasks");
  });

  it("renders the back link to the scans list", () => {
    const fixture = render();
    const link = fixture.nativeElement.querySelector("a.back-link");
    expect(link.textContent).toContain("All Scans");
  });
});
