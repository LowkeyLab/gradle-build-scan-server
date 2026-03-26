# Cache Invalidation Reason Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface cache miss/invalidation reasons for build scan tasks across the full stack (parser -> DB -> GraphQL -> Angular frontend).

**Architecture:** Two-phase approach. Phase 1 surfaces two already-decoded-but-discarded fields (`up_to_date_messages`, `origin_build_invocation_id`) through the existing 6-layer pipeline. Phase 2 reverse-engineers `BUILD_CACHE_*` binary events for per-task cache operation lifecycle. Frontend adds expandable task rows with compact cache detail and extends the existing cache breakdown component.

**Tech Stack:** Rust (Bazel build), SQLite (sqlx), Juniper GraphQL, Angular 21 (standalone components, signals, Tailwind/DaisyUI)

**Spec:** `docs/specs/2026-03-26-cache-invalidation-reason-design.md`

**Build/Test commands:**
- Build all: `aspect build //...`
- Test all: `aspect test //...`
- Format: `bazel run //tools/format`
- Regenerate BUILD files: `bazel run gazelle`
- Dogfood: `cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean build`

**Important:** After editing `.rs` files or `BUILD` files, always run `bazel run gazelle` before building. Gazelle may strip internal workspace deps — use `# keep` comments on ambiguous deps and verify with a build.

**Dependencies:** Chunks 2 and 3 (frontend) depend on Chunk 1 (backend) being complete. The `originExecutionTime` field is already wired in the backend but not yet queried by the frontend. The `upToDateMessages` and `originBuildInvocationId` fields require Chunk 1 to be complete before they carry data.

---

## File Structure

### Phase 1: Backend (files to modify)

| File | Change |
|------|--------|
| `build-scan/lib/src/events/task_finished.rs` | Collect `up_to_date_messages` into Vec instead of discarding; add field to `TaskFinishedEvent` |
| `build-scan/lib/src/models.rs` | Add `up_to_date_messages` and `origin_build_invocation_id` to `Task` |
| `build-scan/lib/src/assembly.rs` | Add fields to `FinishedInfo` and both mapping sites; update test fixture |
| `build-scan/server/domain/src/lib.rs` | Add fields to domain `Task` |
| `build-scan/server/db/migrations/src/sql/005_task_cache_detail.sql` | New migration: add columns |
| `build-scan/server/db/src/lib.rs` | Add to `TaskRow`, insert SQL, select queries, both conversions |
| `build-scan/server/graphql/src/lib.rs` | Add resolver methods |
| `build-scan/server/service/src/lib.rs` | Add builder calls for new fields |

### Phase 1: Frontend (files to create/modify)

| File | Change |
|------|--------|
| `angular/projects/build-scan-web/src/app/scans/scan-detail.component.ts` | Add new GraphQL fields to query; add `tasksLoading` signal |
| `angular/projects/build-scan-web/src/app/scans/task-cache-detail/task-cache-detail.component.ts` | **Create:** compact text summary of task cache behavior |
| `angular/projects/build-scan-web/src/app/scans/task-cache-detail/task-cache-detail.component.spec.ts` | **Create:** tests for all 4 scenarios |
| `angular/projects/build-scan-web/src/app/scans/tasks-table/tasks-table.component.ts` | Add expandable rows |
| `angular/projects/build-scan-web/src/app/scans/cache-breakdown/cache-breakdown.component.ts` | Add time-saved and miss-pattern sections |
| `angular/projects/build-scan-web/src/app/scans/cache-breakdown/cache-breakdown.component.spec.ts` | Add tests for new sections |
| `angular/projects/build-scan-web/src/app/scans/scan-sidebar/scan-sidebar.component.ts` | Rename sidebar item |

---

## Chunk 1: Backend — Surface Phase 1 Fields

### Task 1: Collect `up_to_date_messages` in the decoder

The decoder at `build-scan/lib/src/events/task_finished.rs:81-87` currently reads and discards up-to-date messages in a loop. We need to collect them. Also add the field to `TaskFinishedEvent`.

**Files:**
- Modify: `build-scan/lib/src/events/task_finished.rs:81-87` (discard loop), `:95-106` (struct)

- [ ] **Step 1: Modify the discard loop to collect messages**

Replace the discard loop (lines 81-87) with collection. Uses the project's `kryo::is_field_present` helper (bit 0 = present) and `(body, &mut pos)` cursor pattern:

```rust
// bit 11: upToDateMessages
let up_to_date_messages = if kryo::is_field_present(flags, 11) {
    let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
    let mut messages = Vec::with_capacity(count);
    for _ in 0..count {
        messages.push(table.read_string(body, &mut pos)?);
    }
    Some(messages)
} else {
    None
};
```

- [ ] **Step 2: Add field to `TaskFinishedEvent` struct**

Add after `skip_reason_message` (line 105):

```rust
pub up_to_date_messages: Option<Vec<String>>,
```

And in the struct initialization, add:

```rust
up_to_date_messages,
```

- [ ] **Step 3: Run `bazel run gazelle` and build**

```bash
bazel run gazelle && aspect build //build-scan/lib/...
```

Expected: Builds successfully (field is added but not yet consumed).

- [ ] **Step 4: Add unit test for the new field**

The decoder tests live in the `#[cfg(test)]` module at the bottom of `task_finished.rs`. Add a test that constructs a binary payload with bit 11 present and verifies `up_to_date_messages` is populated. The exact binary fixture depends on the wire format — look at existing tests in this file for the pattern. At minimum, verify that when bit 11 is absent (the common case in existing fixtures), `up_to_date_messages` is `None`:

```rust
#[test]
fn up_to_date_messages_is_none_when_absent() {
    // Use an existing test fixture that doesn't set bit 11
    // Assert: result.up_to_date_messages.is_none()
}
```

If an existing test fixture can be extended to include bit 11 data, add a positive test too:

```rust
#[test]
fn up_to_date_messages_collected_when_present() {
    // Construct or extend a fixture with bit 11 set
    // Assert: result.up_to_date_messages == Some(vec!["All output files are up to date".to_string()])
}
```

- [ ] **Step 5: Run tests**

```bash
aspect test //build-scan/lib/...
```

- [ ] **Step 6: Commit**

```bash
git add build-scan/lib/src/events/task_finished.rs
git commit -m "feat: collect up_to_date_messages in TaskFinished decoder"
```

### Task 2: Add fields to lib `Task` model and wire through assembly

Tasks 2 and 3 from the original plan are merged to avoid a broken-state commit. `origin_build_invocation_id` is already present on `TaskFinishedEvent` (line 102) but dropped during assembly. `up_to_date_messages` is newly added in Task 1.

**Files:**
- Modify: `build-scan/lib/src/models.rs:84-110`
- Modify: `build-scan/lib/src/assembly.rs:452-460` (FinishedInfo), `:53-65` (event->FinishedInfo), `:288-304` (FinishedInfo->Task), `:594+` (test fixture)

- [ ] **Step 1: Add fields to `Task` struct in `models.rs`**

Add after `inputs` field (line 109):

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub up_to_date_messages: Option<Vec<String>>,
#[serde(skip_serializing_if = "Option::is_none")]
pub origin_build_invocation_id: Option<String>,
```

- [ ] **Step 2: Add fields to `FinishedInfo` struct in `assembly.rs`**

Add after `timestamp` (line 459):

```rust
up_to_date_messages: Option<Vec<String>>,
origin_build_invocation_id: Option<String>,
```

- [ ] **Step 3: Map from `TaskFinishedEvent` to `FinishedInfo`**

Add after `timestamp: frame.timestamp,` (line 63):

```rust
up_to_date_messages: e.up_to_date_messages.clone(),
origin_build_invocation_id: e.origin_build_invocation_id.clone(),
```

- [ ] **Step 4: Map from `FinishedInfo` to `Task`**

Add after `inputs,` (line 303):

```rust
up_to_date_messages: fin.and_then(|f| f.up_to_date_messages.clone()),
origin_build_invocation_id: fin.and_then(|f| f.origin_build_invocation_id.clone()),
```

- [ ] **Step 5: Update test fixture in `assembly.rs`**

The existing `test_assemble_single_task` test (around line 594) constructs a `TaskFinishedEvent` directly. Add the new field to the test literal:

```rust
up_to_date_messages: None,
```

(`origin_build_invocation_id` is already in the struct, so only `up_to_date_messages` needs adding.)

- [ ] **Step 6: Add assembly test for new fields**

The existing `test_assemble_single_task` test verifies field propagation. Add assertions for the new fields. Either extend the existing test or add a new one:

```rust
#[test]
fn assemble_task_with_up_to_date_messages() {
    // Construct a TaskFinishedEvent with up_to_date_messages and origin_build_invocation_id set
    // Run through assemble()
    // Assert: resulting Task has both fields populated
}
```

- [ ] **Step 7: Build and test**

```bash
bazel run gazelle && aspect build //build-scan/lib/... && aspect test //build-scan/lib/...
```

Expected: All pass. The lib layer now carries both fields end-to-end.

- [ ] **Step 8: Commit**

```bash
git add build-scan/lib/src/models.rs build-scan/lib/src/assembly.rs
git commit -m "feat: wire up_to_date_messages and origin_build_invocation_id through lib layer"
```

### Task 3: Add fields to domain `Task`

**Files:**
- Modify: `build-scan/server/domain/src/lib.rs:310-334`

- [ ] **Step 1: Add fields to domain `Task`**

Add after `caching_disabled_explanation` (line 333):

```rust
#[builder(setter(strip_option), default)]
pub up_to_date_messages: Option<Vec<String>>,
#[builder(setter(strip_option), default)]
pub origin_build_invocation_id: Option<String>,
```

- [ ] **Step 2: Build**

```bash
bazel run gazelle && aspect build //build-scan/server/domain/...
```

Expected: Builds. Builder derives new methods automatically.

- [ ] **Step 3: Commit**

```bash
git add build-scan/server/domain/src/lib.rs
git commit -m "feat: add cache detail fields to domain Task"
```

### Task 4: DB migration

**Files:**
- Create: `build-scan/server/db/migrations/src/sql/005_task_cache_detail.sql`

- [ ] **Step 1: Create migration file**

```sql
-- up_to_date_messages: JSON array of strings, NULL when absent (never empty array)
-- origin_build_invocation_id: build invocation that produced the cached result
ALTER TABLE tasks ADD COLUMN up_to_date_messages TEXT;
ALTER TABLE tasks ADD COLUMN origin_build_invocation_id TEXT;
```

- [ ] **Step 2: Build migrations crate**

```bash
bazel run gazelle && aspect build //build-scan/server/db/migrations/...
```

- [ ] **Step 3: Commit**

```bash
git add build-scan/server/db/migrations/src/sql/005_task_cache_detail.sql
git commit -m "feat: add migration for task cache detail columns"
```

### Task 5: Wire fields through DB layer

6 insertion points in `db/src/lib.rs`. `serde_json` is already a dependency of the db crate (used for `requested_tasks` parsing).

**Files:**
- Modify: `build-scan/server/db/src/lib.rs` at lines ~40-53 (TaskRow), ~121-153 (TaskRow->domain), ~185-201 (domain->TaskRow), ~275-302 (insert SQL), ~358-369 (list queries), ~401 (get query)

- [ ] **Step 1: Add fields to `TaskRow` struct**

Add after `caching_disabled_explanation` (line 52):

```rust
up_to_date_messages: Option<String>,
origin_build_invocation_id: Option<String>,
```

Note: `up_to_date_messages` is `Option<String>` in the DB layer (JSON text), not `Vec<String>`.

- [ ] **Step 2: Update `TaskRow -> domain::Task` conversion**

Add after `caching_disabled_explanation` mapping (line 150). Note the turbofish on `from_str` for type inference:

```rust
up_to_date_messages: row
    .up_to_date_messages
    .as_deref()
    .map(|s| serde_json::from_str::<Vec<String>>(s))
    .transpose()
    .map_err(|e| anyhow::anyhow!("failed to parse up_to_date_messages: {e}"))?,
origin_build_invocation_id: row.origin_build_invocation_id,
```

- [ ] **Step 3: Update `domain::Task -> TaskRow` conversion**

Add after `caching_disabled_explanation` mapping (line 199):

```rust
up_to_date_messages: task
    .up_to_date_messages
    .as_ref()
    .map(|msgs| serde_json::to_string(msgs).expect("failed to serialize up_to_date_messages")),
origin_build_invocation_id: task.origin_build_invocation_id.clone(),
```

- [ ] **Step 4: Update insert SQL**

Add `up_to_date_messages, origin_build_invocation_id` to the column list and two more `?` placeholders. Add two `.bind()` calls:

```rust
.bind(row.up_to_date_messages.as_deref())
.bind(row.origin_build_invocation_id.as_deref())
```

- [ ] **Step 5: Update all SELECT queries**

Add `up_to_date_messages, origin_build_invocation_id` to the column list in all three SELECT statements (list_tasks cursor, list_tasks no-cursor, get_task).

- [ ] **Step 6: Build and test**

```bash
bazel run gazelle && aspect build //build-scan/server/db/... && aspect test //build-scan/server/db/...
```

- [ ] **Step 7: Commit**

```bash
git add build-scan/server/db/
git commit -m "feat: wire cache detail fields through DB layer"
```

### Task 6: Add GraphQL resolvers

**Files:**
- Modify: `build-scan/server/graphql/src/lib.rs:232-291`

- [ ] **Step 1: Add resolver methods**

Add after `caching_disabled_explanation` resolver (line 290):

```rust
fn up_to_date_messages(&self) -> Option<&Vec<String>> {
    self.task.up_to_date_messages.as_ref()
}

fn origin_build_invocation_id(&self) -> Option<&str> {
    self.task.origin_build_invocation_id.as_deref()
}
```

- [ ] **Step 2: Build**

```bash
bazel run gazelle && aspect build //build-scan/server/graphql/...
```

- [ ] **Step 3: Commit**

```bash
git add build-scan/server/graphql/src/lib.rs
git commit -m "feat: expose cache detail fields in GraphQL"
```

### Task 7: Wire through service layer

**Files:**
- Modify: `build-scan/server/service/src/lib.rs:133-176`

- [ ] **Step 1: Add builder calls**

Add after `caching_disabled_explanation` builder call (line 170). The lib `Task.origin_build_invocation_id` is populated via assembly (Task 2 Step 3), so it flows through correctly here:

```rust
if let Some(ref msgs) = task.up_to_date_messages {
    if !msgs.is_empty() {
        task_builder.up_to_date_messages(msgs.clone());
    }
}
if let Some(ref id) = task.origin_build_invocation_id {
    task_builder.origin_build_invocation_id(id.clone());
}
```

- [ ] **Step 2: Build and run full test suite**

```bash
bazel run gazelle && aspect test //...
```

Expected: All tests pass. The full backend pipeline now carries both fields.

- [ ] **Step 3: Commit**

```bash
git add build-scan/server/service/src/lib.rs
git commit -m "feat: map cache detail fields in service layer"
```

### Task 8: End-to-end backend verification

- [ ] **Step 1: Start the server and publish a test scan**

```bash
bazel run //build-scan/server/src:main &
cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean build
```

- [ ] **Step 2: Query GraphQL for new fields**

Open GraphiQL at `http://localhost:3000/graphiql` and run:

```graphql
{
  buildScans(first: 1) {
    edges {
      node {
        tasks(first: 5) {
          edges {
            node {
              taskPath
              outcome
              upToDateMessages
              originBuildInvocationId
              originExecutionTime
            }
          }
        }
      }
    }
  }
}
```

Note: `originExecutionTime` was already wired before this plan — it's included here to verify the full picture.

Verify that `originBuildInvocationId` is populated for FROM_CACHE tasks.

- [ ] **Step 3: Run a second build (incremental) to generate UP_TO_DATE tasks**

```bash
cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew build
```

Re-query and verify `upToDateMessages` contains reason strings like "All output files are up to date".

---

## Chunk 2: Frontend — Expandable Task Rows

**Prerequisite:** Chunk 1 must be complete. The `upToDateMessages` and `originBuildInvocationId` fields only carry data after the backend changes land. `originExecutionTime` already works.

Use the `frontend-design` skill when implementing the `TaskCacheDetailComponent`.

### Task 9: Add new fields to GraphQL query

**Files:**
- Modify: `angular/projects/build-scan-web/src/app/scans/scan-detail.component.ts:43-64`

- [ ] **Step 1: Add fields to task node query**

In the `GET_BUILD_SCAN` query, add these fields inside `tasks > edges > node` (after `cachingDisabledExplanation`, around line 56):

```graphql
upToDateMessages
originBuildInvocationId
originExecutionTime
```

(`originExecutionTime` was already in the GraphQL schema but not queried by the frontend.)

- [ ] **Step 2: Verify the app builds**

```bash
cd angular && pnpm build
```

- [ ] **Step 3: Commit**

```bash
git add angular/projects/build-scan-web/src/app/scans/scan-detail.component.ts
git commit -m "feat: query cache detail fields in scan-detail GraphQL"
```

### Task 10: Create `TaskCacheDetailComponent`

A standalone component that renders the compact text summary for a single task's cache behavior. All time values from the backend are in **milliseconds**.

**Files:**
- Create: `angular/projects/build-scan-web/src/app/scans/task-cache-detail/task-cache-detail.component.ts`
- Create: `angular/projects/build-scan-web/src/app/scans/task-cache-detail/task-cache-detail.component.spec.ts`

- [ ] **Step 1: Write tests first**

Create `task-cache-detail.component.spec.ts`:

```typescript
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
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
aspect test //angular/...
```

Expected: FAIL — component doesn't exist yet.

- [ ] **Step 3: Implement the component**

Create `task-cache-detail.component.ts`:

```typescript
import { Component, ChangeDetectionStrategy, input, computed } from "@angular/core";

interface TaskNode {
  outcome: string | null;
  cacheable: boolean | null;
  cachingDisabledReason: string | null;
  cachingDisabledExplanation: string | null;
  upToDateMessages: string[] | null;
  originBuildInvocationId: string | null;
  originExecutionTime: number | null;
  cacheKey: string | null;
  durationMs: number | null;
}

@Component({
  selector: "app-task-cache-detail",
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="px-6 py-3 bg-base-200/50 text-sm">
      @switch (scenario()) {
        @case ("up-to-date") {
          <div class="flex items-center gap-2">
            <span class="text-info">&#10003;</span>
            @for (msg of taskNode().upToDateMessages ?? []; track msg) {
              <span class="text-info">{{ msg }}</span>
            }
          </div>
        }
        @case ("from-cache") {
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-success">&#10003; Cache hit</span>
            @if (taskNode().originExecutionTime; as time) {
              <span class="text-base-content/50">&bull;</span>
              <span class="text-success">Saved {{ formatMs(time) }}</span>
            }
          </div>
          <div class="text-base-content/50 text-xs mt-1">
            @if (taskNode().originBuildInvocationId; as origin) {
              Origin: <span class="font-mono">{{ origin }}</span>
            }
            @if (taskNode().cacheKey; as key) {
              &bull; Key: <span class="font-mono">{{ key }}</span>
            }
          </div>
        }
        @case ("not-cacheable") {
          <div class="flex items-center gap-2">
            <span class="text-warning">&#9940; Caching disabled: {{ taskNode().cachingDisabledReason }}</span>
          </div>
          @if (taskNode().cachingDisabledExplanation; as explanation) {
            <div class="text-base-content/50 text-xs mt-1">{{ explanation }}</div>
          }
        }
        @case ("executed") {
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-amber-400">&#9881; Executed</span>
            @if (taskNode().durationMs; as dur) {
              <span class="text-amber-400">({{ formatMs(dur) }})</span>
            }
            @if (taskNode().cacheKey; as key) {
              <span class="text-base-content/50">&bull;</span>
              <span class="text-base-content/50 text-xs font-mono">Key: {{ key }}</span>
            }
          </div>
        }
        @default {
          <span class="text-base-content/40">No cache detail available</span>
        }
      }
    </div>
  `,
})
export class TaskCacheDetailComponent {
  taskNode = input.required<TaskNode>();

  scenario = computed(() => {
    const node = this.taskNode();
    if (node.outcome === "UpToDate" && node.upToDateMessages?.length) return "up-to-date";
    if (node.outcome === "FromCache") return "from-cache";
    if (node.cacheable === false && node.cachingDisabledReason) return "not-cacheable";
    if (node.cacheable === true && (node.outcome === "Success" || node.outcome === "Failed"))
      return "executed";
    return "fallback";
  });

  formatMs(ms: number): string {
    if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
    return ms + "ms";
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
aspect test //angular/...
```

- [ ] **Step 5: Commit**

```bash
git add angular/projects/build-scan-web/src/app/scans/task-cache-detail/
git commit -m "feat: add TaskCacheDetailComponent for expandable cache detail"
```

### Task 11: Make task table rows expandable

**Files:**
- Modify: `angular/projects/build-scan-web/src/app/scans/tasks-table/tasks-table.component.ts`

- [ ] **Step 1: Write tests for expand/collapse behavior**

Add or create test file `tasks-table.component.spec.ts`. If no spec file exists, create one following the pattern from `cache-breakdown.component.spec.ts`:

```typescript
import { ComponentFixture, TestBed } from "@angular/core/testing";
import { TasksTableComponent } from "./tasks-table.component";

function buildTaskEdge(overrides: Record<string, unknown> = {}) {
  return {
    node: {
      id: crypto.randomUUID(),
      taskPath: ":app:compileJava",
      outcome: "Success",
      cacheable: true,
      durationMs: 1000,
      className: "JavaCompile",
      cachingDisabledReason: null,
      cachingDisabledExplanation: null,
      upToDateMessages: null,
      originBuildInvocationId: null,
      originExecutionTime: null,
      cacheKey: null,
      ...overrides,
    },
  };
}

describe("TasksTableComponent", () => {
  let fixture: ComponentFixture<TasksTableComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TasksTableComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(TasksTableComponent);
  });

  function render(edges: any[] = [buildTaskEdge()]) {
    fixture.componentRef.setInput("taskEdges", edges);
    fixture.componentRef.setInput("taskCount", edges.length);
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  it("does not show detail row by default", () => {
    const el = render();
    expect(el.querySelector("app-task-cache-detail")).toBeNull();
  });

  it("shows detail row when row is clicked", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.click();
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });

  it("hides detail row when clicked again", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.click();
    fixture.detectChanges();
    row.click();
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).toBeNull();
  });

  it("sets aria-expanded on clicked row", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    expect(row.getAttribute("aria-expanded")).toBe("false");
    row.click();
    fixture.detectChanges();
    expect(row.getAttribute("aria-expanded")).toBe("true");
  });

  it("expands row on Enter keydown", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });

  it("expands row on Space keydown", () => {
    const el = render();
    const row = el.querySelector("tbody tr") as HTMLElement;
    row.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    fixture.detectChanges();
    expect(el.querySelector("app-task-cache-detail")).not.toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
aspect test //angular/...
```

- [ ] **Step 3: Add expand/collapse state and import**

Add `TaskCacheDetailComponent` import. Add a signal to track expanded row IDs:

```typescript
import { TaskCacheDetailComponent } from "../task-cache-detail/task-cache-detail.component";

// In imports array:
imports: [TaskCacheDetailComponent],

// In component class:
expandedRows = signal<Set<string>>(new Set());

toggleRow(id: string) {
  this.expandedRows.update((set) => {
    const next = new Set(set);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  });
}
```

- [ ] **Step 4: Update template for expandable rows**

Modify the `@for` loop. Each `<tr>` gets:
- `(click)="toggleRow(edge.node.id)"`, `(keydown.enter)="toggleRow(edge.node.id)"`, and `(keydown.space)="toggleRow(edge.node.id); $event.preventDefault()"` on the row
- `tabindex="0"` and `[attr.aria-expanded]="expandedRows().has(edge.node.id)"` for accessibility
- `class="cursor-pointer hover:bg-base-200"` for visual affordance

After each `<tr>`, add a conditional detail row. Note: `colspan="6"` must match the header column count:

```html
@if (expandedRows().has(edge.node.id)) {
  <tr>
    <td colspan="6" class="p-0">
      <app-task-cache-detail [taskNode]="edge.node" />
    </td>
  </tr>
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
aspect test //angular/...
```

- [ ] **Step 6: Commit**

```bash
git add angular/projects/build-scan-web/src/app/scans/tasks-table/
git commit -m "feat: add expandable rows to tasks table with cache detail"
```

---

## Chunk 3: Frontend — Extend Cache Breakdown

### Task 12: Add time-saved and miss-pattern sections

**Files:**
- Modify: `angular/projects/build-scan-web/src/app/scans/cache-breakdown/cache-breakdown.component.ts`
- Modify: `angular/projects/build-scan-web/src/app/scans/cache-breakdown/cache-breakdown.component.spec.ts`
- Modify: `angular/projects/build-scan-web/src/app/scans/scan-detail.component.ts`

- [ ] **Step 1: Write tests for new computed signals**

Update the `buildTaskEdge` helper in `cache-breakdown.component.spec.ts` to accept optional overrides:

```typescript
function buildTaskEdge(
  outcome: string,
  overrides: Record<string, unknown> = {},
) {
  return {
    node: {
      id: crypto.randomUUID(),
      outcome,
      cacheable: null,
      cachingDisabledReason: null,
      originExecutionTime: null,
      ...overrides,
    },
  };
}
```

Add these new tests:

```typescript
it("computes time saved from FromCache tasks", () => {
  render([
    buildTaskEdge("FromCache", { originExecutionTime: 2000 }),
    buildTaskEdge("FromCache", { originExecutionTime: 3000 }),
    buildTaskEdge("Success"),
  ]);
  expect(fixture.nativeElement.textContent).toContain("5.0s saved");
});

it("does not show time saved when no FromCache tasks", () => {
  render([buildTaskEdge("Success"), buildTaskEdge("Success")]);
  expect(fixture.nativeElement.textContent).not.toContain("saved");
});

it("groups miss patterns by cachingDisabledReason", () => {
  render([
    buildTaskEdge("Success", {
      cacheable: false,
      cachingDisabledReason: "OVERLAPPING_OUTPUTS",
    }),
    buildTaskEdge("Success", {
      cacheable: false,
      cachingDisabledReason: "OVERLAPPING_OUTPUTS",
    }),
    buildTaskEdge("Success", {
      cacheable: false,
      cachingDisabledReason: "BUILD_CACHE_DISABLED",
    }),
  ]);
  const text = fixture.nativeElement.textContent;
  expect(text).toContain("OVERLAPPING_OUTPUTS");
  expect(text).toContain("2");
});

it("shows loading indicator when loading is true", () => {
  fixture.componentRef.setInput("taskEdges", [buildTaskEdge("Success")]);
  fixture.componentRef.setInput("taskCount", 1);
  fixture.componentRef.setInput("loading", true);
  fixture.detectChanges();
  expect(fixture.nativeElement.textContent).toContain("Loading");
});

it("hides loading indicator when loading is false", () => {
  fixture.componentRef.setInput("taskEdges", [buildTaskEdge("Success")]);
  fixture.componentRef.setInput("taskCount", 1);
  fixture.componentRef.setInput("loading", false);
  fixture.detectChanges();
  expect(fixture.nativeElement.textContent).not.toContain("Loading");
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
aspect test //angular/...
```

- [ ] **Step 3: Add typed interface and computed signals**

Add to the component file:

```typescript
interface TaskEdge {
  node: {
    outcome: string;
    cacheable: boolean | null;
    cachingDisabledReason: string | null;
    originExecutionTime: number | null;
  };
}
```

Update the input type from `any[]` to use this interface. Add new input and computed signals to the component class:

```typescript
loading = input<boolean>(false);

timeSaved = computed(() => {
  const edges = this.taskEdges() as TaskEdge[];
  return edges
    .filter((e) => e.node.outcome === "FromCache" && e.node.originExecutionTime)
    .reduce((sum, e) => sum + (e.node.originExecutionTime ?? 0), 0);
});

missPatterns = computed(() => {
  const edges = this.taskEdges() as TaskEdge[];
  const patterns = new Map<string, number>();
  for (const e of edges) {
    const reason = e.node.cachingDisabledReason;
    if (reason) {
      patterns.set(reason, (patterns.get(reason) || 0) + 1);
    }
  }
  return [...patterns.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([reason, count]) => ({ reason, count }));
});
```

- [ ] **Step 4: Add template sections below the legend**

After the existing legend `@for` block, add:

```html
@if (loading()) {
  <div class="text-sm opacity-60">Loading all tasks...</div>
} @else {
  @if (timeSaved() > 0) {
    <div class="stat p-2">
      <div class="stat-title text-xs">Time Saved by Cache</div>
      <div class="stat-value text-lg text-success">
        {{ (timeSaved() / 1000).toFixed(1) }}s saved
      </div>
    </div>
  }
  @if (missPatterns().length > 0) {
    <div class="mt-3">
      <div class="text-xs font-semibold opacity-70 mb-1">Not Cacheable</div>
      @for (pattern of missPatterns(); track pattern.reason) {
        <div class="flex items-center gap-2 text-sm">
          <span class="badge badge-sm badge-warning">{{ pattern.count }}</span>
          <span class="opacity-80">{{ pattern.reason }}</span>
        </div>
      }
    </div>
  }
}
```

- [ ] **Step 5: Pass `loading` input from scan-detail**

In `scan-detail.component.ts`, add a signal and update the pagination handler:

```typescript
// Add to component class:
tasksLoading = signal(true);
```

In the `tap` operator inside the `scan$` pipeline (around lines 157-168), update to detect when pagination is complete:

```typescript
tap((scan) => {
  if (scan.tasks.pageInfo.hasNextPage) {
    queryRef.fetchMore({
      variables: { afterTasks: scan.tasks.pageInfo.endCursor },
    });
  } else {
    this.tasksLoading.set(false);
  }
}),
```

Update the template to pass the loading state:

```html
<app-cache-breakdown [taskEdges]="scan.tasks.edges" [taskCount]="scan.taskCount" [loading]="tasksLoading()" />
```

- [ ] **Step 6: Run tests**

```bash
aspect test //angular/...
```

- [ ] **Step 7: Commit**

```bash
git add angular/projects/build-scan-web/src/app/scans/cache-breakdown/ angular/projects/build-scan-web/src/app/scans/scan-detail.component.ts
git commit -m "feat: add time-saved and miss-pattern sections to cache breakdown"
```

### Task 13: Update sidebar navigation

**Files:**
- Modify: `angular/projects/build-scan-web/src/app/scans/scan-sidebar/scan-sidebar.component.ts:196-207`

- [ ] **Step 1: Rename sidebar item**

Change the `cache-breakdown` section label (line 199):

```typescript
{ id: "cache-breakdown", label: "Cache Performance", icon: "▤" },
```

- [ ] **Step 2: Build and test**

```bash
aspect test //angular/...
```

- [ ] **Step 3: Commit**

```bash
git add angular/projects/build-scan-web/src/app/scans/scan-sidebar/scan-sidebar.component.ts
git commit -m "feat: rename sidebar item to Cache Performance"
```

---

## Chunk 4: Phase 2 — BUILD_CACHE_* Event Decoders (Reverse Engineering)

> **Note:** This chunk requires reverse engineering the Gradle Build Scan binary format for 12 new event types. The exact wire format is unknown and must be determined by decompiling the Gradle plugin JAR and analyzing binary payloads. The steps below provide the framework; exact field structures will be discovered during implementation.

### Task 14: Decompile and analyze BUILD_CACHE_* serializers

- [ ] **Step 1: Extract and decompile the Gradle plugin JAR**

```bash
# Find the JAR
find ~/.gradle/caches -name "develocity-gradle-plugin-*.jar" | head -1

# Extract to /tmp/decompiled if not already done
# Use cfr or procyon decompiler
```

- [ ] **Step 2: Find serializers for wire IDs 41-48, 144-147**

Look in `com/gradle/scan/agent/serialization/scan/serializer/kryo/` for classes that handle these event types. Map each wire ID to its serializer class and document the field structure.

- [ ] **Step 3: Document findings**

Add decoded structures to `~/Documents/Everything/Learnings/Gradle Build Scan Binary Format — Reverse Engineering Notes.md`.

- [ ] **Step 4: Commit documentation**

### Task 15: Implement BUILD_CACHE_* decoders

For each event pair (local_load, remote_load, pack, unpack, local_store, remote_store):

- [ ] **Step 1: Write a test with a binary fixture for the "started" event**
- [ ] **Step 2: Implement the decoder**
- [ ] **Step 3: Write a test for the "finished" event**
- [ ] **Step 4: Implement the decoder**
- [ ] **Step 5: Register both decoders in the event registry**
- [ ] **Step 6: Build and test**

### Task 16: Create `CacheOperations` model and DB table

- [ ] **Step 1: Create DB migration `006_task_cache_operations.sql`**

```sql
CREATE TABLE task_cache_operations (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    operation_type TEXT NOT NULL,
    succeeded INTEGER NOT NULL,
    duration_ms INTEGER,
    UNIQUE(task_id, operation_type)
);
```

- [ ] **Step 2: Add domain model, DB layer, GraphQL type**
- [ ] **Step 3: Wire through assembly and service**
- [ ] **Step 4: Build and test**

### Task 17: Frontend — Display cache operations in expandable row

- [ ] **Step 1: Update `TaskCacheDetailComponent` to render cache operation flow**

Add the compact text pipeline: `X Local miss -> X Remote miss -> Executed (2.3s) -> Stored to remote (120ms)`

- [ ] **Step 2: Update `CacheBreakdownComponent` with tier breakdown**

Add cache tier stats (local hits / remote hits / misses) computed from operation data.

- [ ] **Step 3: Test and commit**

### Task 18: End-to-end dogfooding

- [ ] **Step 1: Run server, publish scans, verify all data flows**
- [ ] **Step 2: Verify expandable rows show correct data for each scenario**
- [ ] **Step 3: Verify cache breakdown shows time saved and miss patterns**
- [ ] **Step 4: Create PR**
