# Targeted Hex Search Discovery Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a context-aware hex search Python script to trace byte offsets of known task telemetry strings/varints and manually map Event IDs for TaskStarted/TaskFinished.

**Architecture:** A standalone Python utility (`search_hex_context.py`) that reads `.bin` Gradle payload files, searches for exact byte sequences (strings or encoded LEB128 varints), and dumps annotated hex output.

**Tech Stack:** Python 3 (sys, struct, math for LEB128 encoding)

---

## Phase 1: Tooling

### Task 1: Basic String Hex Searcher

**Files:**
- Create: `search_hex_context.py`

**Step 1: Write the failing test**
No test framework, but we will write a script that takes a file and string argument and prints "Not implemented".

```python
# search_hex_context.py
import sys

def main():
    if len(sys.argv) < 3:
        print("Usage: python3 search_hex_context.py <file.bin> <string_to_search>")
        sys.exit(1)
    
    print("Not implemented")

if __name__ == "__main__":
    main()
```

**Step 2: Run to verify it "fails" (prints Not implemented)**
Run: `python3 search_hex_context.py captured-output/payloads/out.bin ":app:compileKotlin"`
Expected: Prints `Not implemented`

**Step 3: Implement minimal string search**
```python
import sys

def main():
    if len(sys.argv) < 3:
        print("Usage: python3 search_hex_context.py <file.bin> <string_to_search>")
        sys.exit(1)
        
    file_path = sys.argv[1]
    target_string = sys.argv[2]
    
    with open(file_path, 'rb') as f:
        data = f.read()
        
    target_bytes = target_string.encode('utf-8')
    idx = 0
    matches = 0
    while True:
        idx = data.find(target_bytes, idx)
        if idx == -1:
            break
            
        print(f"Match found at offset {idx}")
        start = max(0, idx - 30)
        end = min(len(data), idx + len(target_bytes) + 50)
        chunk = data[start:end]
        
        hex_str = ' '.join(f'{b:02x}' for b in chunk)
        print(f"HEX:  {hex_str}")
        
        idx += 1
        matches += 1
        
    if matches == 0:
        print(f"String '{target_string}' not found.")

if __name__ == "__main__":
    main()
```

**Step 4: Run to verify it passes**
Run: `python3 search_hex_context.py captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.bin ":app:compileKotlin"`
Expected: Prints offset and a hex dump chunk.

**Step 5: Commit**
```bash
git add search_hex_context.py
git commit -m "feat: add basic hex string searcher"
```

---

### Task 2: LEB128 Varint Encoding Search

**Files:**
- Modify: `search_hex_context.py`

**Step 1: Write the failing test**
Add support for a `--varint` flag to search for numeric LEB128 varints instead of strings.

**Step 2: Implement minimal varint encoding**
```python
import sys

def encode_leb128(value):
    result = bytearray()
    while True:
        byte = value & 0x7f
        value >>= 7
        if value != 0:
            byte |= 0x80
        result.append(byte)
        if value == 0:
            break
    return bytes(result)

def main():
    if len(sys.argv) < 3:
        print("Usage: python3 search_hex_context.py <file.bin> <target> [--varint]")
        sys.exit(1)
        
    file_path = sys.argv[1]
    target_raw = sys.argv[2]
    is_varint = "--varint" in sys.argv
    
    with open(file_path, 'rb') as f:
        data = f.read()
        
    if is_varint:
        target_bytes = encode_leb128(int(target_raw))
        print(f"Searching for LEB128 encoded value: {' '.join(f'{b:02x}' for b in target_bytes)}")
    else:
        target_bytes = target_raw.encode('utf-8')
        
    idx = 0
    matches = 0
    while True:
        idx = data.find(target_bytes, idx)
        if idx == -1:
            break
            
        print(f"Match found at offset {idx}")
        start = max(0, idx - 30)
        end = min(len(data), idx + len(target_bytes) + 50)
        chunk = data[start:end]
        
        hex_str = ' '.join(f'{b:02x}' for b in chunk)
        print(f"HEX:  {hex_str}")
        
        idx += 1
        matches += 1
        
    if matches == 0:
        print(f"Target not found.")

if __name__ == "__main__":
    main()
```

**Step 3: Run to verify**
Run: `python3 search_hex_context.py captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.bin 58 --varint`
Expected: Finds the byte `0x3a` encoding for 58.

**Step 4: Commit**
```bash
git add search_hex_context.py
git commit -m "feat: support searching for LEB128 encoded varints"
```

---

## Phase 2: Execution

### Task 3: Identify TaskStarted Event

**Step 1: Search for known string**
Run: `python3 search_hex_context.py captured-output/payloads/20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.bin ":app:compileKotlin"`

**Step 2: Map backward to find Event ID**
Look at the 10 bytes preceding the string length byte to find the 1-byte Event ID. Identify the exact sequence of properties (`id`, `buildPath`, `path`, etc.). Write down the hypothesis in the terminal output.

### Task 4: Identify TaskFinished Event

**Step 1: Check build log for UP-TO-DATE task**
Run: `grep UP-TO-DATE captured-output/gradle-build-output.log | head -n 1`
Expected: Gets task path like `:list:compileKotlin`

**Step 2: Search for that task in payload**
Run: `python3 search_hex_context.py <latest_payload.bin> ":list:compileKotlin"`
Expected: Multiple hits. One is `TaskStarted`, the other is `TaskFinished`. Look for outcome varint (`0-6`) around the second hit.

### Task 5: Identify Task Duration

**Step 1: Find a duration from the Chrome UI (or guess)**
If Chrome UI is unavailable, build duration is found in the log (`BUILD SUCCESSFUL in 3s`). `TaskFinished` has `originExecutionTime`.

**Step 2: Search for duration as varint**
Run: `python3 search_hex_context.py <latest_payload.bin> <duration_ms> --varint`
Correlate to see if it matches the `TaskFinished` event mapped in Task 4.

### Task 6: Document Event IDs

**Files:**
- Modify: `docs/plans/2026-02-21-task-centric-parser-design.md`

**Step 1: Write Findings**
Update the "Known Event IDs" table with `TASK_STARTED` and `TASK_FINISHED` and their exact payload structure discovered.

**Step 2: Commit**
```bash
git add docs/plans/2026-02-21-task-centric-parser-design.md
git commit -m "docs: map TaskStarted and TaskFinished event IDs and structure"
```