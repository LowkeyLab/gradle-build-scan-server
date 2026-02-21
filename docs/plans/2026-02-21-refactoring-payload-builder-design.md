# Refactoring PayloadBuilder Design

## Purpose
Refactor `PayloadBuilder` to remove its dependency on decompression logic (`Decompressor`). By introducing a layer above it, `BuildScanParser`, we can test the `PayloadBuilder` logic directly using uncompressed bytes instead of gzip-compressing the bytes within tests.

## Approach
1. **New Struct (`BuildScanParser`)**: Introduce a new structural layer, potentially named `BuildScanParser` in `build-scan/lib/src/parser.rs`.
2. **Move Decompression Logic**: Move the `build_from_compressed` method from `PayloadBuilder` to `BuildScanParser` (e.g., `pub fn parse_compressed(&self, data: &[u8]) -> Result<BuildScanPayload, ParseError>`).
3. **Refactor existing `PayloadBuilder` Tests**: Update `test_builder_maps_known_events` and `test_builder_parses_known_events_and_halts_on_unknown` to remove `gzip_compress` logic and test `PayloadBuilder::build()` directly with the uncompressed `raw_data`.
4. **Integration Test for `BuildScanParser`**: Add a small unit test for `BuildScanParser::parse_compressed` to ensure it correctly orchestrates decompression and payload parsing.
5. **CLI Integration**: Update the CLI to utilize the new `BuildScanParser` instead of directly instantiating `PayloadBuilder` to parse payloads.
