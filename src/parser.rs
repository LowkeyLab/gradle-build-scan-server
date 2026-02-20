use crate::error::ParseError;
use crate::models::BuildScanPayload;
use crate::primitives::{Primitive, StreamDecoder};

pub struct PayloadBuilder {
    pub dictionary: Vec<String>,
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

        while let Some(prim_res) = decoder.next() {
            let prim = prim_res?;
            match prim {
                Primitive::String(s) => {
                    self.dictionary.push(s);
                }
                // Implement state machine logic here based on heuristics
                // For example, if we hit a known Event ID varint, consume next N primitives
                _ => {}
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
}
