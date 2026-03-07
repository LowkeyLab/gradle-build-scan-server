use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestResultEvent};

pub struct TestResultDecoder;

impl BodyDecoder for TestResultDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };
        // Bit 1: result data — read as 8-byte LE, interpret as u64
        let result_ordinal = if kryo::is_field_present(flags as u16, 1) {
            let raw = kryo::read_task_id(body, &mut pos)?;
            Some(raw as u64)
        } else {
            None
        };

        Ok(DecodedEvent::TestResult(TestResultEvent {
            executor_id,
            result_ordinal,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_result() {
        // Flags are inverted: bit=0 means field IS present.
        // Both bit 0 and bit 1 = 0 → both fields present → flags = 0x00
        let mut data = vec![0x00];
        data.extend_from_slice(&5i64.to_le_bytes()); // executor_id
        data.extend_from_slice(&2i64.to_le_bytes()); // result_ordinal = 2

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, 5);
            assert_eq!(e.result_ordinal, Some(2));
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_without_result() {
        // bit 0 = 0 (executor_id present), bit 1 = 1 (result_ordinal absent) → flags = 0x02
        let mut data = vec![0x02];
        data.extend_from_slice(&8i64.to_le_bytes()); // executor_id

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, 8);
            assert_eq!(e.result_ordinal, None);
        } else {
            panic!("expected TestResult");
        }
    }
}
