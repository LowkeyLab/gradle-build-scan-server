use error::ParseError;

use super::{BodyDecoder, DecodedEvent};

pub struct BuildStartedDecoder;

/// Wire 6: BuildStarted_1_0 — marker event with empty body.
impl BodyDecoder for BuildStartedDecoder {
    fn decode(&self, _body: &[u8]) -> Result<DecodedEvent, ParseError> {
        Ok(DecodedEvent::BuildStarted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_empty_body() {
        let decoder = BuildStartedDecoder;
        let result = decoder.decode(&[]).unwrap();
        assert!(matches!(result, DecodedEvent::BuildStarted));
    }
}
