use error::ParseError;

use super::{BodyDecoder, DecodedEvent, BuildFinishedEvent};

pub struct BuildFinishedDecoder;

/// Wire 259: BuildFinished_1_1 — single nullable failureId.
impl BodyDecoder for BuildFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let failure_id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildFinished(BuildFinishedEvent {
            failure_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_no_failure() {
        let data = [0x01]; // bit 0 absent → no failure
        let decoder = BuildFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildFinished(e) = result {
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildFinished");
        }
    }

    #[test]
    fn test_decode_with_failure() {
        let data = [0x00, 0x14]; // bit 0 present, zigzag(10) = 20
        let decoder = BuildFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildFinished(e) = result {
            assert_eq!(e.failure_id, Some(10));
        } else {
            panic!("expected BuildFinished");
        }
    }
}
