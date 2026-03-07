# Test Event Decoders Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Decode Gradle test execution events from build scan payloads and expose them through GraphQL.

**Architecture:** Add 5 event decoders for test lifecycle wire IDs (127, 128, 284, 798, 803), assemble them into a `TestCase` model, persist via a new `tests` table, and expose through GraphQL as a peer to `Task`.

**Tech Stack:** Rust, Bazel, SQLx (SQLite), Juniper (GraphQL), Kryo binary format

---

### Task 1: Add TestCase event decoder (wire 798)

This is the richest test event — contains class names and method names.

**Files:**
- Create: `build-scan/lib/src/events/test_case.rs`
- Modify: `build-scan/lib/src/events/mod.rs`
- Modify: `build-scan/lib/src/events/BUILD.bazel`

**Step 1: Write the failing test**

Add to `build-scan/lib/src/events/test_case.rs`:

```rust
use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestCaseEvent};

pub struct TestCaseDecoder;

impl BodyDecoder for TestCaseDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_method_level_event() {
        // Wire 798 method-level: flags=0x05, executor_id(8B), method_id(8B),
        // method_name(kryo_str), class_name(kryo_str), executor_name(kryo_str)
        // Use real bytes extracted from captured payload for a testAdd() method
        let decoder = TestCaseDecoder;
        // We'll fill in real bytes after hex-dumping wire 798 from the captured payload
        // For now, construct a synthetic payload following the Kryo pattern
        let mut data = vec![0x05]; // flags: bits 0 and 2 set
        data.extend_from_slice(&42i64.to_le_bytes()); // executor_id
        data.extend_from_slice(&99i64.to_le_bytes()); // method_id
        // method_name "testAdd()" → zigzag(9)=18
        data.push(0x12);
        data.extend_from_slice(b"testAdd()");
        // class_name "org.example.list.LinkedListTest" → zigzag(31)=62
        data.push(0x3e);
        data.extend_from_slice(b"org.example.list.LinkedListTest");
        // executor_name "Gradle Test Executor 1" → zigzag(22)=44
        data.push(0x2c);
        data.extend_from_slice(b"Gradle Test Executor 1");

        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.class_name, "org.example.list.LinkedListTest");
            assert_eq!(e.method_name, Some("testAdd()".to_string()));
            assert_eq!(e.executor_name, Some("Gradle Test Executor 1".to_string()));
        } else {
            panic!("expected TestCase, got {:?}", result);
        }
    }

    #[test]
    fn test_decode_class_level_event() {
        // Wire 798 class-level: flags=0x00
        let mut data = vec![0x00]; // flags: all present
        data.extend_from_slice(&42i64.to_le_bytes()); // executor_id
        data.extend_from_slice(&88i64.to_le_bytes()); // class_id
        // class_name "org.example.list.LinkedListTest"
        data.push(0x3e);
        data.extend_from_slice(b"org.example.list.LinkedListTest");
        // short_class_name "LinkedListTest"
        data.push(0x1c);
        data.extend_from_slice(b"LinkedListTest");

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.class_name, "org.example.list.LinkedListTest");
            assert_eq!(e.method_name, None);
        } else {
            panic!("expected TestCase, got {:?}", result);
        }
    }
}
```

**Step 2: Add event struct and enum variant to `mod.rs`**

Add to `events/mod.rs` after existing event structs:

```rust
pub mod test_case;

// In DecodedEvent enum:
TestCase(TestCaseEvent),

// New struct:
#[derive(Debug, Clone)]
pub struct TestCaseEvent {
    pub executor_id: i64,
    pub class_name: String,
    pub method_name: Option<String>,
    pub executor_name: Option<String>,
}
```

**Step 3: Register decoder in `DecoderRegistry::new()`**

```rust
registry.register(798, Box::new(test_case::TestCaseDecoder));
```

**Step 4: Add `test_case.rs` to `BUILD.bazel` srcs list**

Add `"test_case.rs"` to the srcs list in `build-scan/lib/src/events/BUILD.bazel`.

**Step 5: Implement the decoder**

```rust
impl BodyDecoder for TestCaseDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let _secondary_id = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let name1 = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let name2 = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let name3 = if kryo::is_field_present(flags as u16, 4) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        // Two forms based on flags:
        // Method-level (flags=0x05): name1=method_name, name2=class_name, name3=executor_name
        // Class-level (flags=0x00): name1=class_name, name2=short_class_name, name3=None
        let is_method_level = name3.is_some();

        let (class_name, method_name, executor_name) = if is_method_level {
            (name2.unwrap_or_default(), name1, name3)
        } else {
            (name1.unwrap_or_default(), None, None)
        };

        Ok(DecodedEvent::TestCase(TestCaseEvent {
            executor_id,
            class_name,
            method_name,
            executor_name,
        }))
    }
}
```

**Step 6: Run tests to verify**

Run: `bazel test //build-scan/lib/src/events:events_test`
Expected: PASS (both test_decode_method_level_event and test_decode_class_level_event)

**Step 7: Commit**

```bash
git add build-scan/lib/src/events/test_case.rs build-scan/lib/src/events/mod.rs build-scan/lib/src/events/BUILD.bazel
git commit -m "feat: add TestCase event decoder (wire 798)"
```

---

### Task 2: Add TestExecutorIdentity decoder (wire 127)

**Files:**
- Create: `build-scan/lib/src/events/test_executor_identity.rs`
- Modify: `build-scan/lib/src/events/mod.rs`
- Modify: `build-scan/lib/src/events/BUILD.bazel`

**Step 1: Write the failing test**

```rust
use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestExecutorIdentityEvent};

pub struct TestExecutorIdentityDecoder;

impl BodyDecoder for TestExecutorIdentityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_executor_identity() {
        // Wire 127: flags(1B) + executor_id(8B) + hash(8B) + name(kryo_str)
        let mut data = vec![0x00]; // flags: all present
        data.extend_from_slice(&1i64.to_le_bytes()); // executor_id
        data.extend_from_slice(&2i64.to_le_bytes()); // hash
        // name "Gradle Test Executor 1"
        data.push(0x2c); // zigzag(22)=44
        data.extend_from_slice(b"Gradle Test Executor 1");

        let decoder = TestExecutorIdentityDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorIdentity(e) = result {
            assert_eq!(e.executor_id, 1);
            assert_eq!(e.name, "Gradle Test Executor 1");
        } else {
            panic!("expected TestExecutorIdentity, got {:?}", result);
        }
    }
}
```

**Step 2: Add event struct and variant**

In `mod.rs`:

```rust
pub mod test_executor_identity;

// In DecodedEvent enum:
TestExecutorIdentity(TestExecutorIdentityEvent),

// New struct:
#[derive(Debug, Clone)]
pub struct TestExecutorIdentityEvent {
    pub executor_id: i64,
    pub name: String,
}
```

**Step 3: Register in `DecoderRegistry::new()`**

```rust
registry.register(127, Box::new(test_executor_identity::TestExecutorIdentityDecoder));
```

**Step 4: Implement**

```rust
impl BodyDecoder for TestExecutorIdentityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        // Skip hash field (8 bytes)
        if kryo::is_field_present(flags as u16, 1) {
            let _ = kryo::read_task_id(body, &mut pos)?;
        }

        let name = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        Ok(DecodedEvent::TestExecutorIdentity(TestExecutorIdentityEvent {
            executor_id,
            name,
        }))
    }
}
```

**Step 5: Update BUILD.bazel, run tests, commit**

Add `"test_executor_identity.rs"` to srcs in `BUILD.bazel`.

Run: `bazel test //build-scan/lib/src/events:events_test`

```bash
git add build-scan/lib/src/events/test_executor_identity.rs build-scan/lib/src/events/mod.rs build-scan/lib/src/events/BUILD.bazel
git commit -m "feat: add TestExecutorIdentity event decoder (wire 127)"
```

---

### Task 3: Add TestResult decoder (wire 284)

**Files:**
- Create: `build-scan/lib/src/events/test_result.rs`
- Modify: `build-scan/lib/src/events/mod.rs`
- Modify: `build-scan/lib/src/events/BUILD.bazel`

**Step 1: Write the failing test**

```rust
use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestResultEvent};

pub struct TestResultDecoder;

impl BodyDecoder for TestResultDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_test_result() {
        // Wire 284: flags(1B) + executor_id(8B) + result_data(8B)
        let mut data = vec![0x00]; // flags: all present
        data.extend_from_slice(&42i64.to_le_bytes()); // executor_id
        data.extend_from_slice(&0i64.to_le_bytes()); // result_data (0 = passed)

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, 42);
            assert_eq!(e.result_ordinal, Some(0));
        } else {
            panic!("expected TestResult, got {:?}", result);
        }
    }
}
```

**Step 2: Add event struct and variant**

In `mod.rs`:

```rust
pub mod test_result;

// In DecodedEvent enum:
TestResult(TestResultEvent),

// New struct:
#[derive(Debug, Clone)]
pub struct TestResultEvent {
    pub executor_id: i64,
    pub result_ordinal: Option<u64>,
}
```

**Step 3: Register + implement + BUILD + test + commit**

Register: `registry.register(284, Box::new(test_result::TestResultDecoder));`

```rust
impl BodyDecoder for TestResultDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let result_ordinal = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_task_id(body, &mut pos)? as u64)
        } else {
            None
        };

        Ok(DecodedEvent::TestResult(TestResultEvent {
            executor_id,
            result_ordinal,
        }))
    }
}
```

Add `"test_result.rs"` to BUILD.bazel srcs.

Run: `bazel test //build-scan/lib/src/events:events_test`

```bash
git add build-scan/lib/src/events/test_result.rs build-scan/lib/src/events/mod.rs build-scan/lib/src/events/BUILD.bazel
git commit -m "feat: add TestResult event decoder (wire 284)"
```

---

### Task 4: Add TestExecutorStarted (wire 128) and TestExecutorFinished (wire 803) decoders

These are simple confirmation events. Bundle them together.

**Files:**
- Create: `build-scan/lib/src/events/test_executor_started.rs`
- Create: `build-scan/lib/src/events/test_executor_finished.rs`
- Modify: `build-scan/lib/src/events/mod.rs`
- Modify: `build-scan/lib/src/events/BUILD.bazel`

**Step 1: Write both decoders with tests**

`test_executor_started.rs`:
```rust
use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestExecutorStartedEvent};

pub struct TestExecutorStartedDecoder;

impl BodyDecoder for TestExecutorStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        Ok(DecodedEvent::TestExecutorStarted(TestExecutorStartedEvent {
            executor_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_executor_started() {
        let mut data = vec![0x00];
        data.extend_from_slice(&7i64.to_le_bytes());
        // Skip remaining bytes (hash)
        data.extend_from_slice(&0i64.to_le_bytes());

        let decoder = TestExecutorStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorStarted(e) = result {
            assert_eq!(e.executor_id, 7);
        } else {
            panic!("expected TestExecutorStarted");
        }
    }
}
```

`test_executor_finished.rs`:
```rust
use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestExecutorFinishedEvent};

pub struct TestExecutorFinishedDecoder;

impl BodyDecoder for TestExecutorFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        Ok(DecodedEvent::TestExecutorFinished(TestExecutorFinishedEvent {
            executor_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_executor_finished() {
        let mut data = vec![0x00];
        data.extend_from_slice(&7i64.to_le_bytes());
        // Remaining bytes (sentinel)
        data.extend_from_slice(&[0xFF; 9]);

        let decoder = TestExecutorFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorFinished(e) = result {
            assert_eq!(e.executor_id, 7);
        } else {
            panic!("expected TestExecutorFinished");
        }
    }
}
```

**Step 2: Add structs and variants in `mod.rs`**

```rust
pub mod test_executor_started;
pub mod test_executor_finished;

// In DecodedEvent enum:
TestExecutorStarted(TestExecutorStartedEvent),
TestExecutorFinished(TestExecutorFinishedEvent),

// New structs:
#[derive(Debug, Clone)]
pub struct TestExecutorStartedEvent {
    pub executor_id: i64,
}

#[derive(Debug, Clone)]
pub struct TestExecutorFinishedEvent {
    pub executor_id: i64,
}
```

**Step 3: Register both, update BUILD, test, commit**

```rust
registry.register(128, Box::new(test_executor_started::TestExecutorStartedDecoder));
registry.register(803, Box::new(test_executor_finished::TestExecutorFinishedDecoder));
```

Add both `.rs` files to BUILD.bazel srcs.

Run: `bazel test //build-scan/lib/src/events:events_test`

```bash
git add build-scan/lib/src/events/test_executor_started.rs build-scan/lib/src/events/test_executor_finished.rs build-scan/lib/src/events/mod.rs build-scan/lib/src/events/BUILD.bazel
git commit -m "feat: add TestExecutorStarted/Finished decoders (wires 128, 803)"
```

---

### Task 5: Add TestCase to parser models and assembly

**Files:**
- Modify: `build-scan/lib/src/models.rs`
- Modify: `build-scan/lib/src/assembly.rs`

**Step 1: Add TestCase model to `models.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub class_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TestOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}
```

Add `tests: Vec<TestCase>` to `BuildScanPayload`:

```rust
#[serde(skip_serializing_if = "Vec::is_empty")]
pub tests: Vec<TestCase>,
```

**Step 2: Update assembly to collect test events**

In `assembly.rs`, add collection logic:

```rust
// New collections at top of assemble():
let mut test_cases: Vec<models::TestCase> = Vec::new();
let mut executor_names: HashMap<i64, String> = HashMap::new();

// In the match block:
DecodedEvent::TestExecutorIdentity(e) => {
    executor_names.insert(e.executor_id, e.name.clone());
}
DecodedEvent::TestCase(e) => {
    if e.method_name.is_some() {
        let executor_name = e.executor_name.clone()
            .or_else(|| executor_names.get(&e.executor_id).cloned());
        test_cases.push(models::TestCase {
            class_name: e.class_name.clone(),
            method_name: e.method_name.clone(),
            executor_name,
            outcome: None, // Populated later by TestResult correlation
        });
    }
}
// Events decoded for coverage but not yet consumed:
DecodedEvent::TestExecutorStarted(_) => {}
DecodedEvent::TestExecutorFinished(_) => {}
DecodedEvent::TestResult(_) => {}
```

Add `tests: test_cases` to the `BuildScanPayload` construction.

**Step 3: Write assembly test**

In `assembly.rs` tests module:

```rust
#[test]
fn test_assemble_test_cases() {
    let events = vec![
        (
            frame(127, 1000),
            DecodedEvent::TestExecutorIdentity(TestExecutorIdentityEvent {
                executor_id: 42,
                name: "Gradle Test Executor 1".into(),
            }),
        ),
        (
            frame(798, 2000),
            DecodedEvent::TestCase(TestCaseEvent {
                executor_id: 42,
                class_name: "org.example.list.LinkedListTest".into(),
                method_name: Some("testAdd()".into()),
                executor_name: Some("Gradle Test Executor 1".into()),
            }),
        ),
        (
            frame(798, 2001),
            DecodedEvent::TestCase(TestCaseEvent {
                executor_id: 42,
                class_name: "org.example.list.LinkedListTest".into(),
                method_name: None,
                executor_name: None,
            }),
        ),
    ];
    let payload = assemble(events);
    // Only method-level events become test cases
    assert_eq!(payload.tests.len(), 1);
    assert_eq!(payload.tests[0].class_name, "org.example.list.LinkedListTest");
    assert_eq!(payload.tests[0].method_name.as_deref(), Some("testAdd()"));
    assert_eq!(payload.tests[0].executor_name.as_deref(), Some("Gradle Test Executor 1"));
}
```

**Step 4: Run tests + commit**

Run: `bazel test //build-scan/lib/src:assembly_test`

```bash
git add build-scan/lib/src/models.rs build-scan/lib/src/assembly.rs
git commit -m "feat: assemble TestCase events into BuildScanPayload"
```

---

### Task 6: Validate against real payload

**Step 1: Run the integration test against the captured payload**

Run: `bazel test //build-scan/lib/src:integration_test`

If the test passes, the decoders are correctly parsing the real payload without errors. Check that the `raw_events` count for wire IDs 127, 128, 284, 798, 803 dropped to 0 (they're now decoded instead of raw).

**Step 2: Run CLI against the captured payload to inspect test output**

```bash
sqlite3 build-scans.db "SELECT writefile('/tmp/test_payload.bin', raw_payload) FROM build_scans WHERE id='5755d086-24ee-4b00-9816-0a369593ee92';"
bazel run //cli/src:main -- /tmp/test_payload.bin 2>/dev/null | jq '.tests'
```

Expected: JSON array of test cases with class_name, method_name, executor_name.

**Step 3: Commit if any fixes were needed**

---

### Task 7: Add domain type for Test

**Files:**
- Modify: `build-scan/server/domain/src/lib.rs`

**Step 1: Add types**

```rust
// New ID type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestId(pub Uuid);

impl From<Uuid> for TestId {
    fn from(v: Uuid) -> Self {
        Self(v)
    }
}

// New string-wrapped types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestClassName(pub String);

impl From<String> for TestClassName {
    fn from(v: String) -> Self {
        Self(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodName(pub String);

impl From<String> for MethodName {
    fn from(v: String) -> Self {
        Self(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutorName(pub String);

impl From<String> for ExecutorName {
    fn from(v: String) -> Self {
        Self(v)
    }
}

// New enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestOutcome::Passed => write!(f, "Passed"),
            TestOutcome::Failed => write!(f, "Failed"),
            TestOutcome::Skipped => write!(f, "Skipped"),
        }
    }
}

impl std::str::FromStr for TestOutcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Passed" => Ok(TestOutcome::Passed),
            "Failed" => Ok(TestOutcome::Failed),
            "Skipped" => Ok(TestOutcome::Skipped),
            other => Err(format!("unknown test outcome: '{other}'")),
        }
    }
}

// New aggregate
#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct Test {
    pub id: TestId,
    pub scan_id: BuildScanId,
    pub class_name: TestClassName,
    #[builder(setter(strip_option), default)]
    pub method_name: Option<MethodName>,
    #[builder(setter(strip_option), default)]
    pub executor_name: Option<ExecutorName>,
    #[builder(setter(strip_option), default)]
    pub outcome: Option<TestOutcome>,
}
```

**Step 2: Build to verify**

Run: `bazel build //build-scan/server/domain/src:domain`

**Step 3: Commit**

```bash
git add build-scan/server/domain/src/lib.rs
git commit -m "feat: add Test domain type with TestOutcome enum"
```

---

### Task 8: Add DB migration and queries for tests

**Files:**
- Create: `build-scan/server/db/migrations/src/sql/003_tests_table.sql`
- Modify: `build-scan/server/db/src/lib.rs`

**Step 1: Create migration**

```sql
CREATE TABLE IF NOT EXISTS tests (
    id TEXT PRIMARY KEY,
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    class_name TEXT NOT NULL,
    method_name TEXT,
    executor_name TEXT,
    outcome TEXT
);

CREATE INDEX IF NOT EXISTS idx_tests_scan_id ON tests(scan_id);
```

**Step 2: Add TestRow and conversions in `db/src/lib.rs`**

```rust
#[derive(Debug, sqlx::FromRow)]
struct TestRow {
    id: String,
    scan_id: String,
    class_name: String,
    method_name: Option<String>,
    executor_name: Option<String>,
    outcome: Option<String>,
}

impl TryFrom<TestRow> for domain::Test {
    type Error = anyhow::Error;

    fn try_from(row: TestRow) -> Result<Self, Self::Error> {
        let id = domain::TestId(
            uuid::Uuid::parse_str(&row.id)
                .with_context(|| format!("invalid test id '{}'", row.id))?,
        );
        let scan_id = domain::BuildScanId(
            uuid::Uuid::parse_str(&row.scan_id)
                .with_context(|| format!("invalid scan_id '{}'", row.scan_id))?,
        );
        let outcome = row
            .outcome
            .map(|s| s.parse::<domain::TestOutcome>().map_err(anyhow::Error::msg))
            .transpose()?;

        Ok(domain::Test {
            id,
            scan_id,
            class_name: domain::TestClassName(row.class_name),
            method_name: row.method_name.map(domain::MethodName),
            executor_name: row.executor_name.map(domain::ExecutorName),
            outcome,
        })
    }
}

impl From<&domain::Test> for TestRow {
    fn from(test: &domain::Test) -> Self {
        Self {
            id: test.id.0.to_string(),
            scan_id: test.scan_id.0.to_string(),
            class_name: test.class_name.0.clone(),
            method_name: test.method_name.as_ref().map(|m| m.0.clone()),
            executor_name: test.executor_name.as_ref().map(|e| e.0.clone()),
            outcome: test.outcome.map(|o| o.to_string()),
        }
    }
}
```

**Step 3: Add CRUD functions**

```rust
pub async fn insert_test<'c, E: sqlx::Executor<'c, Database = sqlx::Sqlite>>(
    executor: E,
    test: &domain::Test,
) -> Result<()> {
    let row = TestRow::from(test);
    sqlx::query(
        "INSERT INTO tests (id, scan_id, class_name, method_name, executor_name, outcome) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.scan_id)
    .bind(&row.class_name)
    .bind(row.method_name.as_deref())
    .bind(row.executor_name.as_deref())
    .bind(row.outcome.as_deref())
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_tests(
    pool: &SqlitePool,
    scan_id: &str,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<domain::Test>> {
    let rows = if let Some(cursor) = after_id {
        sqlx::query_as::<_, TestRow>(
            "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
             FROM tests WHERE scan_id = ? AND id > ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, TestRow>(
            "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
             FROM tests WHERE scan_id = ? ORDER BY id LIMIT ?",
        )
        .bind(scan_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    rows.into_iter()
        .map(domain::Test::try_from)
        .collect::<Result<Vec<_>>>()
}

pub async fn count_tests(pool: &SqlitePool, scan_id: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tests WHERE scan_id = ?")
        .bind(scan_id)
        .fetch_one(pool)
        .await?;

    Ok(count)
}

pub async fn get_test(pool: &SqlitePool, id: &str) -> Result<Option<domain::Test>> {
    let row = sqlx::query_as::<_, TestRow>(
        "SELECT id, scan_id, class_name, method_name, executor_name, outcome \
         FROM tests WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(domain::Test::try_from).transpose()
}
```

**Step 4: Build to verify**

Run: `bazel build //build-scan/server/db/src:db`

**Step 5: Commit**

```bash
git add build-scan/server/db/migrations/src/sql/003_tests_table.sql build-scan/server/db/src/lib.rs
git commit -m "feat: add tests table migration and DB queries"
```

---

### Task 9: Wire Test through service layer

**Files:**
- Modify: `build-scan/server/service/src/lib.rs`

**Step 1: Add test persistence in `process_upload`**

After the task insertion loop, add:

```rust
let test_count = payload.tests.len();
for test in &payload.tests {
    let mut test_builder = domain::TestBuilder::default();
    test_builder
        .id(Uuid::new_v4())
        .scan_id(scan_uuid)
        .class_name(test.class_name.clone());

    if let Some(mn) = &test.method_name {
        test_builder.method_name(mn.clone());
    }
    if let Some(en) = &test.executor_name {
        test_builder.executor_name(en.clone());
    }

    let domain_test = test_builder
        .build()
        .map_err(|e| anyhow::anyhow!(e))
        .context("failed to build Test")?;

    db::insert_test(&mut *tx, &domain_test)
        .await
        .with_context(|| format!("failed to store test {}", test.class_name))?;
}
```

Update log line:
```rust
info!(scan_id = %scan_id, task_count = task_count, test_count = test_count, "Stored build scan successfully");
```

**Step 2: Add query methods**

```rust
pub async fn list_tests(
    &self,
    scan_id: &str,
    limit: i64,
    after_id: Option<&str>,
) -> Result<Vec<domain::Test>> {
    db::list_tests(&self.pool, scan_id, limit, after_id).await
}

pub async fn count_tests(&self, scan_id: &str) -> Result<i64> {
    db::count_tests(&self.pool, scan_id).await
}

pub async fn get_test(&self, id: &str) -> Result<Option<domain::Test>> {
    db::get_test(&self.pool, id).await
}
```

**Step 3: Build + commit**

Run: `bazel build //build-scan/server/service/src:service`

```bash
git add build-scan/server/service/src/lib.rs
git commit -m "feat: wire Test persistence through service layer"
```

---

### Task 10: Add Test to GraphQL schema

**Files:**
- Modify: `build-scan/server/graphql/src/lib.rs`

**Step 1: Add Test GraphQL type**

```rust
// Update Node interface
#[derive(GraphQLInterface)]
#[graphql(for = [BuildScan, Task, Test], context = Context)]
pub struct Node {
    pub id: ID,
}

// New Test type
pub struct Test {
    pub test: domain::Test,
}

#[graphql_object(context = Context, impl = NodeValue)]
impl Test {
    fn id(&self) -> ID {
        RelayId::encode("Test", &self.test.id.0.to_string())
    }

    fn test_id(&self) -> String {
        self.test.id.0.to_string()
    }

    fn scan_id(&self) -> String {
        self.test.scan_id.0.to_string()
    }

    fn class_name(&self) -> &str {
        &self.test.class_name.0
    }

    fn method_name(&self) -> Option<&str> {
        self.test.method_name.as_ref().map(|m| m.0.as_str())
    }

    fn executor_name(&self) -> Option<&str> {
        self.test.executor_name.as_ref().map(|e| e.0.as_str())
    }

    fn outcome(&self) -> Option<String> {
        self.test.outcome.map(|o| o.to_string())
    }
}
```

**Step 2: Add TestConnection / TestEdge (rename existing ones first)**

The existing `TaskConnection`/`TaskEdge` will conflict. Rename them or create test-specific versions:

```rust
pub struct TestCaseEdge {
    pub cursor: String,
    pub node: Test,
}

#[graphql_object(context = Context)]
impl TestCaseEdge {
    fn cursor(&self) -> &str {
        &self.cursor
    }

    fn node(&self) -> &Test {
        &self.node
    }
}

pub struct TestCaseConnection {
    pub edges: Vec<TestCaseEdge>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

#[graphql_object(context = Context)]
impl TestCaseConnection {
    fn edges(&self) -> &Vec<TestCaseEdge> {
        &self.edges
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    fn total_count(&self) -> i32 {
        self.total_count
    }
}
```

**Step 3: Add `tests` field to BuildScan**

Add to the `BuildScan` graphql_object impl:

```rust
async fn test_count(&self, context: &Context) -> FieldResult<i32> {
    let count = context
        .service
        .count_tests(&self.scan.id.0.to_string())
        .await
        .map_err(|e| FieldError::from(e.to_string()))? as i32;
    Ok(count)
}

async fn tests(
    &self,
    context: &Context,
    first: Option<i32>,
    after: Option<String>,
) -> FieldResult<TestCaseConnection> {
    let limit = validate_pagination(first).map_err(|e| FieldError::from(e.to_string()))?;
    let after_id = after
        .as_deref()
        .map(Cursor::decode)
        .transpose()
        .map_err(|e| FieldError::from(e.to_string()))?
        .map(|c| c.value);

    let scan_id_str = self.scan.id.0.to_string();
    let mut tests = context
        .service
        .list_tests(&scan_id_str, (limit + 1) as i64, after_id.as_deref())
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;

    let has_next_page = tests.len() > limit as usize;
    if has_next_page {
        tests.pop();
    }

    let end_cursor = tests
        .last()
        .map(|t| Cursor::new(t.id.0.to_string()).encode());

    let edges: Vec<TestCaseEdge> = tests
        .into_iter()
        .map(|t| {
            let cursor = Cursor::new(t.id.0.to_string()).encode();
            TestCaseEdge {
                cursor,
                node: Test { test: t },
            }
        })
        .collect();

    let total_count = context
        .service
        .count_tests(&scan_id_str)
        .await
        .map_err(|e| FieldError::from(e.to_string()))? as i32;

    Ok(TestCaseConnection {
        edges,
        page_info: PageInfo {
            has_next_page,
            end_cursor,
        },
        total_count,
    })
}
```

**Step 4: Add Test to node resolver**

In `QueryRoot::node()`:

```rust
"Test" => {
    let test = context
        .service
        .get_test(&relay_id.raw_id)
        .await
        .map_err(|e| FieldError::from(e.to_string()))?;
    Ok(test.map(|t| NodeValue::Test(Test { test: t })))
}
```

**Step 5: Build + commit**

Run: `bazel build //build-scan/server/graphql/src:graphql`

```bash
git add build-scan/server/graphql/src/lib.rs
git commit -m "feat: expose Test type in GraphQL schema with pagination"
```

---

### Task 11: Run gazelle, format, full test suite

**Step 1: Run gazelle**

```bash
bazel run gazelle
```

**Step 2: Format**

```bash
bazel run //tools/format
```

**Step 3: Run full test suite**

```bash
bazel test //...
```

**Step 4: Build full project**

```bash
bazel build //...
```

**Step 5: Commit any formatting changes**

```bash
git add -A && git commit -m "chore: run gazelle and format"
```

---

### Task 12: End-to-end validation

**Step 1: Start the server, run the Gradle build, query GraphQL**

```bash
rm -f build-scans.db
bazel-bin/build-scan/server/src/main &
sleep 2
cd gradle && DEVELOCITY_SERVER_URL=http://localhost:3000 ./gradlew clean test --no-build-cache --no-configuration-cache
```

**Step 2: Query tests via GraphQL**

```bash
curl -s http://localhost:3000/graphql -X POST \
  -H "Content-Type: application/json" \
  -d '{"query":"{ buildScans(first: 1) { edges { node { scanId tests(first: 20) { totalCount edges { node { className methodName executorName outcome } } } } } } }"}' | jq .
```

Expected: Test cases with class names like `org.example.list.LinkedListTest` and method names like `testAdd()`, `testRemove()`, etc.

**Step 3: Stop the server**

```bash
kill %1
```

---

### Task 13: Create PR

```bash
gh pr create --title "feat: decode test events from Gradle build scans" --body "$(cat <<'EOF'
## Summary
- Adds 5 event decoders for the Gradle test lifecycle (wire IDs 127, 128, 284, 798, 803)
- Assembles test events into `TestCase` model alongside existing Task model
- Persists tests to new `tests` SQLite table with migration
- Exposes tests through GraphQL with Relay pagination on `BuildScan.tests`

## Wire IDs decoded
| Wire ID | Event | Description |
|---------|-------|-------------|
| 127 | TestExecutorIdentity | Executor name + ID |
| 128 | TestExecutorStarted | Executor started |
| 284 | TestResult | Per-test outcome |
| 798 | TestCase | Class + method names |
| 803 | TestExecutorFinished | Executor finished |

## Test plan
- [ ] Unit tests for each decoder pass
- [ ] Assembly test validates test case correlation
- [ ] Integration test passes against real captured payload
- [ ] Full `bazel test //...` passes
- [ ] End-to-end: Gradle build → server → GraphQL query returns test data
EOF
)"
```
