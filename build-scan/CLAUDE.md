# Build Scan

Core parsing library and CLI for decoding Gradle build scan binary payloads.

## Modules

### Library (`lib/`)

Decodes the Gradle build scan wire format: outer header → gzip decompression → delta-encoded event frames → Kryo body decoding → structured `BuildScanPayload`.

```bash
aspect test //build-scan/lib/...   # Run library tests
```

#### Parsing Pipeline

```
Raw Bytes → OuterHeader → Gzip → Framing (wire_id + body) → Event Decoding → Assembly → BuildScanPayload
```

#### Key Source Files

| File | Purpose |
|------|---------|
| `lib.rs` | Entry point: `parse(raw_bytes) → BuildScanPayload` |
| `outer_header.rs` | Parses outer header, finds gzip offset |
| `decompress.rs` | Gzip decompression |
| `framing.rs` | Reads delta-encoded event frames |
| `varint.rs` | LEB128 varint + ZigZag encoding |
| `kryo.rs` | Kryo serialization helpers (per-event string interning) |
| `events/mod.rs` | DecoderRegistry — 40+ wire_id → BodyDecoder mappings |
| `assembly.rs` | Maps decoded events into `BuildScanPayload` |
| `models.rs` | Data structures (Task, TestCase, TaskOutcome, etc.) |

### CLI (`cli/`)

Standalone tool that reads an echo-server JSON payload file, decodes the base64 build scan body, and prints parsed JSON.

```bash
bazel run //build-scan/cli/src:main -- /path/to/payload.json
```

### Server (`server/`)

See [`server/CLAUDE.md`](server/CLAUDE.md) for the HTTP server, database, GraphQL API, and ingest protocol.
