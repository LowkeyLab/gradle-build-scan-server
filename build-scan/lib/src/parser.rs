use error::ParseError;
use models::BuildScanPayload;
use primitives::StreamDecoder;

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
        let decompressed = StreamDecoder::decompress(data)?;
        let mut decoder = StreamDecoder::new(&decompressed);
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
                // TODO: Add actual event mappings here later
                _ => {
                    eprintln!("Unknown Event ID encountered: {}", event_id);
                    return Err(ParseError::UnknownEvent { id: event_id });
                }
            }
        }

        Ok(payload)
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
    fn test_builder_graceful_unknown_events() {
        let mut builder = PayloadBuilder::new();
        // A dummy payload: Event 99, value 42
        // 99 = 0x63, 42 = 0x2A
        // Since we don't handle it, it should just error or skip.
        // Compressed gzip of [0x63, 0x2A]
        let payload: [u8; 22] = [
            31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 75, 214, 2, 0, 77, 227, 178, 212, 2, 0, 0, 0,
        ];

        let result = builder.build(&payload);

        match result {
            Err(ParseError::UnknownEvent { id }) => assert_eq!(id, 99),
            _ => panic!("Expected UnknownEvent error, got {:?}", result),
        }
    }
}
