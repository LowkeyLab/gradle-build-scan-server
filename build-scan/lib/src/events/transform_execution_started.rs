use error::ParseError;
use models::TransformId;

use super::{BodyDecoder, DecodedEvent, TransformExecutionStartedEvent};

pub struct TransformExecutionStartedDecoder;

impl BodyDecoder for TransformExecutionStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;

        // No flags byte. Single field always present.
        let id = TransformId::new(kryo::read_zigzag_i64(body, &mut pos)?);

        Ok(DecodedEvent::TransformExecutionStarted(
            TransformExecutionStartedEvent { id },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryo::encode_zigzag_i64;

    #[test]
    fn test_decode_positive_id() {
        let data = encode_zigzag_i64(5);
        let decoder = TransformExecutionStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionStarted(e) = result {
            assert_eq!(e.id, TransformId::new(5));
        } else {
            panic!("expected TransformExecutionStarted");
        }
    }

    #[test]
    fn test_decode_negative_id() {
        let data = encode_zigzag_i64(-3);
        let decoder = TransformExecutionStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionStarted(e) = result {
            assert_eq!(e.id, TransformId::new(-3));
        } else {
            panic!("expected TransformExecutionStarted");
        }
    }
}
