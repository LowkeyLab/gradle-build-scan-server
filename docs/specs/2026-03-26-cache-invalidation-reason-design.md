# Cache Invalidation Reason — Design Spec

## Goal

Capture and display cache miss/invalidation reasons for build scan tasks, answering "why wasn't this task served from cache?" at both task-level and cache-operation-level granularity.

## Current State

The backend already parses and stores from `TaskFinished` events:
- `cacheable: bool` — whether the task is cache-eligible
- `caching_disabled_reason` — category string (e.g., OVERLAPPING_OUTPUTS)
- `caching_disabled_explanation` — human-readable detail
- `cache_key` — hex-encoded build cache key
- `outcome` — includes FromCache variant for cache hits
- `origin_execution_time` — already fully wired end-to-end (parser -> domain -> DB -> GraphQL)

Two additional fields are **decoded but discarded**: `up_to_date_messages` (bit 11), `origin_build_invocation_id` (bit 7).

Fourteen `BUILD_CACHE_*` events (wire IDs 37-48, 144-148, 155) exist in the binary format but have no decoders.

The frontend has an existing `CacheBreakdownComponent` that renders a stacked bar of task outcomes (cache hit, up-to-date, executed, etc.) with percentages.

## Architecture

Two-phase approach: task-level fields first, cache operation events second.

### Phase 1: Task-Level Fields

Surface the two decoded-but-discarded fields from `TaskFinished`:

| Field | Bit | Type | Purpose |
|-------|-----|------|---------|
| `up_to_date_messages` | 11 | `Vec<String>` | Why a task's outputs are still valid |
| `origin_build_invocation_id` | 7 | `String` | Which build produced the cached result |

Note: `origin_execution_time` (bit 9) is already wired through the full stack and requires no backend work.

**Backend changes:**
- `task_finished.rs` — Return these two fields instead of discarding
- `models.rs` — Add fields to `Task`: `up_to_date_messages: Option<Vec<String>>`, `origin_build_invocation_id: Option<String>`
- `assembly.rs` — Map new fields from `FinishedInfo` into domain model
- DB migration — `up_to_date_messages TEXT` (JSON-serialized array, `null` when absent), `origin_build_invocation_id TEXT`
- GraphQL — `upToDateMessages: [String!]` (nullable field, `null` when absent, non-empty list when present), `originBuildInvocationId: String`

### Phase 2: Cache Operation Events

Reverse-engineer and decode `BUILD_CACHE_*` event pairs:

| Event Pair | Wire IDs | Captures |
|------------|----------|----------|
| Local Load | 144/145 | Local cache lookup hit/miss |
| Remote Load | 43/44 | Remote cache lookup hit/miss |
| Pack | 41/42 | Packing outputs for storage |
| Unpack | 47/48 | Unpacking cached outputs |
| Local Store | 146/147 | Storing to local cache |
| Remote Store | 45/46 | Storing to remote cache |

Wire IDs 37 (BUILD_CACHE_CONFIGURATION), 148 (BUILD_CACHE_REMOTE_DISABLED), and 155 (BUILD_CACHE_REMOTE_FALLBACK) are global build-level events, not per-task. They are out of scope for this feature but may be useful for a future "build cache configuration" display.

**Backend changes:**
- New decoder files for each event pair
- `CacheOperations` model aggregating per-task cache lifecycle
- DB table: `task_cache_operations` with columns:
  - `id TEXT PRIMARY KEY` (UUID, consistent with existing schema)
  - `task_id TEXT NOT NULL REFERENCES tasks(id)` (foreign key to tasks table)
  - `operation_type TEXT NOT NULL` (e.g., "local_load", "remote_load", "pack", "unpack", "local_store", "remote_store")
  - `succeeded INTEGER NOT NULL` (0/1)
  - `duration_ms INTEGER`
  - `UNIQUE(task_id, operation_type)` — one operation per type per task
- GraphQL: `cacheOperations` field on `Task` type

## Frontend

### Expandable Row Detail

Tasks table rows become expandable. Clicking reveals a compact text summary of cache behavior. Rows are keyboard-accessible (Enter/Space to toggle, `aria-expanded` attribute).

**Cache miss (executed):**
```
X Local miss -> X Remote miss -> Executed (2.3s) -> Stored to remote (120ms)
Key: a3f8b2c1d4e5...
```

**Cache hit:**
```
Local hit (1ms) -> Unpacked (15ms) | Saved 4.1s
Origin: build-2024-03-25-abc123 | Key: f7e6d5c4b3a2...
```

**Up-to-date:**
```
All output files are up to date
```

**Not cacheable:**
```
Caching disabled: OVERLAPPING_OUTPUTS
Gradle detected that this task has outputs that overlap with task ':app:integrationTest'
```

**Implementation:**
- New `TaskCacheDetailComponent` — standalone component rendering compact text flow
- Modify `TasksTableComponent` — click-to-expand rows, lazy-load cache detail
- Phase 1 data displays immediately; Phase 2 data displays once decoders exist
- If no cache detail exists for a task, show "No cache detail available"

### Cache Performance Section (extends CacheBreakdownComponent)

Extend the existing `CacheBreakdownComponent` rather than creating a new component. The current stacked bar remains; new content is added below it:

- **Time saved** — total time saved via cache hits (sum of `originExecutionTime` for FROM-CACHE tasks) vs total build time
- **Common miss patterns** — group tasks by `cachingDisabledReason` with counts (e.g., "12 tasks: OVERLAPPING_OUTPUTS")
- **Cache tier breakdown** (Phase 2) — local hits / remote hits / misses, added once cache operation data exists
- **Cache store summary** (Phase 2) — tasks stored to local/remote after execution

Note: Aggregations are computed from the already-fetched task list. Since tasks load via pagination (100 per page with `fetchMore`), the component should show a loading indicator until all pages are fetched. The existing `CacheBreakdownComponent` has this same latent issue — fixing it here benefits both.

**Implementation:**
- Extend `CacheBreakdownComponent` with new sections below the stacked bar
- Add sidebar nav item "Cache Performance" linking to this section

## Testing

**Backend:**
- Unit tests for the two newly-surfaced `TaskFinished` fields (`up_to_date_messages`, `origin_build_invocation_id`)
- Unit tests for each `BUILD_CACHE_*` event decoder (Phase 2)
- Integration test: parse real payload, verify cache operations assemble per task

**Frontend:**
- `TaskCacheDetailComponent` — test hit/miss/not-cacheable/up-to-date scenarios
- `CacheBreakdownComponent` extensions — test time-saved calculation, miss pattern grouping, loading state during pagination

## Error Handling

- Cache operation events are optional — older plugin versions may not include them. Expandable row gracefully shows only Phase 1 data (up-to-date messages, origin info, caching disabled reason).
- If no cache detail exists for a task, the expanded row shows "No cache detail available"
- Malformed cache events are logged and skipped, not fatal

## Dogfooding

Generate test scans: `cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean build`
