use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TransformExecutionRequestEvent};

pub struct TransformExecutionRequestDecoder;

impl BodyDecoder for TransformExecutionRequestDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let node_id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let identification_id = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let execution_id = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::TransformExecutionRequest(
            TransformExecutionRequestEvent {
                node_id,
                identification_id,
                execution_id,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00];
        data.extend_from_slice(&1i64.to_le_bytes());
        data.extend_from_slice(&2i64.to_le_bytes());
        data.extend_from_slice(&3i64.to_le_bytes());
        let decoder = TransformExecutionRequestDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionRequest(e) = result {
            assert_eq!(e.node_id, Some(1));
            assert_eq!(e.identification_id, Some(2));
            assert_eq!(e.execution_id, Some(3));
        } else {
            panic!("expected TransformExecutionRequest");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        let data = vec![0x07]; // bits 0,1,2 set = all absent
        let decoder = TransformExecutionRequestDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionRequest(e) = result {
            assert_eq!(e.node_id, None);
            assert_eq!(e.identification_id, None);
            assert_eq!(e.execution_id, None);
        } else {
            panic!("expected TransformExecutionRequest");
        }
    }
}
