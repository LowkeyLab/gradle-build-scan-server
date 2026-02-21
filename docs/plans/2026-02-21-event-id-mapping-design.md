# Design: Chrome-first Build Scan Event ID Mapping

**Date:** 2026-02-21  
**Goal:** Expand the known event-ID mapping in `build-scan/lib/src/parser.rs` by correlating binary payload bytes with the data visible in the Gradle build scan web UI.

---

## Context

The binary upload payload (`POST /scans/publish/gradle/4.3.2/upload`) is a schema-dependent LEB128 event stream. Each **event ID** (encoded as a LEB128 varint) dictates the exact sequence and type of primitives that follow. Because the format is not self-describing, the parser cannot make progress on unknown event IDs without knowing the schema.

Known event IDs (from previous session):

| Event ID | Data type | Meaning |
|---|---|---|
| 0 | Timestamp | Build timestamp (ms since epoch) |
| 1 | Varint | Unknown |
| 2 | Varint | Unknown |
| 4 | Varint | Unknown |
| 10 | Varint | Unknown |
| 12 | Varint | Unknown |
| 14 | String/StringRef | String dictionary insertion |
| 10800000 | Varint | Unknown |
| 3543246354218 | Varint | Unknown |

---

## Approach: Chrome-first Top-down Correlation

### Phase 1 — Fresh Payload Capture

Run the automated capture script:

```bash
./.opencode/skills/capturing-gradle-payloads/capture.sh
```

This produces:
- `captured-output/payloads/*.json` — 3 JSON files (user-check, token-request, binary upload)
- `captured-output/gradle-build-output.log` — contains the live `https://gradle.com/s/<id>` URL
- `captured-output/echo-server-output.log` — proxy trace

The binary upload payload (the `POST /scans/publish/gradle/4.3.2/upload` file) is the target for analysis.

### Phase 2 — Chrome UI Cataloging

Open the scan URL in Chrome and document every visible value across all tabs:

| UI Section | Fields to capture |
|---|---|
| Summary | Build duration, build outcome, Gradle version, project name |
| Performance | Task counts, build phases, test count |
| Timeline | Task names, task outcomes (UP-TO-DATE, SUCCESS, FAILED), task durations |
| Environment | OS name+version, JVM version, username, hostname, CI flag |
| Dependencies | Group/artifact IDs, versions |

Each value becomes a search anchor in the binary.

### Phase 3 — Binary Cross-reference

1. Base64-decode the upload payload body
2. Skip the 28-byte GRADLE cleartext header; find the gzip magic bytes `\x1f\x8b\x08`
3. Decompress the gzip stream → raw LEB128 event stream
4. Search the decompressed bytes for UTF-8 string values from Chrome (e.g., `"linux"`, task names like `:app:compileKotlin`, JVM version string)
5. For each found string: scan backwards in the stream to identify the preceding LEB128 varint → that is the event ID that produced this value
6. Record: `event ID → produced value → meaning from Chrome UI`

### Phase 4 — Mapping Table Update

Update `docs/gradle-build-scan-binary-format.md` with:
- Newly discovered event IDs and their meanings
- Example values from the captured payload
- Data type (string, varint, timestamp, string-ref)

---

## Success Criteria

The session is complete when we have identified event IDs for at least:
- OS name/version
- JVM version
- Gradle version (plugin + build tool)
- At least 3 task-related event IDs (task name, task outcome, task duration)

---

## Files Affected

| File | Change |
|---|---|
| `captured-output/` | Replaced with fresh payload from new `capture.sh` run |
| `docs/gradle-build-scan-binary-format.md` | Updated with new event-ID mappings |
| `build-scan/lib/src/parser.rs` | (Out of scope for this design — implementation plan handles this) |
