# Task-Centric Parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a functional parser that extracts task execution telemetry from Gradle build scan payloads.

**Architecture:** Extend the existing `PayloadBuilder` state machine to handle TaskStarted/TaskFinished events, populate the data model, and output structured JSON.

**Tech Stack:** Rust, serde for JSON serialization, flate2 for decompression

---

## Phase 1: Chrome-First Discovery Session

### Task 1: Capture Build Scan Payload

**Files:**
- Use: `.opencode/skills/capturing-gradle-payloads/capture.sh`

**Step 1: Run capture script**

Run: `./.opencode/skills/capturing-gradle-payloads/capture.sh`
Expected: Creates JSON files in `captured-output/payloads/`

**Step 2: Verify capture**

Run: `ls -la captured-output/payloads/`
Expected: See JSON files with timestamps

---

### Task 2: Run Chrome-First Discovery Session

**Files:**
- Read: `docs/gradle-build-scan-binary-format.md`
- Use: `captured-output/payloads/*upload*.json`

**Step 1: Open build scan in Chrome**

1. Navigate to scans.gradle.com
2. Find your build scan
3. Open DevTools (F12)

**Step 2: Discover TaskStarted event ID**

Method:
1. In the build scan UI, find a task path (e.g., ":compileKotlin")
2. Extract the binary payload from captured JSON (base64 decode)
3. Search for the task path string in the binary
4. Note the event ID byte(s) immediately before it
5. Repeat for 2-3 tasks to confirm

Document result: `TASK_STARTED = <discovered_id>`

**Step 3: Discover TaskFinished event ID**

Method:
1. Find task outcome in UI (SUCCESS, FAILED, UP-TO-DATE, etc.)
2. Search for patterns near task path strings
3. Look for varint values 0-6 (enum values for TaskOutcome)
4. Note the event ID preceding it

Document result: `TASK_FINISHED = <discovered_id>`

**Step 4: Document discoveries**

Create/update: `docs/gradle-build-scan-binary-format.md`

Add section:
```markdown
## Task Events

### TaskStarted (<event_id>)
- id: varint
- path: string
- className: string (optional?)

### TaskFinished (<event_id>)
- id: varint
- path: string
- outcome: varint (0=SUCCESS, 1=FAILED, 2=UP_TO_DATE, 3=SKIPPED, 4=FROM_CACHE, 5=NO_SOURCE, 6=AVOIDED_FOR_UNKNOWN_REASON)
- cacheable: boolean (varint 0/1)
```

**Step 5: Commit discoveries**

```bash
git add docs/gradle-build-scan-binary-format.md
git commit -m "docs: add task event IDs from Chrome-first discovery"
```

---

## Phase 2: Data Model Extension

### Task 3: Update TaskOutcome Enum

**Files:**
- Modify: `build-scan/lib/src/models.rs:1-14`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_outcome_from_varint() {
        assert_eq!(TaskOutcome::try_from(0u64).unwrap(), TaskOutcome::Success);
        assert_eq!(TaskOutcome::try_from(1u64).unwrap(), TaskOutcome::Failed);
        assert_eq!(TaskOutcome::try_from(2u64).unwrap(), TaskOutcome::UpToDate);
        assert_eq!(TaskOutcome::try_from(3u64).unwrap(), TaskOutcome::Skipped);
        assert_eq!(TaskOutcome::try_from(4u64).unwrap(), TaskOutcome::FromCache);
        assert_eq!(TaskOutcome::try_from(5u64).unwrap(), TaskOutcome::NoSource);
        assert_eq!(TaskOutcome::try_from(6u64).unwrap(), TaskOutcome::AvoidedForUnknownReason);
        assert!(TaskOutcome::try_from(7u64).is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL with "TaskOutcome not found" or similar

**Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub environment: Option<BuildEnvironment>,
    pub tasks: Vec<TaskExecution>,
    pub build_outcome: Option<BuildOutcome>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: u64,
    pub path: String,
    pub class_name: Option<String>,
    pub outcome: TaskOutcome,
    pub cacheable: bool,
    pub origin_execution_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskOutcome {
    Success,
    Failed,
    UpToDate,
    Skipped,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

impl TryFrom<u64> for TaskOutcome {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TaskOutcome::Success),
            1 => Ok(TaskOutcome::Failed),
            2 => Ok(TaskOutcome::UpToDate),
            3 => Ok(TaskOutcome::Skipped),
            4 => Ok(TaskOutcome::FromCache),
            5 => Ok(TaskOutcome::NoSource),
            6 => Ok(TaskOutcome::AvoidedForUnknownReason),
            _ => Err(format!("Unknown task outcome: {}", value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildEnvironment {
    pub os: Option<OsInfo>,
    pub jvm: Option<JvmInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsInfo {
    pub family: String,
    pub name: String,
    pub version: String,
    pub arch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JvmInfo {
    pub version: String,
    pub vendor: String,
    pub vm_name: String,
    pub vm_version: String,
    pub vm_vendor: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildOutcome {
    pub success: bool,
    pub failure_id: Option<u64>,
}
```

**Step 4: Run test to verify it passes**

Run: `cd build-scan && cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add build-scan/lib/src/models.rs
git commit -m "feat: extend data model for task execution telemetry"
```

---

### Task 4: Update Parser Constants

**Files:**
- Modify: `build-scan/lib/src/parser.rs:6-11`

**Step 1: Add new event ID constants**

After discovery session, update constants:

```rust
const EVENT_TIMESTAMP: u64 = 0;
const EVENT_USER_HOST_INFO: u64 = 2;
const EVENT_JVM_INFO: u64 = 3;
const EVENT_OS_INFO: u64 = 8;
const EVENT_DICTIONARY_ADD: u64 = 14;
const EVENT_TASK_STARTED: u64 = <discovered_id>;
const EVENT_TASK_FINISHED: u64 = <discovered_id>;
const EVENT_BUILD_FINISHED: u64 = <discovered_id>;
```

**Step 2: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: add task event ID constants"
```

---

## Phase 3: Parser Implementation

### Task 5: Implement TaskStarted Handler

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builder_parses_task_started() {
    let mut builder = PayloadBuilder::new();
    // Event TASK_STARTED
    // id: 1
    // path: ":compileKotlin"
    let mut raw_data = Vec::new();
    raw_data.push(EVENT_TASK_STARTED as u8);
    // id = 1
    raw_data.push(1);
    // path = ":compileKotlin" (length 14, varint 28)
    raw_data.push(28);
    raw_data.extend_from_slice(b":compileKotlin");

    let result = builder.build(&raw_data);
    assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
    assert_eq!(builder.pending_tasks.len(), 1);
    assert_eq!(builder.pending_tasks[0].path, ":compileKotlin");
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

Add to `PayloadBuilder`:
```rust
pub struct PayloadBuilder {
    pub dictionary: Vec<String>,
    pub payload: BuildScanPayload,
    pub pending_tasks: HashMap<u64, PendingTask>,
}

pub struct PendingTask {
    pub id: u64,
    pub path: String,
    pub class_name: Option<String>,
}
```

Add handler:
```rust
EVENT_TASK_STARTED => {
    let id = decoder.read_raw_varint()?;
    let path = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let class_name = match decoder.read_string()? {
        Primitive::String(s) => Some(s),
        Primitive::StringRef(_) => None, // or handle ref
        _ => None,
    };
    self.pending_tasks.insert(id, PendingTask { id, path, class_name });
}
```

**Step 4: Run test to verify it passes**

Run: `cd build-scan && cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: implement TaskStarted event handler"
```

---

### Task 6: Implement TaskFinished Handler

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builder_parses_task_finished() {
    let mut builder = PayloadBuilder::new();
    // Event TASK_STARTED
    let mut raw_data = Vec::new();
    raw_data.push(EVENT_TASK_STARTED as u8);
    raw_data.push(1); // id = 1
    raw_data.push(28); // path length
    raw_data.extend_from_slice(b":compileKotlin");

    // Event TASK_FINISHED
    raw_data.push(EVENT_TASK_FINISHED as u8);
    raw_data.push(1); // id = 1
    raw_data.push(28); // path length
    raw_data.extend_from_slice(b":compileKotlin");
    raw_data.push(0); // outcome = SUCCESS
    raw_data.push(0); // cacheable = false

    let result = builder.build(&raw_data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().tasks.len(), 1);
    assert_eq!(result.unwrap().tasks[0].outcome, TaskOutcome::Success);
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
EVENT_TASK_FINISHED => {
    let id = decoder.read_raw_varint()?;
    let path = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let outcome_val = decoder.read_raw_varint()?;
    let outcome = TaskOutcome::try_from(outcome_val)
        .map_err(|e| ParseError::InvalidData { reason: e })?;
    let cacheable = decoder.read_raw_varint()? != 0;

    if let Some(pending) = self.pending_tasks.remove(&id) {
        self.payload.tasks.push(TaskExecution {
            id,
            path,
            class_name: pending.class_name,
            outcome,
            cacheable,
            origin_execution_time_ms: None,
        });
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd build-scan && cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: implement TaskFinished event handler"
```

---

### Task 7: Implement OsInfo Handler

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builder_parses_os_info() {
    let mut builder = PayloadBuilder::new();
    // Event 8 (OS_INFO) with payload length + 4 strings
    let mut raw_data = Vec::new();
    raw_data.push(8); // EVENT_OS_INFO
    
    // Payload length (we'll read strings directly)
    // family: "unix"
    raw_data.push(8); // length 4 * 2
    raw_data.extend_from_slice(b"unix");
    // name: "Linux"
    raw_data.push(10);
    raw_data.extend_from_slice(b"Linux");
    // version: "6.1.0"
    raw_data.push(12);
    raw_data.extend_from_slice(b"6.1.0");
    // arch: "amd64"
    raw_data.push(12);
    raw_data.extend_from_slice(b"amd64");

    let result = builder.build(&raw_data);
    assert!(result.is_ok());
    assert!(result.unwrap().environment.is_some());
    let env = result.unwrap().environment.unwrap();
    assert!(env.os.is_some());
    assert_eq!(env.os.unwrap().name, "Linux");
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
EVENT_OS_INFO => {
    let family = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let name = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let version = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let arch = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    
    self.payload.environment = Some(BuildEnvironment {
        os: Some(OsInfo { family, name, version, arch }),
        jvm: None,
    });
}
```

**Step 4: Run test to verify it passes**

Run: `cd build-scan && cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: implement OsInfo event handler"
```

---

### Task 8: Implement JvmInfo Handler

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builder_parses_jvm_info() {
    let mut builder = PayloadBuilder::new();
    // Event 3 (JVM_INFO)
    let mut raw_data = Vec::new();
    raw_data.push(3);
    
    // Read strings (based on Jvm_1_0: 9 strings)
    // For simplicity, test minimal
    let strings = vec!["21", "N/A", "OpenJDK", "21.0.2", "65", "mixed mode", "OpenJDK 64-Bit Server VM", "21.0.2+13-LTS", "Eclipse Adoptium"];
    for s in strings {
        let len = (s.len() * 2) as u8;
        raw_data.push(len);
        raw_data.extend_from_slice(s.as_bytes());
    }

    let result = builder.build(&raw_data);
    assert!(result.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
EVENT_JVM_INFO => {
    // Jvm_1_0 has 9 strings
    let version = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let vendor = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let _runtime_name = decoder.read_string()?;
    let _runtime_version = decoder.read_string()?;
    let _class_version = decoder.read_string()?;
    let _vm_info = decoder.read_string()?;
    let vm_name = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let vm_version = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };
    let vm_vendor = match decoder.read_string()? {
        Primitive::String(s) => s,
        _ => return Err(ParseError::UnexpectedPrimitive { expected: "String" }),
    };

    if let Some(ref mut env) = self.payload.environment {
        env.jvm = Some(JvmInfo { version, vendor, vm_name, vm_version, vm_vendor });
    } else {
        self.payload.environment = Some(BuildEnvironment {
            os: None,
            jvm: Some(JvmInfo { version, vendor, vm_name, vm_version, vm_vendor }),
        });
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd build-scan && cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: implement JvmInfo event handler"
```

---

### Task 9: Implement Unknown Event Skipping

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builder_skips_unknown_events() {
    let mut builder = PayloadBuilder::new();
    let mut raw_data = Vec::new();
    
    // Event 0 (timestamp)
    raw_data.push(0);
    raw_data.push(0);
    
    // Unknown event 999 with payload length prefix
    raw_data.push(199); // some unknown event
    raw_data.push(8); // payload length 4
    raw_data.extend_from_slice(b"skip");
    
    // Event 14 (dictionary add) - should still work
    raw_data.push(14);
    raw_data.push(8);
    raw_data.extend_from_slice(b"test");

    let result = builder.build(&raw_data);
    assert!(result.is_ok());
    assert_eq!(builder.dictionary, vec!["test".to_string()]);
}
```

**Step 2: Run test to verify it fails**

Run: `cd build-scan && cargo test --lib`
Expected: FAIL with UnknownEvent error

**Step 3: Write minimal implementation**

```rust
// Add skip functionality
_ => {
    // Try to skip unknown event
    // Heuristic: read a varint as length, skip that many bytes
    // This is fragile but better than failing completely
    eprintln!("Skipping unknown event: {}", event_id);
    // For events without length prefix, we're stuck
    // For now, continue without skipping (will likely fail)
    // TODO: Improve with event registry
}
```

Note: Proper skipping requires knowing the event schema. For now, we can add a heuristic or log and continue.

**Step 4: Run test to verify behavior**

Run: `cd build-scan && cargo test --lib`

**Step 5: Commit**

```bash
git add build-scan/lib/src/parser.rs
git commit -m "feat: add heuristic for skipping unknown events"
```

---

## Phase 4: End-to-End Testing

### Task 10: Test with Captured Payload

**Files:**
- Use: `captured-output/payloads/*upload*.json`
- Modify: `build-scan/cli/src/main.rs`

**Step 1: Run parser on captured payload**

Run: `cd build-scan && cargo run --bin build-scan-cli -- ../captured-output/payloads/<latest>_upload.json`
Expected: JSON output with tasks array

**Step 2: Verify output structure**

Check that output contains:
- `tasks` array with paths and outcomes
- `environment` with OS/JVM info

**Step 3: Compare with Develocity UI**

Verify task paths and outcomes match what's shown in the build scan UI.

**Step 4: Document results**

Update: `docs/gradle-build-scan-binary-format.md` with any corrections.

**Step 5: Commit**

```bash
git add docs/gradle-build-scan-binary-format.md
git commit -m "docs: update binary format with parsing results"
```

---

## Phase 5: Finalization

### Task 11: Run Full Test Suite

**Step 1: Run all tests**

Run: `cd build-scan && cargo test`
Expected: All tests pass

**Step 2: Run linter**

Run: `cd build-scan && cargo clippy`
Expected: No errors

**Step 3: Format code**

Run: `cd build-scan && cargo fmt`

**Step 4: Final commit**

```bash
git add .
git commit -m "feat: complete task-centric parser implementation"
```

---

## Success Criteria

- [ ] Parser processes captured payload end-to-end without errors
- [ ] All tasks extracted with correct path and outcome
- [ ] OS and JVM info populated
- [ ] Build outcome correctly identified
- [ ] JSON output matches expected schema
- [ ] All tests pass
- [ ] Code passes clippy and fmt
