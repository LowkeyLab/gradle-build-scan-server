import { ComponentFixture, TestBed } from "@angular/core/testing";
import { TaskCacheDetailComponent } from "./task-cache-detail.component";

function buildTaskNode(overrides: Record<string, unknown> = {}) {
  return {
    outcome: "Success",
    cacheable: true,
    cachingDisabledReason: null,
    cachingDisabledExplanation: null,
    upToDateMessages: null,
    originBuildInvocationId: null,
    originExecutionTime: null,
    cacheKey: null,
    durationMs: null,
    ...overrides,
  };
}

describe("TaskCacheDetailComponent", () => {
  let fixture: ComponentFixture<TaskCacheDetailComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TaskCacheDetailComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(TaskCacheDetailComponent);
  });

  function render(taskNode: Record<string, unknown>) {
    fixture.componentRef.setInput("taskNode", taskNode);
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  it("shows up-to-date messages for UpToDate outcome", () => {
    const el = render(
      buildTaskNode({
        outcome: "UpToDate",
        upToDateMessages: ["All output files are up to date"],
      }),
    );
    expect(el.textContent).toContain("All output files are up to date");
  });

  it("shows caching disabled info for non-cacheable tasks", () => {
    const el = render(
      buildTaskNode({
        outcome: "Success",
        cacheable: false,
        cachingDisabledReason: "OVERLAPPING_OUTPUTS",
        cachingDisabledExplanation: "Outputs overlap with :app:integrationTest",
      }),
    );
    expect(el.textContent).toContain("OVERLAPPING_OUTPUTS");
    expect(el.textContent).toContain("Outputs overlap");
  });

  it("shows cache hit info for FromCache outcome", () => {
    const el = render(
      buildTaskNode({
        outcome: "FromCache",
        cacheKey: "a3f8b2c1d4e5",
        originBuildInvocationId: "build-abc123",
        originExecutionTime: 4100,
      }),
    );
    expect(el.textContent).toContain("build-abc123");
    expect(el.textContent).toContain("a3f8b2c1d4e5");
    expect(el.textContent).toContain("4.1s");
  });

  it("shows executed info for cacheable miss", () => {
    const el = render(
      buildTaskNode({
        outcome: "Success",
        cacheable: true,
        durationMs: 2300,
        cacheKey: "a3f8b2c1d4e5",
      }),
    );
    expect(el.textContent).toContain("Executed");
    expect(el.textContent).toContain("2.3s");
  });

  it("shows fallback message when no detail available", () => {
    const el = render(buildTaskNode());
    expect(el.textContent).toContain("No cache detail available");
  });
});
