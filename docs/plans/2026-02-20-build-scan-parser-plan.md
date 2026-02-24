# Build Scan Parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Correctly parse the raw uncompressed Stream produced by Gradle's `--scan` feature by migrating `StreamDecoder` to explicit token decoding rather than an iterator, resolving "Malformed LEB128 varint encountered" errors.

**Architecture:** A High-Level `PayloadBuilder` state machine interprets Context-Dependent raw Event IDs, instructing the Low-Level `StreamDecoder` to decode exact primitive types (`String`, `Varint`, `Timestamp`, `StringRef`).

**Tech Stack:** Rust, Bazel

---

### Task 1: Refactor Primitive Enum

**Files:**
- Modify: `build-scan/lib/src/primitives.rs`

**Step 1: Write the failing test**

Modify the existing `test_next_string` and `test_next_string_ref` tests to expect the new primitive parsing behavior. We will remove the `Iterator` impl, so tests must explicitly call methods.

```rust
#[test]
fn test_explicit_string() {
    let mut data = vec![0x0A]; // length 5, shifted 1 = 10 (0x0A)
    data.extend_from_slice(b"hello");
    let mut decoder = StreamDecoder::new(&data);
    assert_eq!(
        decoder.read_string().unwrap(),
        Primitive::String("hello".to_string())
    );
}

#[test]
fn test_explicit_string_ref() {
    let data = vec![0x55]; // ref 42 -> (42 << 1) | 1 = 85 (0x55)
    let mut decoder = StreamDecoder::new(&data);
    assert_eq!(decoder.read_string().unwrap(), Primitive::StringRef(42));
}

#[test]
fn test_explicit_timestamp() {
    let ts_val: u64 = 1771622196842;
    // encode ts_val as varint bytes
    let mut data = Vec::new();
    let mut val = ts_val;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        data.push(byte);
        if val == 0 {
            break;
        }
    }
    let mut decoder = StreamDecoder::new(&data);
    let prim = decoder.read_timestamp().unwrap();
    if let Primitive::Timestamp(dt) = prim {
        assert_eq!(dt.timestamp_millis(), ts_val as i64);
    } else {
        panic!("Expected Timestamp primitive");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `bazel test //build-scan/lib/src:parser_test` (Assuming `parser` is the crate name for `lib/src`)
Expected: FAIL due to missing methods `read_string`, `read_timestamp` on `StreamDecoder`.

**Step 3: Write minimal implementation**

Update `build-scan/lib/src/primitives.rs`:

```rust
// Remove `impl<'a> Iterator for StreamDecoder<'a>` entirely

impl<'a> StreamDecoder<'a> {
    // Keep existing `new`, `decompress`, `read_leb128` (maybe rename to `read_raw_varint`)

    pub fn read_raw_varint(&mut self) -> Result<u64, ParseError> {
        self.read_leb128()
    }

    pub fn read_varint(&mut self) -> Result<Primitive, ParseError> {
        self.read_raw_varint().map(Primitive::Varint)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ParseError> {
        if self.offset + len > self.data.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let bytes = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_string(&mut self) -> Result<Primitive, ParseError> {
        let val = self.read_raw_varint()?;
        let bit = val & 1;
        let shifted = val >> 1;

        if bit == 1 {
            Ok(Primitive::StringRef(shifted as u32))
        } else {
            let len = shifted as usize;
            let bytes = self.read_bytes(len)?;
            let s = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidUtf8)?;
            Ok(Primitive::String(s.to_string()))
        }
    }

    pub fn read_timestamp(&mut self) -> Result<Primitive, ParseError> {
        let val = self.read_raw_varint()?;
        let dt = DateTime::from_timestamp((val / 1000) as i64, ((val % 1000) * 1_000_000) as u32)
            .ok_or(ParseError::InvalidTimestamp)?;
        Ok(Primitive::Timestamp(dt))
    }
}
```

Add `InvalidUtf8` and `InvalidTimestamp` to `ParseError` in `build-scan/lib/src/error.rs` if needed.

**Step 4: Run test to verify it passes**

Run: `bazel test //build-scan/lib/src:parser_test` (or `aspect test //...`)
Expected: PASS

**Step 5: Commit**

```bash
bazel run //tools/format
git add build-scan/lib/src/primitives.rs build-scan/lib/src/error.rs
git commit -m "refactor: migrate StreamDecoder to explicit primitive decoding"
```

---

### Task 2: Refactor Parser State Machine

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

We need to test that the parser can iterate through the explicit decoder until EOF without crashing.

```rust
#[test]
fn test_builder_graceful_unknown_events() {
    let mut builder = PayloadBuilder::new();
    // A dummy payload: Event 99, value 42
    // 99 = 0x63, 42 = 0x2A
    // Since we don't handle it, it should just error or skip. 
    // Right now, let's just make sure it doesn't panic.
    // We can't compress here easily in the test, so we'll mock StreamDecoder usage.
    // (We might need to adjust tests based on exact `build` method signature).
}
```
*Note: The existing tests might just need updating to compile since `decoder` is no longer an `Iterator`.*

**Step 2: Run test to verify it fails**

Run: `aspect test //...`
Expected: FAIL because `decoder` is used as an iterator in `parser.rs`.

**Step 3: Write minimal implementation**

Update `build-scan/lib/src/parser.rs`:

```rust
impl PayloadBuilder {
    pub fn build(&mut self, data: &[u8]) -> Result<BuildScanPayload, ParseError> {
        let decompressed = StreamDecoder::decompress(data)?;
        let mut decoder = StreamDecoder::new(&decompressed);
        let payload = BuildScanPayload::default();

        loop {
            // Try to read next event ID. If EOF, we're done.
            let event_id = match decoder.read_raw_varint() {
                Ok(id) => id,
                Err(ParseError::UnexpectedEof) => break,
                Err(e) => return Err(e),
            };

            // Hybrid State Machine: 
            // For now, if we don't recognize the event, we can't reliably skip it 
            // because we don't know if the next varint is a length for raw bytes.
            // But we will print it to stderr and abort to avoid losing sync.
            match event_id {
                // TODO: Add actual event mappings here later
                _ => {
                    eprintln!("Unknown Event ID encountered: {}", event_id);
                    return Err(ParseError::UnknownEvent { id: event_id });
                }
            }
        }
        Ok(payload)
    }
}
```
Update `error.rs` to include `UnknownEvent { id: u64 }`.

**Step 4: Run test to verify it passes**

Run: `aspect test //...`
Expected: PASS (or fail on `UnknownEvent` if testing with real data, which is correct behavior for now until we map events).

**Step 5: Commit**

```bash
bazel run //tools/format
git add build-scan/lib/src/parser.rs build-scan/lib/src/error.rs
git commit -m "feat: implement high-level payload parser state machine"
```

---

### Task 3: Map Known Events (Optional Initial Pass)

**Files:**
- Modify: `build-scan/lib/src/parser.rs`

**Step 1: Write the failing test**

Create a test payload containing Event 12 (Project Path?), Event 14 (String?), etc. based on reverse-engineered payloads.

**Step 2: Run test to verify it fails**

Run: `aspect test //...`
Expected: FAIL due to `UnknownEvent`.

**Step 3: Write minimal implementation**

Update `build-scan/lib/src/parser.rs` event loop to handle some known sequences (e.g. `Event 12 -> read_varint`, `Event 0 -> read_timestamp` or similar sequences you discovered).

```rust
            match event_id {
                12 => {
                    // Example: Event 12 is followed by a varint? 
                    let val = decoder.read_varint()?;
                    // Store/Ignore
                },
                14 => {
                    // Example: Event 14 is a string addition to the dictionary
                    let s = decoder.read_string()?;
                    if let Primitive::String(st) = s {
                        self.dictionary.push(st);
                    }
                },
                _ => {
                    // We don't know what follows this event, we must abort parsing
                    eprintln!("Unknown Event ID encountered: {}", event_id);
                    return Err(ParseError::UnknownEvent { id: event_id });
                }
            }
```
*Note: You will iteratively add event definitions to this match block based on captured payloads.*

**Step 4: Run test to verify it passes**

Run: `aspect test //...`
Expected: PASS

**Step 5: Commit**

```bash
bazel run //tools/format
git add build-scan/lib/src/parser.rs
git commit -m "feat: parse known build scan events"
```
