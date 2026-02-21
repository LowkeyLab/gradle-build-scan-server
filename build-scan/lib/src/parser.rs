use decompress::Decompressor;
use error::ParseError;
use models::BuildScanPayload;
use primitives::{Primitive, StreamDecoder};

const EVENT_TIMESTAMP: u64 = 0;
const EVENT_USER_HOST_INFO: u64 = 2;
const EVENT_JVM_INFO: u64 = 3;
const EVENT_OS_INFO: u64 = 8;
const EVENT_DICTIONARY_ADD: u64 = 14;
const EVENT_TASK_EXECUTION: u64 = 58;

pub struct PayloadBuilder {
    pub dictionary: Vec<String>,
}

impl Default for PayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadBuilder {
    pub fn new() -> Self {
        Self {
            dictionary: Vec::new(),
        }
    }

    pub fn build(&mut self, data: &[u8]) -> Result<BuildScanPayload, ParseError> {
        let mut decoder = StreamDecoder::new(data);
        let payload = BuildScanPayload::default();

        loop {
            // Try to read next event ID. If EOF, we're done.
            let event_id = match decoder.read_raw_varint() {
                Ok(id) => id,
                Err(ParseError::UnexpectedEof) => break,
                Err(e) => return Err(e),
            };

            // Hybrid State Machine:
            // For now, if we don't recognize the event, we can't reliably skip it
            // because we don't know if the next varint is a length for raw bytes.
            // But we will print it to stderr and abort to avoid losing sync.
            match event_id {
                EVENT_USER_HOST_INFO => {
                    let _user = decoder.read_string()?;
                    let _host = decoder.read_string()?;
                }
                EVENT_TASK_EXECUTION => {
                    let _task_path = decoder.read_string()?;
                }
                EVENT_OS_INFO | EVENT_JVM_INFO => {
                    let len = decoder.read_raw_varint()?;
                    let _payload = decoder.read_bytes(len as usize)?;
                }
                EVENT_TIMESTAMP => {
                    let _ts = decoder.read_timestamp()?;
                    // Store/Ignore
                }
                EVENT_DICTIONARY_ADD => {
                    let s = decoder.read_string()?;
                    if let Primitive::String(st) = s {
                        self.dictionary.push(st);
                    } else {
                        return Err(ParseError::UnexpectedPrimitive { expected: "String" });
                    }
                }
                _ => {
                    return Err(ParseError::UnknownEvent { id: event_id });
                }
            }
        }

        Ok(payload)
    }
}

pub struct BuildScanParser {
    pub builder: PayloadBuilder,
}

impl Default for BuildScanParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildScanParser {
    pub fn new() -> Self {
        Self {
            builder: PayloadBuilder::new(),
        }
    }

    pub fn parse_compressed(&mut self, data: &[u8]) -> Result<BuildScanPayload, ParseError> {
        let decompressed = Decompressor::decompress(data)?;
        self.builder.build(&decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_instantiation() {
        let builder = PayloadBuilder::new();
        // Since we don't have a valid gzip payload easily constructable, just assert it compiles
        assert_eq!(builder.dictionary.len(), 0);
    }

    #[test]
    fn test_builder_maps_known_events() {
        let mut builder = PayloadBuilder::new();
        // Construct a dummy payload with known events.
        // 0 -> Timestamp (0)
        // 14 -> String ("test_string" length 11 -> varint 22) + "test_string"
        let mut raw_data = Vec::new();
        // Event 0
        raw_data.push(0);
        // Varint 0
        raw_data.push(0);

        // Event 14
        raw_data.push(14);
        // String length 11 (22 as varint since bit 0 is string vs stringref flag)
        raw_data.push(22);
        raw_data.extend_from_slice(b"test_string");

        let result = builder.build(&raw_data);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
        assert_eq!(builder.dictionary, vec!["test_string".to_string()]);
    }

    #[test]
    fn test_builder_parses_known_events_and_halts_on_unknown() {
        let mut builder = PayloadBuilder::new();
        // Construct a payload with known events, then halt on unknown:
        // Event 0 -> Timestamp (0)
        // Event 14 -> String ("test" -> 8 as varint + "test")
        // Event 0 -> Timestamp (1771622196842 -> varint + something, let's just use 0 -> 0)
        // Actually event 0 expects a timestamp. A timestamp is just a varint.
        // Event 99 -> unknown
        let mut raw_data = Vec::new();
        // Event 0
        raw_data.push(0);
        // Varint 0
        raw_data.push(0);

        // Event 14
        raw_data.push(14);
        raw_data.push(8); // length 4 * 2
        raw_data.extend_from_slice(b"test");

        // Event 0
        raw_data.push(0);
        raw_data.push(0); // timestamp 0

        // Unknown Event 99
        raw_data.push(99);
        raw_data.push(42);

        let result = builder.build(&raw_data);

        match result {
            Err(ParseError::UnknownEvent { id }) => assert_eq!(id, 99),
            _ => panic!("Expected UnknownEvent error for 99, got {:?}", result),
        }
    }

    #[test]
    fn test_build_scan_parser_parses_compressed() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        fn gzip_compress(data: &[u8]) -> Vec<u8> {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }

        let mut raw_data = Vec::new();
        // Event 0 (Timestamp)
        raw_data.push(0);
        raw_data.push(0);

        let compressed = gzip_compress(&raw_data);

        let mut parser = BuildScanParser::new();
        let result = parser.parse_compressed(&compressed);
        assert!(result.is_ok());
    }
}
