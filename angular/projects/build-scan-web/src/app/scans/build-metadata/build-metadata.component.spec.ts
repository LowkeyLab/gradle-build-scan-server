import { describe, it, expect, beforeEach } from "vitest";
import { TestBed, ComponentFixture } from "@angular/core/testing";
import { BuildMetadataComponent } from "./build-metadata.component";

function buildMockScan(overrides: Record<string, unknown> = {}) {
  return {
    outcome: "success",
    createdAt: "2026-01-15T10:00:00Z",
    buildToolType: "Gradle",
    buildToolVersion: "8.0",
    pluginVersion: "3.0",
    hostname: "ci-host",
    osName: "Linux",
    osVersion: "6.0",
    jvmVendor: "Eclipse",
    jvmVersion: "21",
    requestedTasks: ["build"],
    ...overrides,
  };
}

describe("BuildMetadataComponent", () => {
  let fixture: ComponentFixture<BuildMetadataComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [BuildMetadataComponent],
    });
    fixture = TestBed.createComponent(BuildMetadataComponent);
  });

  function render(scanOverrides: Record<string, unknown> = {}) {
    fixture.componentRef.setInput("scan", buildMockScan(scanOverrides));
    fixture.detectChanges();
  }

  it("should display outcome badge with success styling", () => {
    render({ outcome: "success" });
    const badge = fixture.nativeElement.querySelector(".badge");
    expect(badge.textContent.trim()).toBe("success");
    expect(badge.classList.contains("badge-success")).toBe(true);
  });

  it("should display outcome badge with error styling for failed", () => {
    render({ outcome: "failed" });
    const badge = fixture.nativeElement.querySelector(".badge");
    expect(badge.textContent.trim()).toBe("failed");
    expect(badge.classList.contains("badge-error")).toBe(true);
  });

  it("should display build tool info", () => {
    render();
    const text = fixture.nativeElement.textContent;
    expect(text).toContain("Gradle 8.0");
  });

  it("should show dash for null hostname", () => {
    render({ hostname: null });
    const text = fixture.nativeElement.textContent;
    expect(text).toContain("—");
  });

  it("should hide OS when osName is null", () => {
    render({ osName: null });
    const text = fixture.nativeElement.textContent;
    expect(text).not.toContain("Linux");
  });

  it("should hide JVM when jvmVersion is null", () => {
    render({ jvmVersion: null });
    const text = fixture.nativeElement.textContent;
    expect(text).not.toContain("Eclipse");
  });

  it("should show requested tasks", () => {
    render({ requestedTasks: ["clean", "build"] });
    const text = fixture.nativeElement.textContent;
    expect(text).toContain("clean build");
  });

  it("should hide requested tasks when empty", () => {
    render({ requestedTasks: [] });
    const text = fixture.nativeElement.textContent;
    expect(text).not.toContain("Requested Tasks");
  });
});
