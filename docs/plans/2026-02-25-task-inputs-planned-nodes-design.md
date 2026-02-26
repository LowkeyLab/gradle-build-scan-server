# Task Inputs + Planned Nodes Event Decoders Design

**Status:** Approved  
**Date:** 2026-02-25  
**Supersedes:** None (extends existing parser)

## Goal

Decode the next 9 most frequent event types (plus 3 nested types) from the Gradle build scan payload, organized into two semantic clusters:
- **Cluster 1 — Task Inputs** (7 event types): file property roots, file properties, implementation, property names, value properties, snapshotting start/finish
- **Cluster 2 — Planned Nodes** (2 event types): planned node graph, transform execution requests

## Scope

| Wire ID | Event | Serializer | ~Count in ref payload |
|---------|-------|------------|----------------------|
| 88 | TASK_INPUTS_FILE_PROPERTY_ROOT_1_0 | ij | ~58 |
| 345 | TASK_INPUTS_FILE_PROPERTY_1_1 | il | ~29 |
| 91 | TASK_INPUTS_IMPLEMENTATION_1_0 | in | ~45 |
| 92 | TASK_INPUTS_PROPERTY_NAMES_1_0 | io | ~45 |
| 95 | TASK_INPUTS_VALUE_PROPERTIES_1_0 | iu | ~45 |
| 94 | TASK_INPUTS_SNAPSHOTTING_STARTED_1_0 | is | ~27 |
| 349 | TASK_INPUTS_SNAPSHOTTING_FINISHED_2_0 | iq | ~27 |
| 119 | PLANNED_NODE_1_0 | fl | ~45 |
| 137 | TRANSFORM_EXECUTION_REQUEST_1_0 | kk | ~33 |

Plus 3 nested types: `FileRef` (dv), `TaskInputsFilePropertyRootChild` (ii), `TaskInputsSnapshottingResult` (ir).

## Field Layouts

All flags use inverted bit convention: bit=0 means field present.

### TASK_INPUTS_FILE_PROPERTY_ROOT_1_0 (wire 88, serializer `ij`)

```
flags: 1 byte (3 bits)
  bit0: id present
  bit1: rootHash present
  bit2: children present
fields (in order):
  0. id: i64 (8-byte LE task id) — conditional on bit0
  1. file: FileRef (nested, always written)
  2. rootHash: byte[] — conditional on bit1
  3. children: List<TaskInputsFilePropertyRootChild> — conditional on bit2
```

### FileRef (nested, serializer `dv`)

```
flags: 1 byte (2 bits)
  bit0: root present
  bit1: path present
fields:
  0. root: enum ordinal (FileRefRootType) — conditional on bit0
  1. path: interned string — conditional on bit1
```

### TaskInputsFilePropertyRootChild (nested, serializer `ii`)

```
flags: 1 byte (3 bits)
  bit0: name present
  bit1: hash present
  bit2: parent present
fields:
  0. name: interned string — conditional on bit0
  1. hash: byte[] — conditional on bit1
  2. parent: i32 (varint) — conditional on bit2
```

### TASK_INPUTS_FILE_PROPERTY_1_1 (wire 345, serializer `il`)

```
flags: 1 byte (4 bits)
  bit0: id present
  bit1: attributes present
  bit2: hash present
  bit3: roots present
fields:
  0. id: i64 (8-byte LE) — conditional on bit0
  1. attributes: List<interned string> — conditional on bit1
  2. hash: byte[] — conditional on bit2
  3. roots: List<i64> — conditional on bit3
```

### TASK_INPUTS_IMPLEMENTATION_1_0 (wire 91, serializer `in`)

```
flags: 1 byte (4 bits)
  bit0: id present
  bit1: classLoaderHash present
  bit2: actionClassLoaderHashes present
  bit3: actionClassNames present
fields:
  0. id: i64 (8-byte LE) — conditional on bit0
  1. classLoaderHash: byte[] — conditional on bit1
  2. actionClassLoaderHashes: List<byte[]> — conditional on bit2
  3. actionClassNames: List<interned string> — conditional on bit3
```

### TASK_INPUTS_PROPERTY_NAMES_1_0 (wire 92, serializer `io`)

```
flags: 1 byte (4 bits)
  bit0: id present
  bit1: valueInputs present
  bit2: fileInputs present
  bit3: outputs present
fields:
  0. id: i64 (8-byte LE) — conditional on bit0
  1. valueInputs: List<interned string> — conditional on bit1
  2. fileInputs: List<interned string> — conditional on bit2
  3. outputs: List<interned string> — conditional on bit3
```

### TASK_INPUTS_VALUE_PROPERTIES_1_0 (wire 95, serializer `iu`)

```
flags: 1 byte (2 bits)
  bit0: id present
  bit1: hashes present
fields:
  0. id: i64 (8-byte LE) — conditional on bit0
  1. hashes: List<byte[]> — conditional on bit1
```

### TASK_INPUTS_SNAPSHOTTING_STARTED_1_0 (wire 94, serializer `is`)

```
flags: NONE (no flags byte)
fields:
  0. task: i64 (8-byte LE) — always written
```

### TASK_INPUTS_SNAPSHOTTING_FINISHED_2_0 (wire 349, serializer `iq`)

```
flags: 1 byte (3 bits)
  bit0: task present
  bit1: result present
  bit2: failureId present
fields:
  0. task: i64 (8-byte LE) — conditional on bit0
  1. result: TaskInputsSnapshottingResult (nested) — conditional on bit1
  2. failureId: i64 (8-byte LE) — conditional on bit2
```

### TaskInputsSnapshottingResult (nested, serializer `ir`)

```
flags: 1 byte (5 bits)
  bit0: hash present
  bit1: implementation present (i64 cross-ref)
  bit2: propertyNames present (i64 cross-ref)
  bit3: valueInputs present (i64 cross-ref)
  bit4: fileInputs present (List<i64>)
fields:
  0. hash: byte[] — conditional on bit0
  1. implementation: i64 — conditional on bit1
  2. propertyNames: i64 — conditional on bit2
  3. valueInputs: i64 — conditional on bit3
  4. fileInputs: List<i64> — conditional on bit4
```

### PLANNED_NODE_1_0 (wire 119, serializer `fl`)

```
flags: 1 byte (5 bits)
  bit0: id present
  bit1: dependencies present
  bit2: mustRunAfter present
  bit3: shouldRunAfter present
  bit4: finalizedBy present
fields:
  0. id: i64 (8-byte LE) — conditional on bit0
  1. dependencies: List<i64> — conditional on bit1
  2. mustRunAfter: List<i64> — conditional on bit2
  3. shouldRunAfter: List<i64> — conditional on bit3
  4. finalizedBy: List<i64> — conditional on bit4
```

### TRANSFORM_EXECUTION_REQUEST_1_0 (wire 137, serializer `kk`)

```
flags: 1 byte (3 bits)
  bit0: nodeId present
  bit1: identificationId present
  bit2: executionId present
fields:
  0. nodeId: i64 (8-byte LE) — conditional on bit0
  1. identificationId: i64 (8-byte LE) — conditional on bit1
  2. executionId: i64 (8-byte LE) — conditional on bit2
```

## Code Changes

### kryo.rs — New Primitives

4 new helper functions:
- `read_i32_varint(data, pos) -> i32` — ZigZag i32 for TaskInputsFilePropertyRootChild.parent
- `read_list_of_i64(data, pos) -> Vec<i64>` — varint length, then N × 8-byte LE i64s
- `read_list_of_byte_arrays(data, pos) -> Vec<Vec<u8>>` — varint length, then N byte arrays
- `read_list_of_interned_strings(data, pos, table) -> Vec<String>` — varint length, then N interned strings

### events/ — New Decoder Files

9 new files, one per event type. Each implements `BodyDecoder` trait. Nested types decoded inline.

```
events/task_inputs_file_property_root.rs   # wire 88
events/task_inputs_file_property.rs        # wire 345
events/task_inputs_implementation.rs       # wire 91
events/task_inputs_property_names.rs       # wire 92
events/task_inputs_value_properties.rs     # wire 95
events/task_inputs_snapshotting_started.rs # wire 94
events/task_inputs_snapshotting_finished.rs# wire 349
events/planned_node.rs                     # wire 119
events/transform_execution_request.rs      # wire 137
```

### events/mod.rs — Enum + Registry Extensions

`DecodedEvent` enum extended with 9 new variants. `DecoderRegistry::new()` registers 9 new wire IDs.

### models.rs — Data Model Extensions

`Task` gains `inputs: Option<TaskInputs>`. `BuildScanPayload` gains `planned_nodes` and `transform_execution_requests` vectors. New structs for all decoded data types.

### assembly.rs — Enrichment Logic

First pass collects task input events into per-id maps. After building `Task` structs, attach matching `TaskInputs`. `PlannedNode` and `TransformExecutionRequest` collected into standalone vectors on `BuildScanPayload`.

## Testing

- **Unit tests per decoder**: all-fields-present, some-fields-absent, edge cases for list types
- **Nested type tests**: FileRef, TaskInputsFilePropertyRootChild, TaskInputsSnapshottingResult
- **kryo.rs primitive tests**: each new helper with empty/single/multiple element cases
- **Integration test extension**: assert fewer raw events, non-empty inputs/planned_nodes/transform_requests

## Out of Scope

- No new CLI subcommands
- No version negotiation (hardcoded wire IDs for versions seen in captured data)
- No UI or API changes
