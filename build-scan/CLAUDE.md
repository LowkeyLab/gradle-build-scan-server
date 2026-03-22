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

| File              | Purpose                                                 |
| ----------------- | ------------------------------------------------------- |
| `lib.rs`          | Entry point: `parse(raw_bytes) → BuildScanPayload`      |
| `outer_header.rs` | Parses outer header, finds gzip offset                  |
| `decompress.rs`   | Gzip decompression                                      |
| `framing.rs`      | Reads delta-encoded event frames                        |
| `varint.rs`       | LEB128 varint + ZigZag encoding                         |
| `kryo.rs`         | Kryo serialization helpers (per-event string interning) |
| `events/mod.rs`   | DecoderRegistry — 40+ wire_id → BodyDecoder mappings    |
| `assembly.rs`     | Maps decoded events into `BuildScanPayload`             |
| `models.rs`       | Data structures (Task, TestCase, TaskOutcome, etc.)     |

#### Event Decoder Gotchas

- **TestCase/TestResult pairing**: Events are strictly interleaved 1:1 (each TestCase followed by its TestResult). Assembly uses **sequential positional pairing**, not ID-based matching — the ID spaces are different between these event types.
- **TestFinished_1_1 (wire 284)**: The `failed`/`skipped` booleans are encoded **in the flags byte** (bits 2, 3), not as separate data fields. The `task` and `id` fields use fixed 8-byte LE encoding (`read_task_id`), not Kryo-long.
- **Cached test tasks**: When Gradle test tasks resolve FROM-CACHE/UP-TO-DATE, **no test events are emitted**. Use `./gradlew clean test --rerun --no-build-cache` to force test execution and generate test events.

### CLI (`cli/`)

Standalone tool that reads an echo-server JSON payload file, decodes the base64 build scan body, and prints parsed JSON.

```bash
bazel run //build-scan/cli/src:main -- /path/to/payload.json
```

### Server (`server/`)

See [`server/CLAUDE.md`](server/CLAUDE.md) for the HTTP server, database, GraphQL API, and ingest protocol.
