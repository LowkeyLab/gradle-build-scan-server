# Event ID Mapping Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Identify event IDs for OS name/version, JVM version, Gradle version, and at least 3 task-related fields by correlating the binary payload with the Gradle build scan web UI.

**Architecture:** Fresh payload is captured via the proxy server, the scan URL is opened in Chrome MCP for ground-truth cataloging, and the decompressed binary is searched for known string values to anchor event ID identification.

**Tech Stack:** Bash (capture.sh), Chrome DevTools MCP (UI inspection), Python (binary analysis), Rust build-scan CLI (existing parser output).

---

### Task 1: Capture Fresh Payload

**Files:**
- Read: `captured-output/gradle-build-output.log` (after run — extract scan URL)
- Read: `captured-output/payloads/*.json` (binary upload file)

**Step 1: Run the capture script**

```bash
./.opencode/skills/capturing-gradle-payloads/capture.sh
```

Expected: Script completes without error. Three JSON files appear in `captured-output/payloads/`.

**Step 2: Extract the scan URL**

Look in `captured-output/gradle-build-output.log` for a line like:
```
Publishing build scan...
https://gradle.com/s/<id>
```

Record the URL — this is used in Task 2.

**Step 3: Identify the binary upload payload file**

```bash
ls -1 captured-output/payloads/
```

The file with the latest timestamp and whose `request.uri` contains `/upload` is the binary payload. Confirm:

```bash
python3 -c "
import json, os, glob
files = sorted(glob.glob('captured-output/payloads/*.json'))
for f in files:
    d = json.load(open(f))
    print(f, d['request']['method'], d['request']['uri'])
"
```

Expected: Three files, one with `POST /scans/publish/gradle/4.3.2/upload`.

**Step 4: Commit**

```bash
git add captured-output/
git commit -m "chore: refresh captured payloads for event-id mapping session"
```

---

### Task 2: Catalog Chrome UI Fields

**Tools:** Chrome DevTools MCP

**Step 1: Open the scan URL**

Use Chrome MCP `navigate_page` to open the scan URL from Task 1, Step 2.

**Step 2: Take a snapshot and catalog summary fields**

Use `take_snapshot` on the loaded page. Record:
- Project name
- Build outcome (SUCCESS / FAILED)
- Build duration (e.g., `1m 23s`)
- Gradle version
- Date/time of build

**Step 3: Navigate to the Environment tab**

Click the "Environment" or "Infrastructure" tab. Record:
- Operating system name (e.g., `Linux`)
- OS version (e.g., `6.13.1-arch1-1`)
- JVM version (e.g., `25.0.2`)
- JVM vendor (e.g., `Oracle Corporation`)
- Username
- Max memory

**Step 4: Navigate to the Timeline / Tasks tab**

Record the first 5 task entries:
- Task path (e.g., `:app:compileKotlin`)
- Task outcome (e.g., `UP-TO-DATE`, `SUCCESS`)
- Task duration (e.g., `23ms`)

**Step 5: Note unique string anchors**

From the collected values, identify strings that are:
- Long enough to be unambiguous in a binary search (>4 chars)
- Likely to appear literally in the payload (not integers)

Good anchors: OS name, JVM vendor, task paths, project name.

---

### Task 3: Decompress Binary Payload and Search for Anchors

**Files:**
- Read: `captured-output/payloads/<upload-file>.json`
- Create: `analyze_event_ids.py` (temporary analysis script)

**Step 1: Write the analysis script**

Create `analyze_event_ids.py`:

```python
#!/usr/bin/env python3
"""Search for string anchors in the decompressed Gradle build scan payload."""

import base64
import gzip
import json
import sys
import glob

def load_upload_payload():
    """Find and load the binary upload payload file."""
    files = sorted(glob.glob("captured-output/payloads/*.json"))
    for f in files:
        d = json.load(open(f))
        if "/upload" in d["request"]["uri"]:
            return f, d
    raise ValueError("No upload payload found")

def decode_body(body):
    """Decode the request body (base64 or plain string)."""
    if isinstance(body, dict) and "base64" in body:
        return base64.b64decode(body["base64"])
    return body.encode()

def find_gzip_offset(data: bytes) -> int:
    """Find the offset of the gzip magic bytes."""
    magic = b'\x1f\x8b\x08'
    idx = data.find(magic)
    if idx == -1:
        raise ValueError("No gzip magic bytes found")
    return idx

def read_leb128(data: bytes, pos: int):
    """Read a LEB128 varint starting at pos. Returns (value, new_pos)."""
    result = 0
    shift = 0
    while True:
        b = data[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            break
        shift += 7
    return result, pos

def search_string(data: bytes, target: str, context_before: int = 32):
    """Find all occurrences of target in data and show context bytes before each."""
    target_bytes = target.encode('utf-8')
    results = []
    start = 0
    while True:
        idx = data.find(target_bytes, start)
        if idx == -1:
            break
        before_start = max(0, idx - context_before)
        context = data[before_start:idx + len(target_bytes)]
        results.append({
            "offset": idx,
            "context_hex": context.hex(),
            "context_bytes": list(context),
            "string": target,
        })
        start = idx + 1
    return results

def main():
    path, payload = load_upload_payload()
    print(f"Loaded: {path}")

    body = decode_body(payload["request"]["body"])
    gz_offset = find_gzip_offset(body)
    print(f"Gzip starts at offset: {gz_offset}")

    decompressed = gzip.decompress(body[gz_offset:])
    print(f"Decompressed size: {len(decompressed)} bytes")
    print(f"First 64 bytes (hex): {decompressed[:64].hex()}")

    # Search for each anchor string passed as command-line arguments
    anchors = sys.argv[1:] if len(sys.argv) > 1 else ["linux"]
    for anchor in anchors:
        hits = search_string(decompressed, anchor)
        print(f"\n=== Anchor: '{anchor}' ===")
        for hit in hits:
            print(f"  Offset: {hit['offset']}")
            print(f"  Context hex ({len(hit['context_bytes'])} bytes before+string): {hit['context_hex']}")

if __name__ == "__main__":
    main()
```

**Step 2: Run with OS name as first anchor**

```bash
python3 analyze_event_ids.py "Linux"
```

Expected: One or more hits showing the offset and context bytes before the string "Linux".

**Step 3: Run with JVM vendor and task path anchors**

```bash
python3 analyze_event_ids.py "Oracle Corporation" ":app:compileKotlin"
```

(Use the actual values from Task 2.)

**Step 4: For each anchor hit, decode the preceding LEB128 bytes**

For each hit, look at the bytes immediately before the string's length-varint. Those bytes are the event ID LEB128 varint. Add a decoder:

```python
# Add to analyze_event_ids.py or run inline:
# Given: context_bytes = [..., <event_id_leb128_bytes>, <length_varint>, <string_bytes>]
# Walk backwards from the string position to find the event ID

def decode_varint_at(data, pos):
    """Decode LEB128 at pos, return (value, bytes_consumed)."""
    result = 0
    shift = 0
    consumed = 0
    while True:
        b = data[pos + consumed]
        consumed += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            break
        shift += 7
    return result, consumed
```

The structure at each string hit is:
```
[event_id: LEB128] [length_varint: LEB128] [string_bytes]
```

Where `length_varint = (len << 1) | is_ref`.

So: decode the length varint first (works backwards from the string offset), then keep decoding backwards until we hit the event ID varint that preceded it.

**Step 5: Build preliminary mapping table**

For each identified event ID, record:
```
Event ID: <id>
Offset in stream: <offset>
String value produced: "<value>"
Meaning (from Chrome UI): "<section> -> <field>"
```

---

### Task 4: Run Existing CLI and Compare

**Files:**
- Existing: `build-scan/cli/` (Rust CLI binary)

**Step 1: Build and run the CLI against the new payload**

```bash
aspect build //build-scan/cli:cli
./bazel-bin/build-scan/cli/cli captured-output/payloads/<upload-file>.json 2>&1 | head -80
```

Expected: Either partial output (then an `UnknownEvent` error showing the first unknown ID), or full output if the payload only uses known IDs.

**Step 2: Note where the CLI fails**

If it fails with `UnknownEvent { id: X }`, record X. This is the next event ID to investigate.

**Step 3: Cross-reference with Python analysis**

Search the decompressed stream for the bytes of event ID X (as LEB128). Look at what follows it — is it a string? A varint? A timestamp?

```bash
python3 -c "
data = open('/tmp/decompressed.bin', 'rb').read()  # save decompressed in step above
# encode X as LEB128 and find it
x = <unknown_id>
leb = []
while x >= 0x80:
    leb.append((x & 0x7F) | 0x80)
    x >>= 7
leb.append(x)
leb_bytes = bytes(leb)
idx = data.find(leb_bytes)
print(f'Found at: {idx}')
print(f'Next 32 bytes: {data[idx:idx+32].hex()}')
"
```

---

### Task 5: Update Binary Format Documentation

**Files:**
- Modify: `docs/gradle-build-scan-binary-format.md`

**Step 1: Open the existing format doc**

Read `docs/gradle-build-scan-binary-format.md`.

**Step 2: Add the new event ID table entries**

For each newly identified event ID, add a row to the event ID table:

```markdown
| <id> | <data type> | <meaning> | <example value> |
```

**Step 3: Add a "Reverse-Engineering Session Notes" section**

Document:
- The Gradle scan URL used
- Gradle + plugin versions from this capture
- String anchors used and where they appeared
- Any open questions (event IDs still unknown)

**Step 4: Commit**

```bash
git add docs/gradle-build-scan-binary-format.md analyze_event_ids.py
git commit -m "docs: update event-id mapping table from Chrome-first reverse-engineering session"
```

---

## Verification Checklist

Before marking this plan complete, confirm:

- [ ] `captured-output/payloads/` contains 3 fresh JSON files from today
- [ ] The scan URL is live and was opened in Chrome
- [ ] At least the following event IDs are identified with meanings:
  - OS name field
  - JVM version field
  - At least 3 task-related event IDs (task path, outcome, duration)
- [ ] `docs/gradle-build-scan-binary-format.md` updated with new mappings
- [ ] All new findings committed to git
