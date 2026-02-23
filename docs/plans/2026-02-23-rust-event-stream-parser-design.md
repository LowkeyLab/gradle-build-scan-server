# Rust Event Stream Parser — Design

**Date:** 2026-02-23
**Status:** Approved
**Supersedes:** 2026-02-20-build-scan-parser-design.md, 2026-02-21-task-centric-parser-design.md

## Context

The Gradle build scan binary format is now fully understood (see Obsidian: "Gradle Build Scan Binary Format — Reverse Engineering Notes"). The existing Rust parser (`parser.rs`, `primitives.rs`) was built on incorrect hypotheses (plain LEB128 event IDs, shifted-bit string encoding) and needs a complete rewrite.

The actual format uses:
- **Outer header**: magic 0x28C5, version 2, tool version blob, then gzip stream
- **Delta-encoded event framing**: inverted flags byte, ZigZag-encoded deltas for wire_id/timestamp/ordinal, body_length + body bytes
- **Kryo-serialized event bodies**: per-event flags bitmask (inverted: 0=present), field-presence protocol, per-event string interning (ZigZag sign discriminates new vs back-ref)

## Scope

**MVP:** Parse the full event stream framing (all 474 events). Decode bodies for TaskIdentity_1_0, TaskStarted_1_6, and TaskFinished_1_8 only. Collect unknown event bodies as raw bytes. Assemble task events into structured output with timing info.

## Architecture: Trait-Based Plugin

```
raw bytes
  → OuterHeader::parse()          # magic, version, tool blob
  → Decompressor::decompress()    # gzip (reuse existing)
  → EventFrameReader (Iterator)   # delta-decoded framing
  → DecoderRegistry (trait dispatch)  # BodyDecoder per wire_id
  → assemble()                    # correlate by task ID
  → BuildScanPayload (JSON output)
```

### Module Structure

```
build-scan/lib/src/
├── lib.rs              # Public API re-exports
├── error.rs            # Extended error types (keep + extend)
├── decompress.rs       # Gzip decompression (keep as-is)
├── varint.rs           # LEB128 + ZigZag encoding (replaces primitives.rs)
├── outer_header.rs     # Outer header parser (magic, version, tool blob)
├── framing.rs          # Delta-encoded event stream framing
├── kryo.rs             # Kryo body primitives (flags, interned strings, enums)
├── events/
│   ├── mod.rs          # BodyDecoder trait + DecoderRegistry + RawEvent
│   ├── task_identity.rs
│   ├── task_started.rs
│   └── task_finished.rs
├── assembly.rs         # Correlates events into structured BuildScanPayload
└── models.rs           # Output types (rewritten)
```

Files deleted: `primitives.rs`, `parser.rs`

## Core Types

### varint.rs

```rust
pub fn read_unsigned_varint(data: &[u8], pos: &mut usize) -> Result<u64>;
pub fn zigzag_decode_i32(n: u32) -> i32;
pub fn zigzag_decode_i64(n: u64) -> i64;
pub fn read_zigzag_i32(data: &[u8], pos: &mut usize) -> Result<i32>;
pub fn read_zigzag_i64(data: &[u8], pos: &mut usize) -> Result<i64>;
```

### framing.rs

```rust
pub struct FramedEvent {
    pub wire_id: u16,
    pub timestamp: i64,       // millis since epoch, accumulated from deltas
    pub ordinal: i32,         // accumulated from deltas
    pub body: Vec<u8>,        // raw Kryo-serialized body
}

pub struct EventFrameReader<'a> { /* Iterator over FramedEvents */ }
```

Per-event framing (from decompiled `serializer/c.class`):
```
[unsigned varint]   flags (inverted: bit=0 means present)
                      bit0: type delta
                      bit1: timestamp delta
                      bit2: actual-timestamp delta
                      bit3: ordinal delta
[zigzag varint]     type_delta     (if bit0 == 0)
[zigzag long]       ts_delta       (if bit1 == 0)
[zigzag long]       actual_delta   (if bit2 == 0)
[zigzag int]        ord_delta      (if bit3 == 0, else default +1)
[unsigned varint]   body_length    (always present)
[bytes]             body           (body_length bytes)
```

### kryo.rs

```rust
pub struct StringInternTable { strings: Vec<String> }

impl StringInternTable {
    /// ZigZag varint: >= 0 = new string (char count), < 0 = back-ref (index = -1 - value)
    /// Characters: unsigned LEB128 varints (ASCII = 1 byte each)
    /// Scope: per-event body (fresh table per decode call)
    pub fn read_string(&mut self, data: &[u8], pos: &mut usize) -> Result<String>;
}

pub fn read_flags_byte(data: &[u8], pos: &mut usize) -> Result<u8>;
pub fn read_flags_short(data: &[u8], pos: &mut usize) -> Result<u16>;
/// Inverted: bit=0 means field IS present
pub fn is_field_present(flags: u16, bit: u8) -> bool;
```

### events/mod.rs

```rust
pub trait BodyDecoder: Send + Sync {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent>;
}

pub enum DecodedEvent {
    TaskIdentity(TaskIdentityEvent),
    TaskStarted(TaskStartedEvent),
    TaskFinished(TaskFinishedEvent),
    Raw(RawEvent),
}

pub struct RawEvent { pub wire_id: u16, pub body: Vec<u8> }

pub struct DecoderRegistry {
    decoders: HashMap<u16, Box<dyn BodyDecoder>>,
}
```

Wire ID registration:
- TaskIdentity_1_0: wire_id = 117 (ordinal 117, version 0)
- TaskStarted_1_6: wire_id = 1563 (27 + 6*256)
- TaskFinished_1_8: wire_id = 2074 (26 + 8*256)

## Event Body Layouts

### TaskIdentity_1_0 (wire_id 117)

```
[flags: 1 byte, 3 bits, inverted]
  bit 0 → id         (long, zigzag varint)
  bit 1 → buildPath  (interned string)
  bit 2 → taskPath   (interned string)
```

### TaskStarted_1_6 (wire_id 1563)

```
[flags: 1 byte, 5 bits, inverted]
  bit 0 → id         (long, zigzag varint)
  bit 1 → buildPath  (interned string)
  bit 2 → path       (interned string)
  bit 3 → className  (interned string)
  bit 4 → parent     (ConfigurationParentRef_1_0: flags byte + enum + long)
```

### TaskFinished_1_8 (wire_id 2074)

```
[flags: 2 bytes (short), 13 bits, inverted]
  bit 0  → id                            (long, zigzag varint)
  bit 1  → path                          (interned string)
  bit 2  → outcome                       (enum ordinal: 0=UpToDate..6=Avoided)
  bit 3  → skipMessage                   (interned string)
  bit 4  → cacheable                     (boolean — BIT IS THE VALUE, no payload)
  bit 5  → cachingDisabledReasonCategory (interned string)
  bit 6  → cachingDisabledExplanation    (interned string)
  bit 7  → originBuildInvocationId       (interned string)
  bit 8  → originBuildCacheKey           (byte array, length-prefixed)
  bit 9  → originExecutionTime           (long, different encoder)
  bit 10 → actionable                    (boolean — BIT IS THE VALUE, no payload)
  bit 11 → upToDateMessages              (list: int count + interned strings)
  bit 12 → skipReasonMessage             (interned string)
```

## Output Model

```rust
pub struct BuildScanPayload {
    pub tasks: Vec<Task>,
    pub raw_events: Vec<RawEventSummary>,
}

pub struct Task {
    pub id: i64,
    pub build_path: String,
    pub task_path: String,
    pub class_name: Option<String>,
    pub outcome: Option<TaskOutcome>,
    pub cacheable: Option<bool>,
    pub caching_disabled_reason: Option<String>,
    pub caching_disabled_explanation: Option<String>,
    pub origin_build_cache_key: Option<Vec<u8>>,
    pub actionable: Option<bool>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

pub enum TaskOutcome {
    UpToDate, Skipped, Failed, Success, FromCache, NoSource, AvoidedForUnknownReason,
}

pub struct RawEventSummary { pub wire_id: u16, pub count: usize }
```

Assembly: correlate TaskIdentity + TaskStarted + TaskFinished by their shared `id` field. Timestamps come from `FramedEvent.timestamp`.

## Testing Strategy

1. **varint.rs** — Unit tests for LEB128 decode, ZigZag encode/decode with known values. **Property-based tests** (proptest): roundtrip encode→decode = identity for arbitrary i32/i64/u64.
2. **framing.rs** — Hand-crafted byte sequences for delta-encoded events. Verify wire_id accumulation, timestamp accumulation, body extraction. Use reference payload's first event (DAEMON_STATE at offset 0, wire_id 265) as fixture.
3. **kryo.rs** — Unit tests for string interning (new string, back-reference, empty string). Flags bitmask tests (inverted bits). Property-based tests for string roundtrips.
4. **events/*.rs** — Each decoder tested with extracted body bytes from the reference payload.
5. **Integration test** — Parse full reference payload JSON → assert 474 events, 45 tasks, known task paths like `:app:compileKotlin` with correct outcomes.

**New dependency:** `proptest` for property-based testing (dev-dependency).

## Reference

- Obsidian: "Gradle Build Scan Binary Format — Reverse Engineering Notes" (single source of truth)
- Reference payload: `captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.json`
- Decompiled plugin: `/tmp/decompiled/com/gradle/scan/agent/serialization/scan/serializer/kryo/`
- Event model Javadoc: https://docs.gradle.com/enterprise/event-model-javadoc/
