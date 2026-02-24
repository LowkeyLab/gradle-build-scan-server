# PayloadBuilder Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor `PayloadBuilder` by introducing a `BuildScanParser` layer that combines decompression and payload building, allowing `PayloadBuilder` to be tested without compressing beforehand.

**Architecture:** We will create a new struct `BuildScanParser` in `build-scan/lib/src/parser.rs` which takes compressed data, decompresses it via `Decompressor`, and delegates to `PayloadBuilder`. The tests for `PayloadBuilder` will be modified to pass uncompressed raw bytes to `builder.build()`. We will also update `build-scan/cli/src/main.rs` to use `BuildScanParser`.

**Tech Stack:** Rust, Bazel

---

### Task 1: Refactor PayloadBuilder and introduce BuildScanParser

**Files:**
- Modify: `build-scan/lib/src/parser.rs`
- Modify: `build-scan/lib/src/lib.rs`

**Step 1: Write the failing tests / Modify existing tests**

In `build-scan/lib/src/parser.rs`, update `test_builder_maps_known_events` and `test_builder_parses_known_events_and_halts_on_unknown`. Remove the `gzip_compress` helper function and its imports, as we will use uncompressed `raw_data` directly. Change `builder.build_from_compressed(&payload)` to `builder.build(&raw_data)`.

Also add a test for `BuildScanParser::parse_compressed`.
```rust
    #[test]
    fn test_build_scan_parser_parses_compressed() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        fn gzip_compress(data: &[u8]) -> Vec<u8> {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }

        let mut raw_data = Vec::new();
        // Event 0 (Timestamp)
        raw_data.push(0);
        raw_data.push(0);

        let compressed = gzip_compress(&raw_data);
        
        let mut parser = BuildScanParser::new();
        let result = parser.parse_compressed(&compressed);
        assert!(result.is_ok());
    }
```

**Step 2: Run test to verify it fails**

Run: `aspect test //build-scan/lib/...`
Expected: Fails because `BuildScanParser` is not defined and `build_from_compressed` still exists inside `PayloadBuilder` but is removed from tests (tests were calling `.build` which is fine, but compiler errors out on `BuildScanParser`).

**Step 3: Write minimal implementation**

In `build-scan/lib/src/parser.rs`, create the `BuildScanParser` struct and move `build_from_compressed` from `PayloadBuilder`. Remove `build_from_compressed` from `PayloadBuilder`.

```rust
// Add below PayloadBuilder
pub struct BuildScanParser {
    pub builder: PayloadBuilder,
}

impl Default for BuildScanParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildScanParser {
    pub fn new() -> Self {
        Self {
            builder: PayloadBuilder::new(),
        }
    }

    pub fn parse_compressed(&mut self, data: &[u8]) -> Result<BuildScanPayload, ParseError> {
        let decompressed = Decompressor::decompress(data)?;
        self.builder.build(&decompressed)
    }
}
```

In `build-scan/lib/src/lib.rs`, add `pub use parser::BuildScanParser;`.
```rust
pub use parser::BuildScanParser;
```

**Step 4: Run test to verify it passes**

Run: `aspect test //build-scan/lib/...`
Expected: PASS

**Step 5: Commit**

```bash
bazel run //tools/format
git add build-scan/lib/src/parser.rs build-scan/lib/src/lib.rs
git commit -m "refactor: introduce BuildScanParser to handle decompression"
```

---

### Task 2: Update CLI to use BuildScanParser

**Files:**
- Modify: `build-scan/cli/src/main.rs`

**Step 1: Write the failing tests**

The tests in `build-scan/cli/src/main.rs` already pass compressed data, but the main code relies on `parser::PayloadBuilder` and `build_from_compressed`. We will just modify the code in `run_parse` as the CLI itself is tested by end-to-end integration tests in `main.rs`. Wait, tests are in the same file and will fail if we remove `build_from_compressed` from `PayloadBuilder`.

Since we just removed `build_from_compressed` from `PayloadBuilder`, the CLI compilation should be failing. 
Let's verify this.

**Step 2: Run test to verify it fails**

Run: `aspect build //build-scan/cli/...`
Expected: FAIL because `PayloadBuilder::build_from_compressed` doesn't exist.

**Step 3: Write minimal implementation**

In `build-scan/cli/src/main.rs`:
Change `run_parse`:
```rust
    // 5. Parse build scan
    let mut parser = parser::BuildScanParser::new();
    let build_scan = parser
        .parse_compressed(&raw_bytes)
        .context("Failed to parse build scan payload")?;
```

**Step 4: Run test to verify it passes**

Run: `aspect build //build-scan/cli/...` and `aspect test //build-scan/cli/...`
Expected: PASS

**Step 5: Commit**

```bash
bazel run //tools/format
git add build-scan/cli/src/main.rs
git commit -m "refactor: update cli to use BuildScanParser"
```
