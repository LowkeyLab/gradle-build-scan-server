# Rust Event Stream Parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite the Rust build scan parser to correctly decode the delta-encoded Gradle event stream with Kryo body decoding for task events.

**Architecture:** Trait-based plugin with three layers: framing (delta-decode event stream into wire_id + body bytes), body decoders (per-wire-id Kryo deserializers registered via `BodyDecoder` trait), and assembly (correlate task events by ID into structured output).

**Tech Stack:** Rust 1.93.1, Bazel with rules_rust, flate2 for gzip, serde for JSON, proptest for property-based tests, thiserror for errors.

**Design doc:** `docs/plans/2026-02-23-rust-event-stream-parser-design.md`

**Reference payload:** `captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.json`

---

## Task 1: Delete Old Files and Extend Errors

**Files:**
- Delete: `build-scan/lib/src/primitives.rs`
- Delete: `build-scan/lib/src/parser.rs`
- Modify: `build-scan/lib/src/error.rs`

**Step 1: Delete obsolete files**

```bash
rm build-scan/lib/src/primitives.rs build-scan/lib/src/parser.rs
```

**Step 2: Extend error.rs with new variants**

Replace `build-scan/lib/src/error.rs` with:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to decompress Gzip stream")]
    InvalidGzip,
    #[error("Malformed LEB128 varint at offset {offset}")]
    MalformedLeb128 { offset: usize },
    #[error("Unexpected end of data at offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,
    #[error("Invalid outer header: {reason}")]
    InvalidHeader { reason: &'static str },
    #[error("Invalid string intern reference: index {index}")]
    InvalidStringRef { index: usize },
    #[error("Invalid enum ordinal {ordinal} for {enum_name}")]
    InvalidEnumOrdinal { ordinal: u64, enum_name: &'static str },
    #[error("Unknown wire ID {wire_id}: body stored as raw bytes")]
    UnknownWireId { wire_id: u16 },
    #[error("Task ID {id} referenced but no identity event found")]
    OrphanTaskEvent { id: i64 },
}
```

**Step 3: Commit**

```bash
git add -A && git commit -m "refactor: delete old parser/primitives, extend error types"
```

---

## Task 2: varint.rs — LEB128 + ZigZag

**Files:**
- Create: `build-scan/lib/src/varint.rs`

**Step 1: Write failing tests**

Create `build-scan/lib/src/varint.rs` with tests only:

```rust
use error::ParseError;

pub fn read_unsigned_varint(data: &[u8], pos: &mut usize) -> Result<u64, ParseError> {
    todo!()
}

pub fn zigzag_decode_i32(n: u32) -> i32 {
    todo!()
}

pub fn zigzag_decode_i64(n: u64) -> i64 {
    todo!()
}

pub fn read_zigzag_i32(data: &[u8], pos: &mut usize) -> Result<i32, ParseError> {
    todo!()
}

pub fn read_zigzag_i64(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_unsigned_varint_single_byte() {
        let data = [0x05];
        let mut pos = 0;
        assert_eq!(read_unsigned_varint(&data, &mut pos).unwrap(), 5);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_unsigned_varint_two_bytes() {
        // 0x92 0x04 = (0x12 | (0x04 << 7)) = 18 + 512 = 530
        let data = [0x92, 0x04];
        let mut pos = 0;
        assert_eq!(read_unsigned_varint(&data, &mut pos).unwrap(), 530);
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_read_unsigned_varint_eof() {
        let data = [0x80]; // continuation bit set but no more bytes
        let mut pos = 0;
        assert!(read_unsigned_varint(&data, &mut pos).is_err());
    }

    #[test]
    fn test_zigzag_decode_i32() {
        assert_eq!(zigzag_decode_i32(0), 0);
        assert_eq!(zigzag_decode_i32(1), -1);
        assert_eq!(zigzag_decode_i32(2), 1);
        assert_eq!(zigzag_decode_i32(3), -2);
        assert_eq!(zigzag_decode_i32(530), 265);   // type_delta for DAEMON_STATE
        assert_eq!(zigzag_decode_i32(517), -259);   // type_delta for BUILD_STARTED
    }

    #[test]
    fn test_zigzag_decode_i64() {
        assert_eq!(zigzag_decode_i64(0), 0);
        assert_eq!(zigzag_decode_i64(1), -1);
        assert_eq!(zigzag_decode_i64(2), 1);
    }

    #[test]
    fn test_read_zigzag_i32() {
        // 0x92 0x04 = unsigned 530 → zigzag → 265
        let data = [0x92, 0x04];
        let mut pos = 0;
        assert_eq!(read_zigzag_i32(&data, &mut pos).unwrap(), 265);
    }
}
```

**Step 2: Run gazelle then test to verify failure**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:varint_test
```

Expected: FAIL (all `todo!()`)

**Step 3: Implement varint functions**

Replace the `todo!()` bodies:

```rust
use error::ParseError;

pub fn read_unsigned_varint(data: &[u8], pos: &mut usize) -> Result<u64, ParseError> {
    let start = *pos;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= data.len() {
            return Err(ParseError::UnexpectedEof { offset: *pos });
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(ParseError::MalformedLeb128 { offset: start });
        }
    }
}

pub fn zigzag_decode_i32(n: u32) -> i32 {
    ((n >> 1) as i32) ^ -((n & 1) as i32)
}

pub fn zigzag_decode_i64(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

pub fn read_zigzag_i32(data: &[u8], pos: &mut usize) -> Result<i32, ParseError> {
    let raw = read_unsigned_varint(data, pos)?;
    Ok(zigzag_decode_i32(raw as u32))
}

pub fn read_zigzag_i64(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    let raw = read_unsigned_varint(data, pos)?;
    Ok(zigzag_decode_i64(raw))
}
```

**Step 4: Run tests**

```bash
aspect test //build-scan/lib/src:varint_test
```

Expected: PASS

**Step 5: Add proptest roundtrip tests**

Add `proptest` to `Cargo.toml` under `[dependencies]`:

```toml
proptest = "1.0"
```

Add to the test module in `varint.rs`:

```rust
    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn encode_unsigned_varint(mut value: u64) -> Vec<u8> {
            let mut buf = Vec::new();
            loop {
                let mut byte = (value & 0x7F) as u8;
                value >>= 7;
                if value != 0 {
                    byte |= 0x80;
                }
                buf.push(byte);
                if value == 0 {
                    break;
                }
            }
            buf
        }

        fn zigzag_encode_i32(n: i32) -> u32 {
            ((n << 1) ^ (n >> 31)) as u32
        }

        fn zigzag_encode_i64(n: i64) -> u64 {
            ((n << 1) ^ (n >> 63)) as u64
        }

        proptest! {
            #[test]
            fn roundtrip_unsigned_varint(value: u64) {
                let encoded = encode_unsigned_varint(value);
                let mut pos = 0;
                let decoded = read_unsigned_varint(&encoded, &mut pos).unwrap();
                prop_assert_eq!(decoded, value);
                prop_assert_eq!(pos, encoded.len());
            }

            #[test]
            fn roundtrip_zigzag_i32(value: i32) {
                let encoded = zigzag_encode_i32(value);
                let decoded = zigzag_decode_i32(encoded);
                prop_assert_eq!(decoded, value);
            }

            #[test]
            fn roundtrip_zigzag_i64(value: i64) {
                let encoded = zigzag_encode_i64(value);
                let decoded = zigzag_decode_i64(encoded);
                prop_assert_eq!(decoded, value);
            }
        }
    }
```

**Step 6: Run gazelle, then all varint tests**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:varint_test
```

Expected: PASS

**Step 7: Commit**

```bash
git add -A && git commit -m "feat: add varint module with LEB128 + ZigZag encoding"
```

---

## Task 3: outer_header.rs — Outer Header Parser

**Files:**
- Create: `build-scan/lib/src/outer_header.rs`

The outer header is: `28 C5` magic + `00 02` version + `00 16` blob_len + DataOutputStream.writeUTF strings.

**Step 1: Write failing tests**

```rust
use error::ParseError;

pub struct OuterHeader {
    pub version: u16,
    pub tool_type: String,
    pub tool_version: String,
    pub plugin_version: String,
    pub gzip_offset: usize,
}

impl OuterHeader {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference outer header bytes:
    // 28 c5 00 02 00 16 00 06 47 52 41 44 4c 45 00 05 39 2e 33 2e 31 00 05 34 2e 33 2e 32
    const HEADER_BYTES: [u8; 28] = [
        0x28, 0xc5, 0x00, 0x02, 0x00, 0x16,
        0x00, 0x06, 0x47, 0x52, 0x41, 0x44, 0x4c, 0x45,  // "GRADLE"
        0x00, 0x05, 0x39, 0x2e, 0x33, 0x2e, 0x31,          // "9.3.1"
        0x00, 0x05, 0x34, 0x2e, 0x33, 0x2e, 0x32,          // "4.3.2"
    ];

    #[test]
    fn test_parse_reference_header() {
        let header = OuterHeader::parse(&HEADER_BYTES).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.tool_type, "GRADLE");
        assert_eq!(header.tool_version, "9.3.1");
        assert_eq!(header.plugin_version, "4.3.2");
        assert_eq!(header.gzip_offset, 28);
    }

    #[test]
    fn test_bad_magic() {
        let mut bad = HEADER_BYTES;
        bad[0] = 0x00;
        assert!(OuterHeader::parse(&bad).is_err());
    }

    #[test]
    fn test_truncated_header() {
        assert!(OuterHeader::parse(&HEADER_BYTES[..4]).is_err());
    }
}
```

**Step 2: Run gazelle then test to verify failure**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:outer_header_test
```

**Step 3: Implement**

```rust
impl OuterHeader {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 6 {
            return Err(ParseError::InvalidHeader { reason: "too short for magic+version+blob_len" });
        }
        let magic = u16::from_be_bytes([data[0], data[1]]);
        if magic != 0x28C5 {
            return Err(ParseError::InvalidHeader { reason: "bad magic bytes" });
        }
        let version = u16::from_be_bytes([data[2], data[3]]);
        let blob_len = u16::from_be_bytes([data[4], data[5]]) as usize;
        let blob_end = 6 + blob_len;
        if data.len() < blob_end {
            return Err(ParseError::InvalidHeader { reason: "truncated tool version blob" });
        }
        let mut pos = 6;
        let tool_type = Self::read_utf(&data, &mut pos, blob_end)?;
        let tool_version = Self::read_utf(&data, &mut pos, blob_end)?;
        let plugin_version = Self::read_utf(&data, &mut pos, blob_end)?;
        Ok(Self { version, tool_type, tool_version, plugin_version, gzip_offset: blob_end })
    }

    fn read_utf(data: &[u8], pos: &mut usize, limit: usize) -> Result<String, ParseError> {
        if *pos + 2 > limit {
            return Err(ParseError::InvalidHeader { reason: "truncated UTF string length" });
        }
        let len = u16::from_be_bytes([data[*pos], data[*pos + 1]]) as usize;
        *pos += 2;
        if *pos + len > limit {
            return Err(ParseError::InvalidHeader { reason: "truncated UTF string data" });
        }
        let s = std::str::from_utf8(&data[*pos..*pos + len])
            .map_err(|_| ParseError::InvalidUtf8)?;
        *pos += len;
        Ok(s.to_string())
    }
}
```

**Step 4: Run tests**

```bash
aspect test //build-scan/lib/src:outer_header_test
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add outer header parser for magic+version+tool blob"
```

---

## Task 4: framing.rs — Delta-Encoded Event Stream

**Files:**
- Create: `build-scan/lib/src/framing.rs`

**Step 1: Write failing tests using reference payload bytes**

```rust
use error::ParseError;
use varint;

pub struct FramedEvent {
    pub wire_id: u16,
    pub timestamp: i64,
    pub ordinal: i32,
    pub body: Vec<u8>,
}

pub struct EventFrameReader<'a> {
    data: &'a [u8],
    pos: usize,
    wire_id: i64,
    timestamp: i64,
    ordinal: i32,
}

impl<'a> EventFrameReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, wire_id: 0, timestamp: 0, ordinal: 0 }
    }
}

impl<'a> Iterator for EventFrameReader<'a> {
    type Item = Result<FramedEvent, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // First 5 events from reference payload (decompressed):
    // Event 0: flags=0x0c, wire_delta=265, ts_delta=large, body_len=14
    // Event 1: flags=0x0e, wire_delta=-259, body_len=0  → wire_id=6 (BUILD_STARTED)
    // Event 2: flags=0x0e, wire_delta=-1, body_len=8    → wire_id=5 (BUILD_REQUESTED_TASKS)
    // Event 3: flags=0x0e, wire_delta=511, body_len=3   → wire_id=516 (BUILD_MODES_v2)
    // Event 4: flags=0x0e, wire_delta=-504, body_len=1  → wire_id=12 (HARDWARE)

    fn first_five_events_bytes() -> Vec<u8> {
        hex::decode(
            "0c9204b4c584d390670e00bba9c2a9c83301018097930500\
             0e85040\
             00e010802010a6275696c64\
             0efe070301bd10\
             0eef070110"
        ).unwrap()
    }

    #[test]
    fn test_first_event_wire_id() {
        let data = first_five_events_bytes();
        let mut reader = EventFrameReader::new(&data);
        let event = reader.next().unwrap().unwrap();
        assert_eq!(event.wire_id, 265);  // DAEMON_STATE_v1
        assert_eq!(event.body.len(), 14);
    }

    #[test]
    fn test_five_events_wire_ids() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].wire_id, 265);  // DAEMON_STATE_v1
        assert_eq!(events[1].wire_id, 6);    // BUILD_STARTED
        assert_eq!(events[2].wire_id, 5);    // BUILD_REQUESTED_TASKS
        assert_eq!(events[3].wire_id, 516);  // BUILD_MODES_v2
        assert_eq!(events[4].wire_id, 12);   // HARDWARE
    }

    #[test]
    fn test_ordinals_increment() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // Default ordinal delta is +1 when bit3=1 (absent)
        assert_eq!(events[0].ordinal, 1);
        assert_eq!(events[1].ordinal, 2);
        assert_eq!(events[2].ordinal, 3);
    }

    #[test]
    fn test_body_content() {
        let data = first_five_events_bytes();
        let events: Vec<_> = EventFrameReader::new(&data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // Event 1 (BUILD_STARTED) has empty body
        assert!(events[1].body.is_empty());
        // Event 2 body contains "build"
        assert_eq!(events[2].body.len(), 8);
    }
}
```

Note: Add `hex` to Cargo.toml as a dev dependency for test fixtures: `hex = "0.4"`.

**Step 2: Run gazelle then test to verify failure**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:framing_test
```

**Step 3: Implement the Iterator**

```rust
impl<'a> Iterator for EventFrameReader<'a> {
    type Item = Result<FramedEvent, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }
        Some(self.read_next())
    }
}

impl<'a> EventFrameReader<'a> {
    fn read_next(&mut self) -> Result<FramedEvent, ParseError> {
        let flags = varint::read_unsigned_varint(self.data, &mut self.pos)? as u8;

        // bit0=0 → type delta present
        if flags & 1 == 0 {
            let delta = varint::read_zigzag_i32(self.data, &mut self.pos)?;
            self.wire_id += delta as i64;
        }
        // bit1=0 → timestamp delta present
        if flags & 2 == 0 {
            let delta = varint::read_zigzag_i64(self.data, &mut self.pos)?;
            self.timestamp += delta;
        }
        // bit2=0 → actual-timestamp delta present (read and discard)
        if flags & 4 == 0 {
            let _actual_delta = varint::read_zigzag_i64(self.data, &mut self.pos)?;
        }
        // bit3=0 → ordinal delta present; bit3=1 → default +1
        if flags & 8 == 0 {
            let delta = varint::read_zigzag_i32(self.data, &mut self.pos)?;
            self.ordinal += delta;
        } else {
            self.ordinal += 1;
        }

        let body_length = varint::read_unsigned_varint(self.data, &mut self.pos)? as usize;
        if self.pos + body_length > self.data.len() {
            return Err(ParseError::UnexpectedEof { offset: self.pos });
        }
        let body = self.data[self.pos..self.pos + body_length].to_vec();
        self.pos += body_length;

        Ok(FramedEvent {
            wire_id: self.wire_id as u16,
            timestamp: self.timestamp,
            ordinal: self.ordinal,
            body,
        })
    }
}
```

**Step 4: Run tests**

```bash
aspect test //build-scan/lib/src:framing_test
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add delta-encoded event frame reader"
```

---

## Task 5: kryo.rs — Kryo Body Primitives

**Files:**
- Create: `build-scan/lib/src/kryo.rs`

**Step 1: Write failing tests**

```rust
use error::ParseError;
use varint;

pub struct StringInternTable {
    strings: Vec<String>,
}

impl StringInternTable {
    pub fn new() -> Self {
        Self { strings: Vec::new() }
    }

    pub fn read_string(&mut self, data: &[u8], pos: &mut usize) -> Result<String, ParseError> {
        todo!()
    }
}

/// Read flags as unsigned varint, return as u8 (for <= 8 fields)
pub fn read_flags_byte(data: &[u8], pos: &mut usize) -> Result<u8, ParseError> {
    Ok(varint::read_unsigned_varint(data, pos)? as u8)
}

/// Read flags as unsigned varint, return as u16 (for <= 16 fields)
pub fn read_flags_short(data: &[u8], pos: &mut usize) -> Result<u16, ParseError> {
    Ok(varint::read_unsigned_varint(data, pos)? as u16)
}

/// Inverted: bit=0 means field IS present
pub fn is_field_present(flags: u16, bit: u8) -> bool {
    (flags >> bit) & 1 == 0
}

/// Read enum ordinal as unsigned varint
pub fn read_enum_ordinal(data: &[u8], pos: &mut usize) -> Result<u64, ParseError> {
    varint::read_unsigned_varint(data, pos)
}

/// Read a byte array: unsigned varint length, then that many bytes
pub fn read_byte_array(data: &[u8], pos: &mut usize) -> Result<Vec<u8>, ParseError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_inverted() {
        // 0b00000000 = all bits 0 → all fields present
        assert!(is_field_present(0x00, 0));
        assert!(is_field_present(0x00, 1));
        // 0b00000001 = bit0=1 → field 0 absent
        assert!(!is_field_present(0x01, 0));
        assert!(is_field_present(0x01, 1));
    }

    #[test]
    fn test_string_intern_new_ascii() {
        // ZigZag(3) = 6, then chars 'f'=102, 'o'=111, 'o'=111
        let data = [0x06, 0x66, 0x6f, 0x6f];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_string_intern_back_reference() {
        // First: write "foo" (zigzag(3)=6, then chars)
        // Then: back-ref to index 0 → zigzag(-1) = 1
        let data = [0x06, 0x66, 0x6f, 0x6f, 0x01];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_string_intern_empty_string() {
        // ZigZag(0) = 0, no chars follow
        let data = [0x00];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "");
    }

    #[test]
    fn test_string_intern_multiple_refs() {
        // "abc" then "xyz" then ref(0) then ref(1)
        // zigzag(3)=6, 'a'=97, 'b'=98, 'c'=99
        // zigzag(3)=6, 'x'=120, 'y'=121, 'z'=122
        // zigzag(-1)=1 → ref 0 = "abc"
        // zigzag(-2)=3 → ref 1 = "xyz"
        let data = [
            0x06, 97, 98, 99,   // "abc"
            0x06, 120, 121, 122, // "xyz"
            0x01,                // ref(0) = "abc"
            0x03,                // ref(1) = "xyz"
        ];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "abc");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "xyz");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "abc");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "xyz");
    }

    #[test]
    fn test_read_byte_array() {
        // length=3, then bytes [0xAA, 0xBB, 0xCC]
        let data = [0x03, 0xAA, 0xBB, 0xCC];
        let mut pos = 0;
        assert_eq!(read_byte_array(&data, &mut pos).unwrap(), vec![0xAA, 0xBB, 0xCC]);
    }
}
```

**Step 2: Run gazelle then test**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:kryo_test
```

**Step 3: Implement StringInternTable::read_string and read_byte_array**

```rust
impl StringInternTable {
    pub fn read_string(&mut self, data: &[u8], pos: &mut usize) -> Result<String, ParseError> {
        let raw = varint::read_zigzag_i32(data, pos)?;
        if raw < 0 {
            // Back-reference: index = -1 - raw
            let index = (-1 - raw) as usize;
            self.strings.get(index)
                .cloned()
                .ok_or(ParseError::InvalidStringRef { index })
        } else {
            // New string: raw = character count
            let char_count = raw as usize;
            let mut s = String::with_capacity(char_count);
            for _ in 0..char_count {
                let ch = varint::read_unsigned_varint(data, pos)? as u32;
                let c = char::from_u32(ch).ok_or(ParseError::InvalidUtf8)?;
                s.push(c);
            }
            self.strings.push(s.clone());
            Ok(s)
        }
    }
}

pub fn read_byte_array(data: &[u8], pos: &mut usize) -> Result<Vec<u8>, ParseError> {
    let len = varint::read_unsigned_varint(data, pos)? as usize;
    if *pos + len > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let bytes = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(bytes)
}
```

**Step 4: Run tests**

```bash
aspect test //build-scan/lib/src:kryo_test
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add Kryo body primitives (string interning, flags, byte arrays)"
```

---

## Task 6: models.rs — Output Types

**Files:**
- Rewrite: `build-scan/lib/src/models.rs`

**Step 1: Replace models.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub tasks: Vec<Task>,
    pub raw_events: Vec<RawEventSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub build_path: String,
    pub task_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TaskOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cacheable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caching_disabled_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caching_disabled_explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actionable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutcome {
    UpToDate,
    Skipped,
    Failed,
    Success,
    FromCache,
    NoSource,
    AvoidedForUnknownReason,
}

impl TaskOutcome {
    pub fn from_ordinal(ordinal: u64) -> Option<Self> {
        match ordinal {
            0 => Some(Self::UpToDate),
            1 => Some(Self::Skipped),
            2 => Some(Self::Failed),
            3 => Some(Self::Success),
            4 => Some(Self::FromCache),
            5 => Some(Self::NoSource),
            6 => Some(Self::AvoidedForUnknownReason),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventSummary {
    pub wire_id: u16,
    pub count: usize,
}
```

**Step 2: Run gazelle then build**

```bash
bazel run gazelle
aspect build //build-scan/lib/src:models
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: rewrite models with Task, TaskOutcome, RawEventSummary"
```

---

## Task 7: events/ — BodyDecoder Trait + Registry + TaskIdentity

**Files:**
- Create: `build-scan/lib/src/events/mod.rs`
- Create: `build-scan/lib/src/events/task_identity.rs`

**Step 1: Create events/mod.rs with trait, registry, and event types**

```rust
use std::collections::HashMap;

use error::ParseError;

pub mod task_identity;
pub mod task_started;
pub mod task_finished;

pub trait BodyDecoder: Send + Sync {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError>;
}

#[derive(Debug, Clone)]
pub enum DecodedEvent {
    TaskIdentity(TaskIdentityEvent),
    TaskStarted(TaskStartedEvent),
    TaskFinished(TaskFinishedEvent),
    Raw(RawEvent),
}

#[derive(Debug, Clone)]
pub struct TaskIdentityEvent {
    pub id: i64,
    pub build_path: String,
    pub task_path: String,
}

#[derive(Debug, Clone)]
pub struct TaskStartedEvent {
    pub id: i64,
    pub build_path: String,
    pub path: String,
    pub class_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskFinishedEvent {
    pub id: i64,
    pub path: String,
    pub outcome: Option<u64>,
    pub cacheable: Option<bool>,
    pub caching_disabled_reason_category: Option<String>,
    pub caching_disabled_explanation: Option<String>,
    pub origin_build_invocation_id: Option<String>,
    pub origin_build_cache_key: Option<Vec<u8>>,
    pub actionable: Option<bool>,
    pub skip_reason_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RawEvent {
    pub wire_id: u16,
    pub body: Vec<u8>,
}

pub struct DecoderRegistry {
    decoders: HashMap<u16, Box<dyn BodyDecoder>>,
}

impl DecoderRegistry {
    pub fn new() -> Self {
        let mut registry = Self { decoders: HashMap::new() };
        registry.register(117, Box::new(task_identity::TaskIdentityDecoder));
        registry.register(1563, Box::new(task_started::TaskStartedDecoder));
        registry.register(2074, Box::new(task_finished::TaskFinishedDecoder));
        registry
    }

    pub fn register(&mut self, wire_id: u16, decoder: Box<dyn BodyDecoder>) {
        self.decoders.insert(wire_id, decoder);
    }

    pub fn decode(&self, wire_id: u16, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        match self.decoders.get(&wire_id) {
            Some(decoder) => decoder.decode(body),
            None => Ok(DecodedEvent::Raw(RawEvent { wire_id, body: body.to_vec() })),
        }
    }
}
```

**Step 2: Create events/task_identity.rs with test**

```rust
use error::ParseError;
use kryo;
use varint;

use super::{BodyDecoder, DecodedEvent, TaskIdentityEvent};

pub struct TaskIdentityDecoder;

impl BodyDecoder for TaskIdentityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            varint::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };
        let build_path = if kryo::is_field_present(flags as u16, 1) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let task_path = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        Ok(DecodedEvent::TaskIdentity(TaskIdentityEvent { id, build_path, task_path }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_fields_present() {
        // flags=0x00 (all present), id=zigzag(1)=2, buildPath=":", taskPath=":app:build"
        let mut data = vec![0x00]; // flags: all present
        data.push(0x02);          // id: zigzag(1)=2
        // buildPath ":"  → zigzag(1)=2, then char ':'=58
        data.push(0x02); data.push(58);
        // taskPath ":app:build" → zigzag(10)=20, then 10 chars
        data.push(0x14);
        for &c in b":app:build" { data.push(c); }

        let decoder = TaskIdentityDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskIdentity(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.build_path, ":");
            assert_eq!(e.task_path, ":app:build");
        } else {
            panic!("expected TaskIdentity");
        }
    }
}
```

**Step 3: Run gazelle then test**

```bash
bazel run gazelle
aspect test //build-scan/lib/src/events:task_identity_test
```

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add BodyDecoder trait, registry, and TaskIdentity decoder"
```

---

## Task 8: events/task_started.rs

**Files:**
- Create: `build-scan/lib/src/events/task_started.rs`

**Step 1: Implement with test**

```rust
use error::ParseError;
use kryo;
use varint;

use super::{BodyDecoder, DecodedEvent, TaskStartedEvent};

pub struct TaskStartedDecoder;

impl BodyDecoder for TaskStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            varint::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };
        let build_path = if kryo::is_field_present(flags as u16, 1) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let path = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let class_name = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };
        // bit 4: parent (ConfigurationParentRef) — skip if present
        if kryo::is_field_present(flags as u16, 4) {
            // Read and discard: flags byte + optional enum + optional long
            let parent_flags = kryo::read_flags_byte(body, &mut pos)?;
            if kryo::is_field_present(parent_flags as u16, 0) {
                let _ = varint::read_unsigned_varint(body, &mut pos)?; // enum ordinal
            }
            if kryo::is_field_present(parent_flags as u16, 1) {
                let _ = varint::read_zigzag_i64(body, &mut pos)?; // id
            }
        }

        Ok(DecodedEvent::TaskStarted(TaskStartedEvent { id, build_path, path, class_name }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_without_parent() {
        // flags=0x10 (bit4=1 → parent absent, bits 0-3 = 0 → present)
        let mut data = vec![0x10]; // flags
        data.push(0x02);           // id: zigzag(1)=2
        data.push(0x02); data.push(58); // buildPath ":"
        // path ":app:compileKotlin" (18 chars) → zigzag(18)=36
        data.push(0x24);
        for &c in b":app:compileKotlin" { data.push(c); }
        // className → zigzag(47)=94
        data.push(0x5e);
        for &c in b"org.jetbrains.kotlin.gradle.tasks.KotlinCompile" { data.push(c); }

        let decoder = TaskStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskStarted(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.path, ":app:compileKotlin");
            assert_eq!(e.class_name.as_deref(), Some("org.jetbrains.kotlin.gradle.tasks.KotlinCompile"));
        } else {
            panic!("expected TaskStarted");
        }
    }
}
```

**Step 2: Run gazelle then test**

```bash
bazel run gazelle
aspect test //build-scan/lib/src/events:task_started_test
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add TaskStarted decoder"
```

---

## Task 9: events/task_finished.rs

**Files:**
- Create: `build-scan/lib/src/events/task_finished.rs`

**Step 1: Implement with test**

```rust
use error::ParseError;
use kryo;
use varint;

use super::{BodyDecoder, DecodedEvent, TaskFinishedEvent};

pub struct TaskFinishedDecoder;

impl BodyDecoder for TaskFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_short(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags, 0) {
            varint::read_zigzag_i64(body, &mut pos)?
        } else { 0 };

        let path = if kryo::is_field_present(flags, 1) {
            table.read_string(body, &mut pos)?
        } else { String::new() };

        let outcome = if kryo::is_field_present(flags, 2) {
            Some(kryo::read_enum_ordinal(body, &mut pos)?)
        } else { None };

        let _skip_message = if kryo::is_field_present(flags, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else { None };

        // bit 4: cacheable (boolean — value IS the bit, no payload)
        let cacheable = if kryo::is_field_present(flags, 4) { Some(true) } else { Some(false) };

        let caching_disabled_reason_category = if kryo::is_field_present(flags, 5) {
            Some(table.read_string(body, &mut pos)?)
        } else { None };

        let caching_disabled_explanation = if kryo::is_field_present(flags, 6) {
            Some(table.read_string(body, &mut pos)?)
        } else { None };

        let origin_build_invocation_id = if kryo::is_field_present(flags, 7) {
            Some(table.read_string(body, &mut pos)?)
        } else { None };

        let origin_build_cache_key = if kryo::is_field_present(flags, 8) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else { None };

        if kryo::is_field_present(flags, 9) {
            // originExecutionTime — read and discard (zigzag long)
            let _ = varint::read_zigzag_i64(body, &mut pos)?;
        }

        // bit 10: actionable (boolean — value IS the bit, no payload)
        let actionable = if kryo::is_field_present(flags, 10) { Some(true) } else { Some(false) };

        if kryo::is_field_present(flags, 11) {
            // upToDateMessages list — read count then skip strings
            let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
            for _ in 0..count {
                let _ = table.read_string(body, &mut pos)?;
            }
        }

        let skip_reason_message = if kryo::is_field_present(flags, 12) {
            Some(table.read_string(body, &mut pos)?)
        } else { None };

        Ok(DecodedEvent::TaskFinished(TaskFinishedEvent {
            id, path, outcome, cacheable,
            caching_disabled_reason_category,
            caching_disabled_explanation,
            origin_build_invocation_id,
            origin_build_cache_key,
            actionable,
            skip_reason_message,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_success_not_cacheable() {
        // flags short: bits 0,1,2 present (0), bit4=1(cacheable=false),
        // bits 3,5-12=1 (absent) → 0b0001_1111_1111_1000 = nah, let me think...
        // bits that are ABSENT (=1): 3,4,5,6,7,8,9,10,11,12
        // bits that are PRESENT (=0): 0,1,2
        // flags = 0b0001_1111_1111_1000 = 0x1FF8
        let mut data = vec![];
        // flags as unsigned varint: 0x1FF8 = 8184
        // LEB128: 8184 & 0x7F = 0x78 | 0x80 = 0xF8, 8184 >> 7 = 63 = 0x3F
        data.push(0xF8); data.push(0x3F);
        data.push(0x02);  // id: zigzag(1)=2
        // path ":app:build" → zigzag(10)=20
        data.push(0x14);
        for &c in b":app:build" { data.push(c); }
        data.push(0x03);  // outcome: ordinal 3 = SUCCESS

        let decoder = TaskFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskFinished(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.path, ":app:build");
            assert_eq!(e.outcome, Some(3)); // SUCCESS
            assert_eq!(e.cacheable, Some(false)); // bit4=1 means absent → false
        } else {
            panic!("expected TaskFinished");
        }
    }
}
```

**Step 2: Run gazelle then test**

```bash
bazel run gazelle
aspect test //build-scan/lib/src/events:task_finished_test
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add TaskFinished decoder with all 13 fields"
```

---

## Task 10: assembly.rs — Correlate Events

**Files:**
- Create: `build-scan/lib/src/assembly.rs`

**Step 1: Implement with test**

```rust
use std::collections::HashMap;

use events::{DecodedEvent, RawEvent};
use framing::FramedEvent;
use models::{BuildScanPayload, RawEventSummary, Task, TaskOutcome};

pub fn assemble(events: Vec<(FramedEvent, DecodedEvent)>) -> BuildScanPayload {
    let mut identities: HashMap<i64, (String, String)> = HashMap::new(); // id → (build_path, task_path)
    let mut started: HashMap<i64, (String, Option<String>, i64)> = HashMap::new(); // id → (class_name, timestamp)
    let mut finished: HashMap<i64, FinishedInfo> = HashMap::new();
    let mut raw_counts: HashMap<u16, usize> = HashMap::new();

    for (frame, decoded) in &events {
        match decoded {
            DecodedEvent::TaskIdentity(e) => {
                identities.insert(e.id, (e.build_path.clone(), e.task_path.clone()));
            }
            DecodedEvent::TaskStarted(e) => {
                started.insert(e.id, (e.build_path.clone(), e.class_name.clone(), frame.timestamp));
            }
            DecodedEvent::TaskFinished(e) => {
                finished.insert(e.id, FinishedInfo {
                    outcome: e.outcome.and_then(TaskOutcome::from_ordinal),
                    cacheable: e.cacheable,
                    caching_disabled_reason: e.caching_disabled_reason_category.clone(),
                    caching_disabled_explanation: e.caching_disabled_explanation.clone(),
                    actionable: e.actionable,
                    timestamp: frame.timestamp,
                });
            }
            DecodedEvent::Raw(r) => {
                *raw_counts.entry(r.wire_id).or_insert(0) += 1;
            }
        }
    }

    let mut tasks: Vec<Task> = identities.into_iter().map(|(id, (build_path, task_path))| {
        let (class_name, started_at) = started.get(&id)
            .map(|(_, cn, ts)| (cn.clone(), Some(*ts)))
            .unwrap_or((None, None));
        let fin = finished.get(&id);
        let finished_at = fin.map(|f| f.timestamp);
        let duration_ms = match (started_at, finished_at) {
            (Some(s), Some(f)) => Some(f - s),
            _ => None,
        };
        Task {
            id, build_path, task_path, class_name,
            outcome: fin.and_then(|f| f.outcome.clone()),
            cacheable: fin.and_then(|f| f.cacheable),
            caching_disabled_reason: fin.and_then(|f| f.caching_disabled_reason.clone()),
            caching_disabled_explanation: fin.and_then(|f| f.caching_disabled_explanation.clone()),
            actionable: fin.and_then(|f| f.actionable),
            started_at, finished_at, duration_ms,
        }
    }).collect();

    tasks.sort_by_key(|t| t.id);

    let mut raw_events: Vec<RawEventSummary> = raw_counts.into_iter()
        .map(|(wire_id, count)| RawEventSummary { wire_id, count })
        .collect();
    raw_events.sort_by_key(|r| r.wire_id);

    BuildScanPayload { tasks, raw_events }
}

struct FinishedInfo {
    outcome: Option<TaskOutcome>,
    cacheable: Option<bool>,
    caching_disabled_reason: Option<String>,
    caching_disabled_explanation: Option<String>,
    actionable: Option<bool>,
    timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::*;

    fn frame(wire_id: u16, ts: i64) -> FramedEvent {
        FramedEvent { wire_id, timestamp: ts, ordinal: 0, body: vec![] }
    }

    #[test]
    fn test_assemble_single_task() {
        let events = vec![
            (frame(117, 1000), DecodedEvent::TaskIdentity(TaskIdentityEvent {
                id: 1, build_path: ":".into(), task_path: ":app:build".into(),
            })),
            (frame(1563, 2000), DecodedEvent::TaskStarted(TaskStartedEvent {
                id: 1, build_path: ":".into(), path: ":app:build".into(),
                class_name: Some("org.gradle.DefaultTask".into()),
            })),
            (frame(2074, 3000), DecodedEvent::TaskFinished(TaskFinishedEvent {
                id: 1, path: ":app:build".into(), outcome: Some(3),
                cacheable: Some(false), caching_disabled_reason_category: None,
                caching_disabled_explanation: None, origin_build_invocation_id: None,
                origin_build_cache_key: None, actionable: Some(false),
                skip_reason_message: None,
            })),
        ];
        let payload = assemble(events);
        assert_eq!(payload.tasks.len(), 1);
        let task = &payload.tasks[0];
        assert_eq!(task.task_path, ":app:build");
        assert_eq!(task.started_at, Some(2000));
        assert_eq!(task.finished_at, Some(3000));
        assert_eq!(task.duration_ms, Some(1000));
        assert!(matches!(task.outcome, Some(TaskOutcome::Success)));
    }
}
```

**Step 2: Run gazelle then test**

```bash
bazel run gazelle
aspect test //build-scan/lib/src:assembly_test
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add assembly layer to correlate task events by ID"
```

---

## Task 11: lib.rs — Top-Level API

**Files:**
- Rewrite: `build-scan/lib/src/lib.rs`

**Step 1: Replace lib.rs with new public API**

```rust
pub mod assembly;
pub mod decompress;
pub mod error;
pub mod events;
pub mod framing;
pub mod kryo;
pub mod models;
pub mod outer_header;
pub mod varint;

use error::ParseError;
use models::BuildScanPayload;

pub fn parse(raw_bytes: &[u8]) -> Result<BuildScanPayload, ParseError> {
    let header = outer_header::OuterHeader::parse(raw_bytes)?;
    let decompressed = decompress::Decompressor::decompress(&raw_bytes[header.gzip_offset..])?;
    let registry = events::DecoderRegistry::new();

    let decoded_events: Result<Vec<_>, _> = framing::EventFrameReader::new(&decompressed)
        .map(|frame_result| {
            let frame = frame_result?;
            let decoded = registry.decode(frame.wire_id, &frame.body)?;
            Ok((frame, decoded))
        })
        .collect();

    Ok(assembly::assemble(decoded_events?))
}
```

**Step 2: Run gazelle then build**

```bash
bazel run gazelle
aspect build //build-scan/lib/src:lib
```

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: add top-level parse() API composing all layers"
```

---

## Task 12: Update CLI

**Files:**
- Modify: `build-scan/cli/src/main.rs`
- Modify: `build-scan/cli/src/BUILD.bazel`

**Step 1: Update CLI to use new parse API**

In `main.rs`, replace the `run_parse` function's step 5 (parse build scan):

```rust
// Old:
// let mut parser = parser::BuildScanParser::new();
// let build_scan = parser.parse_compressed(&raw_bytes)...

// New:
let build_scan = lib::parse(&raw_bytes)
    .context("Failed to parse build scan payload")?;
```

Update imports: replace `use parser::BuildScanParser` with `use lib` (the build-scan lib crate).

Update `BUILD.bazel` dep from `//build-scan/lib/src:parser` to `//build-scan/lib/src:lib`.

**Step 2: Run gazelle then build**

```bash
bazel run gazelle
aspect build //build-scan/cli/src:cli
```

**Step 3: Remove old CLI tests that depend on the old parser API**

The existing CLI tests create minimal gzip blobs with `[0, 0]` (old event format). These need updating or removing since the new parser expects the full outer header + delta-encoded framing.

For now, remove the `happy_path_parses_valid_payload` test (it relied on the old format). The integration test in Task 13 will cover the full pipeline.

**Step 4: Run tests**

```bash
aspect test //build-scan/cli/src:cli_test
```

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: update CLI to use new parse() API"
```

---

## Task 13: Integration Test with Reference Payload

**Files:**
- Create: `build-scan/lib/tests/integration_test.rs` (or add to existing test)

**Step 1: Write integration test**

This test loads the actual reference payload JSON and verifies the full pipeline:

```rust
use std::path::Path;

#[test]
fn test_parse_reference_payload() {
    let payload_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.json");

    // If payload file doesn't exist, skip (CI might not have it)
    if !payload_path.exists() {
        eprintln!("Skipping integration test: reference payload not found");
        return;
    }

    let contents = std::fs::read_to_string(&payload_path).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&contents).unwrap();
    let b64 = payload["request"]["body"]["base64"].as_str().unwrap();
    let raw_bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();

    let result = lib::parse(&raw_bytes).unwrap();

    // Reference payload has 45 tasks
    assert_eq!(result.tasks.len(), 45, "expected 45 tasks, got {}", result.tasks.len());

    // Check known task paths exist
    let paths: Vec<&str> = result.tasks.iter().map(|t| t.task_path.as_str()).collect();
    assert!(paths.contains(&":app:compileKotlin"), "missing :app:compileKotlin");
    assert!(paths.contains(&":app:build"), "missing :app:build");
    assert!(paths.contains(&":list:compileKotlin"), "missing :list:compileKotlin");

    // All tasks should have timing info
    for task in &result.tasks {
        assert!(task.started_at.is_some(), "task {} missing started_at", task.task_path);
        assert!(task.finished_at.is_some(), "task {} missing finished_at", task.task_path);
    }

    // Raw events should be populated (most events are undecoded)
    assert!(!result.raw_events.is_empty());
}
```

Note: This test may need adjustment depending on exact Bazel test data paths. Use `data` attribute in BUILD.bazel to include the payload file, or use a relative path from the test binary's runfiles.

**Step 2: Run test**

```bash
aspect test //build-scan/lib/tests:integration_test
```

The number 45 is approximate (from the Obsidian notes saying "45x TASK_STARTED_v6"). Adjust if the actual count differs.

**Step 3: Commit**

```bash
git add -A && git commit -m "test: add integration test with reference payload"
```

---

## Task 14: BUILD.bazel + Gazelle + Format

**Step 1: Run gazelle to regenerate all BUILD files**

```bash
bazel run gazelle
```

**Step 2: Format all files**

```bash
bazel run //tools/format
```

**Step 3: Run all tests**

```bash
aspect test //...
```

**Step 4: Fix any issues, then final commit**

```bash
git add -A && git commit -m "chore: regenerate BUILD files and format"
```

---

## Summary

| Task | Component | Key Files |
|------|-----------|-----------|
| 1 | Cleanup + errors | error.rs |
| 2 | Varint encoding | varint.rs |
| 3 | Outer header | outer_header.rs |
| 4 | Event framing | framing.rs |
| 5 | Kryo primitives | kryo.rs |
| 6 | Output models | models.rs |
| 7 | Trait + TaskIdentity | events/mod.rs, events/task_identity.rs |
| 8 | TaskStarted | events/task_started.rs |
| 9 | TaskFinished | events/task_finished.rs |
| 10 | Assembly | assembly.rs |
| 11 | Top-level API | lib.rs |
| 12 | CLI update | cli/src/main.rs |
| 13 | Integration test | tests/integration_test.rs |
| 14 | Build + format | BUILD.bazel files |
