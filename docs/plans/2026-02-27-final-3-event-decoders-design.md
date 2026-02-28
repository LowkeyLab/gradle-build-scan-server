
# Design: Decode Final 3 Event Types

**Date:** 2026-02-27
**Status:** Approved

## Goal

Decode the remaining 3 undecoded event types in the Gradle build scan parser to achieve 100% event coverage. All 3 are singleton events (1x per build scan).

## Events

### 1. TaskRegistrationSummary (wire 122, 1 byte body)

- **Java model:** `TaskRegistrationSummary_1_0 { int taskCount }`
- **Wire format (inferred):** Single positive varint, no flags byte. 1-byte body = value 0–127.
- **Rust decoder:** Read one `read_positive_varint_i32`, return `TaskRegistrationSummaryEvent { task_count: i32 }`
- **Output model:** `TaskRegistrationSummaryData { task_count: i32 }` on `BuildScanPayload`

### 2. BasicMemoryStats_1_1 (wire 257, 316 bytes body)

- **Java model:** `BasicMemoryStats_1_1 { long free, long total, long max, List<MemoryPoolSnapshot_1_0> peakSnapshots, long gcTime }`
- **Nested:** `MemoryPoolSnapshot_1_0 { String name, boolean heap, long init, long used, long committed, long max }`
- **Wire format:** Requires decompilation of Kryo serializer to determine exact encoding (flags byte, field ordering, list encoding, nested snapshot format)
- **Rust decoder:** Flags byte + conditional reads for each field. List of snapshots with per-snapshot interned string + bool + 4 longs.
- **Output model:** `BasicMemoryStatsData { free, total, max, peak_snapshots: Vec<MemoryPoolSnapshotData>, gc_time }` on `BuildScanPayload`
- **MemoryPoolSnapshotData:** `{ name: String, heap: bool, init: i64, used: i64, committed: i64, max: i64 }`

### 3. ResourceUsage_2_0 (wire 407, 37 bytes body)

- **Java model:** 16 fields — timestamps, 7 NormalizedSamples, 1 raw byte array, Long totalSystemMemory, List processes, 2 IndexedNormalizedSamples
- **Wire format:** Single u8 flags byte (4 bits used for conditional fields: timestamps, allProcessesCpu, totalSystemMemory, processes); remaining 12 sub-structures are unconditional (each has its own internal flags).
- **Rust decoder:** Read u8 flags, then interleaved conditional/unconditional reads for each field
- **Nested types:**
  - `NormalizedSamples { samples: Vec<u8>, max: i64 }` — `samples` is a length-prefixed raw byte array
  - `IndexedNormalizedSamples { indices: Vec<Vec<i32>>, samples: Vec<Vec<u8>>, max: i64 }` — `indices` is nested varint-encoded int lists, `samples` is a list of length-prefixed raw byte arrays
  - `ProcessInfo { id: i64, name: String, display_name: String, process_type: ProcessType }`
  - `ProcessType` enum: Self, Descendant, Other
- **Output model:** `ResourceUsageData` with all fields as `Option<...>` on `BuildScanPayload`

## Approach

- **TaskRegistrationSummary:** Infer wire format from model (trivial 1-byte body)
- **BasicMemoryStats & ResourceUsage:** Decompile Kryo serializer classes from Develocity plugin JAR to determine exact encoding

## Assembly

All three are singleton events. First occurrence of each stored directly on `BuildScanPayload`:
- `task_registration_summary: Option<TaskRegistrationSummaryData>`
- `basic_memory_stats: Option<BasicMemoryStatsData>`
- `resource_usage: Option<ResourceUsageData>`

## Files to Modify

1. `build-scan/lib/src/events/mod.rs` — Add 3 `DecodedEvent` variants + event structs + registry entries
2. `build-scan/lib/src/events/task_registration_summary.rs` — New decoder
3. `build-scan/lib/src/events/basic_memory_stats.rs` — New decoder
4. `build-scan/lib/src/events/resource_usage.rs` — New decoder
5. `build-scan/lib/src/models.rs` — Add output data structs + fields on `BuildScanPayload`
6. `build-scan/lib/src/assembly.rs` — Assembly logic for new events
