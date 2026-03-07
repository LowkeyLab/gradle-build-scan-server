# Test Event Decoders Design

## Context

The build scan server decodes 34 Gradle build scan event types but has no support for test execution events. By capturing a build scan from a Gradle build with actual test execution (not from-cache), we identified 7 new wire IDs forming a complete test lifecycle.

## Discovered Test Event Lifecycle

Events appear in this order during test execution:

| Wire ID | Count | Event Name | Body Size | Description |
|---------|-------|------------|-----------|-------------|
| 548 | 3 | PluginResolution | 151-232B | Test worker classpath JAR resolution (Maven URLs) |
| 127 | 2 | TestExecutorIdentity | ~40B | Executor name + ID ("Gradle Test Executor N") |
| 123 | 2 | TestExecutorConfirmation | 10B | Executor started confirmation with boolean flag |
| 128 | 2 | TestExecutorStarted | 17B | Executor ID + hash, timing anchor |
| 798 | 9 | TestCase | 51-90B | Class + method names, executor ID. Two forms: class-level (flags=0x00) and method-level (flags=0x05) |
| 284 | 9 | TestResult | 17B | Per-test outcome with executor ID + result hash |
| 803 | 3 | TestExecutorFinished | 18B | Executor ID + 0xFF sentinel (success) |

### Wire 798 body structure (TestCase — the richest event)

Two forms based on flags byte:

**Form A — class-level** (flags = 0x00):
```
[flags:1B] [executor_id:8B] [class_id:8B] [class_name:kryo_str] [short_class_name:kryo_str]
```

**Form B — method-level** (flags = 0x05):
```
[flags:1B] [executor_id:8B] [method_id:8B] [method_name:kryo_str] [class_name:kryo_str] [executor_name:kryo_str]
```

### Wire 284 body structure (TestResult)

```
[flags:1B=0x1c] [executor_id:8B] [result_data:8B]
```

The 9 occurrences match the 5 test methods (4 in LinkedListTest + 1 in MessageUtilsTest) plus class-level and suite-level events.

### Wire 127 body structure (TestExecutorIdentity)

```
[flags:1B=0x34] [executor_id:8B] [hash:8B] [name:kryo_str="Gradle Test Executor N"]
```

## Architecture: Tests as a Peer to Tasks

Following the established Task pattern exactly:

### Parser layer (`build-scan/lib/`)

**New event decoders** in `events/`:
- `test_executor_identity.rs` (wire 127)
- `test_executor_started.rs` (wire 128)
- `test_case.rs` (wire 798)
- `test_result.rs` (wire 284)
- `test_executor_finished.rs` (wire 803)

Skip wire 123 (redundant confirmation) and 548 (classpath resolution, not test-specific).

**New model** in `models.rs`:
```rust
pub struct TestCase {
    pub class_name: String,
    pub method_name: Option<String>,  // None for class-level events
    pub executor_name: String,
    pub outcome: TestOutcome,
}

pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}
```

**Assembly** in `assembly.rs`:
- Correlate TestCase events by executor_id
- Match TestResult events to TestCase events by position/executor
- Populate `BuildScanPayload.tests: Vec<TestCase>`

### Domain layer (`build-scan/server/domain/`)

**New domain type**:
```rust
pub struct Test {
    pub id: TestId,
    pub scan_id: ScanId,
    pub class_name: ClassName,
    pub method_name: Option<MethodName>,
    pub executor_name: ExecutorName,
    pub outcome: TestOutcome,
}
```

### Database layer (`build-scan/server/db/`)

**New table** `tests`:
```sql
CREATE TABLE tests (
    id TEXT PRIMARY KEY,
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    class_name TEXT NOT NULL,
    method_name TEXT,
    executor_name TEXT NOT NULL,
    outcome TEXT NOT NULL
);
```

### GraphQL layer (`build-scan/server/graphql/`)

**New type** on `BuildScan`:
```graphql
type Test {
    id: ID!
    className: String!
    methodName: String
    executorName: String!
    outcome: TestOutcome!
}

enum TestOutcome {
    PASSED
    FAILED
    SKIPPED
}

extend type BuildScan {
    tests: [Test!]!
}
```

## Out of Scope

- Wire 548 (plugin JAR resolution) — not test-specific
- Wire 123 (test executor confirmation) — redundant with 128
- Cache-hit events (144, 145, 303, 304) — separate concern
- Test output/stdout/stderr — not present in current payload
- Pagination on tests query — add later if needed

## Test Data

Reference payload: scan ID `5755d086-24ee-4b00-9816-0a369593ee92` in `build-scans.db`, captured from Gradle 9.4.0 with Develocity plugin 4.3.2. Contains 5 JUnit 5 test methods across 2 test classes executed by 2 parallel test executors.
