# Task-Centric Build Scan Parser Design

**Date:** 2026-02-21  
**Status:** Approved  
**Goal:** Build a functional parser that extracts task execution telemetry from Gradle build scan payloads.

## Scope

### In Scope
- Parse payloads end-to-end without stopping at unknown events
- Extract: task paths, durations, outcomes, execution order
- Extract: OS info, JVM info, build outcome
- Output structured JSON with full build telemetry

### Out of Scope (for now)
- Dependencies resolution data
- Cache statistics
- Memory/GC info
- Server UI/display
- Multiple plugin version support

## Architecture

### Two-Phase Approach

```
Phase 1: Chrome-First Discovery Session
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Run build with  │────▶│ Capture payload │────▶│ Open Develocity │
│   --scan        │     │  via proxy      │     │   UI in Chrome  │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                         │
         ┌───────────────────────────────────────────────┘
         ▼
┌─────────────────────────────────────────────────────────────┐
│ Chrome DevTools: Correlate UI values with binary bytes      │
│  - Find task duration in UI → search bytes for that value   │
│  - Find task outcome in UI → search bytes for that enum     │
│  - Map byte offsets to event IDs                            │
└─────────────────────────────────────────────────────────────┘

Phase 2: Parser Implementation
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Extend parser   │────▶│ Populate        │────▶│ Output JSON     │
│ with new events │     │ TaskExecution   │     │ with all tasks  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### Key Insight

The binary format maps directly to the [Develocity Event Model](https://docs.gradle.com/enterprise/event-model-javadoc/com/gradle/scan/eventmodel/gradle/package-summary.html). Each event type has versioned classes with known fields.

## Data Model

### Task Execution Events

Based on official event model:

| Event | Fields | Purpose |
|-------|--------|---------|
| `TaskStarted_1_6` | `id`, `buildPath`, `path`, `className`, `thread` | Marks task execution start |
| `TaskFinished_1_8` | `id`, `path`, `outcome`, `cacheable`, `originExecutionTime` | Marks task execution end |

### TaskOutcome Enum

Values from `TaskOutcome_1`:
- `SUCCESS`
- `FAILED`
- `UP_TO_DATE`
- `SKIPPED`
- `FROM_CACHE`
- `NO_SOURCE`
- `AVOIDED_FOR_UNKNOWN_REASON`

### Environment Events

| Event | Fields |
|-------|--------|
| `Os_1_0` | `family`, `name`, `version`, `arch` |
| `Jvm_1_0` | `version`, `vendor`, `vmName`, `vmVersion`, `vmVendor`, `runtimeName`, `runtimeVersion`, `classVersion`, `vmInfo` |

### Build Events

| Event | Fields |
|-------|--------|
| `BuildStarted_1_0` | (no fields) |
| `BuildFinished_1_1` | `failure`, `failureId` |

### Rust Model

```rust
pub struct BuildScanPayload {
    pub environment: BuildEnvironment,
    pub tasks: Vec<TaskExecution>,
    pub build_outcome: BuildOutcome,
}

pub struct TaskExecution {
    pub id: u64,
    pub path: String,
    pub class_name: Option<String>,
    pub outcome: TaskOutcome,
    pub cacheable: bool,
    pub origin_execution_time_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskOutcome {
    Success,
    Failed,
    UpToDate,
    Skipped,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

pub struct BuildEnvironment {
    pub os: OsInfo,
    pub jvm: JvmInfo,
}

pub struct OsInfo {
    pub family: String,
    pub name: String,
    pub version: String,
    pub arch: String,
}

pub struct JvmInfo {
    pub version: String,
    pub vendor: String,
    pub vm_name: String,
    pub vm_version: String,
    pub vm_vendor: String,
}

pub struct BuildOutcome {
    pub success: bool,
    pub failure_id: Option<u64>,
}
```

## Chrome-First Discovery Session

### Goal

Map binary event IDs to the official event types using Chrome DevTools to correlate UI values with binary bytes.

### Prerequisites

1. Run build with `--scan` through proxy to capture payload
2. Open build scan on scans.gradle.com in Chrome
3. Have the captured binary payload ready for analysis

### Discovery Targets (Priority Order)

| # | Event Type | Discovery Method | Expected Format |
|---|------------|------------------|-----------------|
| 1 | `TaskStarted` | Find task path string → note preceding event ID | String after event ID |
| 2 | `TaskFinished` | Find task outcome in UI → search for enum value | Varint (0-6 for outcomes) |
| 3 | `Os_1_0` | Find OS name string (e.g., "Linux") → note event ID | Already known as 8 |
| 4 | `Jvm_1_0` | Find JVM version string → note event ID | Already known as 3 |
| 5 | `BuildFinished` | Find build failure status → note event ID | Boolean or failure ID |

### Method Per Target

1. Find the value in Develocity UI
2. Search for that value in the captured binary payload (as string or varint)
3. Note the event ID byte(s) that precede it
4. Verify by checking multiple occurrences

### Session Duration Estimate

1-2 hours for task-centric events

## Parser Implementation

### Current State

The parser has infrastructure but stops at unknown events.

### Implementation Changes

#### 1. Add Event ID Constants

```rust
pub const TASK_STARTED: u32 = ?;      // Discovered in session
pub const TASK_FINISHED: u32 = ?;     // Discovered in session
pub const OS_INFO: u32 = 8;           // Already known
pub const JVM_INFO: u32 = 3;          // Already known
pub const BUILD_FINISHED: u32 = ?;    // Discovered in session
```

#### 2. Extend PayloadBuilder State Machine

```rust
match event_id {
    TASK_STARTED => self.handle_task_started(&mut decoder)?,
    TASK_FINISHED => self.handle_task_finished(&mut decoder)?,
    OS_INFO => self.handle_os_info(&mut decoder)?,
    JVM_INFO => self.handle_jvm_info(&mut decoder)?,
    BUILD_FINISHED => self.handle_build_finished(&mut decoder)?,
    _ => self.skip_unknown_event(event_id, &mut decoder)?,
}
```

#### 3. Implement Event Handlers

- `handle_task_started`: Read id (varint), path (string), className (string) → create pending task
- `handle_task_finished`: Read id (varint), path (string), outcome (varint enum) → finalize task
- `handle_os_info`: Read 4 strings → populate OsInfo
- `handle_jvm_info`: Read strings (count varies by version) → populate JvmInfo
- `handle_build_finished`: Read failure info → populate BuildOutcome

#### 4. Output JSON

Via existing CLI tool in `build-scan/cli/`.

## Known Event IDs (Current)

| Event ID | Name | Format |
|----------|------|--------|
| 0 | Timestamp | `[0] [varint ms since epoch]` |
| 1 | Unknown | Single varint |
| 2 | User/Host | Two strings (username, hostname) |
| 3 | JVM Info | Payload length + strings |
| 8 | OS Info | Payload length + 4 strings |
| 14 | Dictionary Add | String |
| 58 | Task Path | String |

## Success Criteria

1. Parser processes payload end-to-end without errors
2. All tasks extracted with correct path, outcome
3. OS and JVM info populated
4. Build outcome correctly identified
5. JSON output matches expected schema
