use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TransformExecutionStartedEvent};

pub struct TransformExecutionStartedDecoder;

impl BodyDecoder for TransformExecutionStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;

        // No flags byte. Single field always present.
        let id = kryo::read_zigzag_i64(body, &mut pos)?;

        Ok(DecodedEvent::TransformExecutionStarted(
            TransformExecutionStartedEvent { id },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_zigzag_i64(n: i64) -> Vec<u8> {
        let zigzag = ((n << 1) ^ (n >> 63)) as u64;
        let mut buf = Vec::new();
        let mut value = zigzag;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if value == 0 {
                break;
            }
        }
        buf
    }

    #[test]
    fn test_decode_positive_id() {
        let data = encode_zigzag_i64(5);
        let decoder = TransformExecutionStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionStarted(e) = result {
            assert_eq!(e.id, 5);
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
            assert_eq!(e.id, -3);
        } else {
            panic!("expected TransformExecutionStarted");
        }
    }
}
