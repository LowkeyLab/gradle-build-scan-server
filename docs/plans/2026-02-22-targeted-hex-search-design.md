# Targeted Hex Search Discovery Design

**Date:** 2026-02-22  
**Status:** Approved  
**Goal:** Execute Phase 1 (Chrome-First Discovery Session) of the Task-Centric Parser plan by using targeted hex searching to map Gradle build scan event IDs.

## Context
The Gradle Build Scan binary payload is a strict stream of LEB128 tokens without length prefixes for most events (as noted in Obsidian). This means getting out of sync causes the rest of the parsing to fail. The current parser fails at early unknown events (like Event ID 12). Instead of building a perfect parser from start to finish via trial and error ("whack-a-mole"), we will use a targeted search approach to jump directly to the events we care about.

## 1. Tooling: Context-Aware Searcher (`hex_search.py`)
We will create a Python script that bypasses the need for full parsing by searching the raw binary payload.
- Takes a target value (a string like `:app:compileKotlin` or an integer for a duration/outcome enum).
- Finds exact byte offsets in the `.bin` payload.
- Prints a rich context block (e.g., 20 bytes before and 40 bytes after).
- Annotates valid LEB128 varints and strings within that window to help us trace **backward** to the 1-byte Event ID that precedes the data.

## 2. Discovery Workflow
According to the official event model, we need to map `TaskStarted` and `TaskFinished` events.

### Step A: Task Paths (`TaskStarted`)
1. Search for a known task path string (e.g., `:app:compileKotlin`).
2. We previously saw Event ID `58` associated with task paths. The official model expects: `id`, `buildPath`, `path`, `className`, `thread`.
3. We will inspect the bytes around the task path to see if it matches that exact primitive signature.

### Step B: Task Outcomes (`TaskFinished`)
1. Look at the `gradle-build-output.log` (or Chrome UI) to identify tasks that were `UP-TO-DATE` or `SKIPPED`.
2. Search for the task path in the hex, then look nearby for the corresponding `TaskFinished` event.
3. Alternatively, search for small varints (0-6) representing the outcome enum values in the `TaskFinished` event near the task path string.

### Step C: Task Duration
1. Use the Chrome UI to find a specific task's duration (e.g., `43ms`).
2. Search the hex for the varint value `43`.
3. Observe what Event ID is nearby to correlate it to `originExecutionTime` in `TaskFinished`.

## 3. Output
The output of this discovery session will be the exact mapped byte patterns and Event IDs for `TaskStarted` and `TaskFinished`. These findings will be used to update the `parser.rs` constants and the `PayloadBuilder` state machine, enabling the parser to extract full task telemetry.