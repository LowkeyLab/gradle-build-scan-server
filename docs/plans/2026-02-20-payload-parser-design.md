# Gradle Build Scan Payload Parser (Rust)

## Overview

This library provides a fast, safe, in-memory parser for the proprietary binary format used by the Gradle Build Scan plugin. It consumes raw byte payloads (comprising a cleartext header and a gzip-compressed LEB128 binary stream) and outputs strongly-typed, JSON-serializable Rust structures. 

Since the binary format is schema-less and bespoke, the parser relies heavily on heuristics derived from known event structures.

## Core Design Decisions

1.  **In-Memory Processing**: The library will load the entire uncompressed byte slice into memory before parsing. This optimizes for simplicity and speed, as typical build scan payloads are well within memory limits.
2.  **Two-Pass Decoding**: The architecture uses a two-phase approach:
    *   **Pass 1 (Lexing)**: A stream decoder uncompresses the gzip block and iterates through the bytes, yielding intermediate `Primitive` types (varints, strings, timestamps, string references).
    *   **Pass 2 (Semantic Mapping)**: A stateful payload builder consumes the stream of primitives, maintains an interned string dictionary, and constructs the final strongly-typed objects based on known event IDs.
3.  **Strict Error Handling (Fail-Fast)**: Using `thiserror`, the parser will immediately fail and return an error if it encounters an unknown event schema or an unexpected primitive type. This strictness ensures the integrity of the generated strongly-typed data.

## Architecture & Modules

The library is organized into three core modules:

### 1. `primitives`

Responsible for low-level byte manipulation and decompression.

*   `StreamDecoder`: A struct that takes the raw `&[u8]`. It strips the 28-byte cleartext header, decompresses the underlying Gzip stream, and provides an `Iterator` yielding `Result<Primitive, ParseError>`.
*   Handles LEB128 decoding and the custom string bit-shifting logic (`length = varint >> 1`, flag = `varint & 1`).

```rust
pub enum Primitive {
    Varint(u64),
    String(String),
    StringRef(u32),
    Timestamp(DateTime<Utc>),
}
```

### 2. `parser`

Contains the stateful logic to assemble the payload.

*   `PayloadBuilder`: The core struct that drives the parsing. It consumes the `StreamDecoder`, maintains a `Vec<String>` for back-reference resolution, and acts as a state machine matching primitive sequences against known Event IDs.
*   Resolves `StringRef(index)` primitives by looking them up in its internal dictionary.

### 3. `models`

Defines the final, strongly-typed domain objects that the library outputs. These structs should derive `serde::Serialize` and `serde::Deserialize`.

```rust
pub struct BuildScanPayload {
    pub metadata: Metadata,
    pub environment: Environment,
    pub tasks: Vec<TaskExecution>,
    // ...other structures as discovered
}
```

## Error Handling

The library leverages `thiserror` for clean, idiomatic error definitions.

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
    #[error("Unexpected primitive type: expected {expected}, found {found:?}")]
    UnexpectedPrimitive { expected: &'static str, found: Primitive },
    #[error("Unknown or unhandled event schema ID: {0}")]
    UnknownEventSchema(u32),
    #[error("Invalid string reference: index {0} not found in dictionary")]
    InvalidStringRef(u32),
}
```

## Data Flow

1.  Client provides raw bytes (`&[u8]`).
2.  `StreamDecoder` extracts the header, decompresses the gzip payload, and begins yielding `Primitive` enums.
3.  `PayloadBuilder` loops through the primitives.
4.  When a new string is parsed (flag `0`), it is pushed to the builder's dictionary `Vec`.
5.  When a known Event ID is encountered, the builder consumes the exact expected sequence of subsequent primitives to populate a strongly-typed model (e.g., `TaskExecution`).
6.  If an expected primitive is missing, or an unknown Event ID is hit, a `ParseError` is returned immediately.
7.  Upon reaching EOF, the fully constructed `BuildScanPayload` is returned.
