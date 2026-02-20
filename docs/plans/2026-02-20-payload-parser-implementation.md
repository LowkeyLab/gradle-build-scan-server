# Gradle Build Scan Payload Parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a fast, in-memory two-pass decoder in Rust to parse schema-less Gradle Build Scan binary payloads into strongly-typed objects.

**Architecture:** A `primitives` module will provide a `StreamDecoder` to uncompress the gzip stream and yield intermediate `Primitive` types (varints, strings, timestamps). A `parser` module will contain a `PayloadBuilder` state machine that consumes these primitives, maintains an interned string dictionary, and constructs JSON-serializable output objects defined in the `models` module. It uses strict fail-fast error handling via `thiserror`.

**Tech Stack:** Rust (Edition 2024), `thiserror` (to be added), `flate2` (to be added), `chrono`, `serde`.

---

### Task 1: Setup Dependencies and Error Types

**Files:**
- Modify: `Cargo.toml`
- Create: `src/error.rs`
- Modify: `src/main.rs`
- Modify: `BUILD.bazel`

**Step 1: Add dependencies to Cargo.toml**

Add `thiserror = "2.0"` and `flate2 = "1.0"` to the `[dependencies]` section of `Cargo.toml`. Also ensure `chrono` and `serde` are present.

**Step 2: Create error.rs**

Create `src/error.rs` with the `ParseError` enum as designed:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to decompress Gzip stream")]
    InvalidGzip,
    #[error("Malformed LEB128 varint encountered at offset {offset}")]
    MalformedLeb128 { offset: usize },
    #[error("Unexpected primitive type: expected {expected}")]
    UnexpectedPrimitive { expected: &'static str },
    #[error("Unknown or unhandled event schema ID: {0}")]
    UnknownEventSchema(u32),
    #[error("Invalid string reference: index {0} not found in dictionary")]
    InvalidStringRef(u32),
    #[error("Unexpected End Of File")]
    UnexpectedEof,
}
```

**Step 3: Expose error module**

In `src/main.rs`, add `pub mod error;`

**Step 4: Update Bazel Build file**

Run `bazel run gazelle` to update `BUILD.bazel` with the new dependencies (note: you might need to manually add them to `MODULE.bazel` or `Cargo.toml` depending on how rules_rust is configured here).

**Step 5: Run `cargo check` / `bazel build`**

Run `cargo check` to ensure the error types compile.

**Step 6: Commit**

```bash
git add Cargo.toml src/error.rs src/main.rs BUILD.bazel
git commit -m "feat: add payload parser error types and dependencies"
```

---

### Task 2: Implement Primitives and StreamDecoder

**Files:**
- Create: `src/primitives.rs`
- Modify: `src/main.rs`

**Step 1: Define the `Primitive` enum in `src/primitives.rs`**

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq, Clone)]
pub enum Primitive {
    Varint(u64),
    String(String),
    StringRef(u32),
    Timestamp(DateTime<Utc>),
}
```

**Step 2: Implement LEB128 decoding**

In `src/primitives.rs`, add a helper function or method on a `StreamDecoder` struct to read LEB128 varints from a byte slice cursor.

**Step 3: Implement StreamDecoder**

Create `StreamDecoder<'a>` wrapping a `&'a [u8]` cursor. It needs to:
1. Validate the 28-byte cleartext header (or skip to the gzip magic bytes `1f 8b 08`).
2. Decompress the gzip stream into a `Vec<u8>`.
3. Provide an iterator or `next()` method yielding `Result<Primitive, ParseError>`.
4. Apply heuristic: if `varint >> 1` is a valid string length (e.g. 2-500) and `varint & 1 == 0`, read bytes as UTF-8 `Primitive::String`. If `varint & 1 == 1`, emit `Primitive::StringRef((varint >> 1) as u32)`.

**Step 4: Write Tests for StreamDecoder**

Add a `#[cfg(test)]` mod in `src/primitives.rs` to test varint decoding and string primitive emitting.

**Step 5: Run Tests**

Run `cargo test` to ensure primitives decode correctly.

**Step 6: Commit**

```bash
git add src/primitives.rs src/main.rs
git commit -m "feat: implement LEB128 stream decoder and primitives"
```

---

### Task 3: Define Output Models

**Files:**
- Create: `src/models.rs`
- Modify: `src/main.rs`

**Step 1: Create `src/models.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub tasks: Vec<TaskExecution>,
    // Expand as needed
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskExecution {
    pub task_path: String,
    // Add fields as discovered by heuristics
}
```

**Step 2: Expose models module**

In `src/main.rs`, add `pub mod models;`

**Step 3: Run `cargo check`**

Verify models compile.

**Step 4: Commit**

```bash
git add src/models.rs src/main.rs
git commit -m "feat: define strongly-typed output models"
```

---

### Task 4: Implement PayloadBuilder

**Files:**
- Create: `src/parser.rs`
- Modify: `src/main.rs`

**Step 1: Create PayloadBuilder in `src/parser.rs`**

```rust
use crate::error::ParseError;
use crate::models::BuildScanPayload;
use crate::primitives::{Primitive, StreamDecoder};

pub struct PayloadBuilder {
    dictionary: Vec<String>,
}

impl PayloadBuilder {
    pub fn new() -> Self {
        Self { dictionary: Vec::new() }
    }

    pub fn build(&mut self, data: &[u8]) -> Result<BuildScanPayload, ParseError> {
        let mut decoder = StreamDecoder::new(data)?;
        let mut payload = BuildScanPayload::default();

        while let Some(prim) = decoder.next()? {
            match prim {
                Primitive::String(s) => {
                    self.dictionary.push(s);
                }
                // Implement state machine logic here based on heuristics
                // For example, if we hit a known Event ID varint, consume next N primitives
                _ => {} 
            }
        }
        Ok(payload)
    }
}
```

**Step 2: Expose parser module**

In `src/main.rs`, add `pub mod parser;`

**Step 3: Add integration tests**

Add a test in `src/parser.rs` that takes a known binary payload from `test_data/` and parses it into a `BuildScanPayload`.

**Step 4: Run tests**

Run `cargo test`.

**Step 5: Commit**

```bash
git add src/parser.rs src/main.rs
git commit -m "feat: implement stateful payload builder"
```
