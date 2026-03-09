use error::ParseError;
use models::ExecutorId;

use super::{BodyDecoder, DecodedEvent, TestResultEvent};

pub struct TestResultDecoder;

impl BodyDecoder for TestResultDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            ExecutorId::new(kryo::read_kryo_long(body, &mut pos)?)
        } else {
            ExecutorId::new(0)
        };
        let result_ordinal = if kryo::is_field_present(flags as u16, 1) {
            let raw = kryo::read_kryo_long(body, &mut pos)?;
            if raw < 0 { None } else { Some(raw as u64) }
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
        data.extend_from_slice(&kryo::encode_kryo_long(5)); // executor_id
        data.extend_from_slice(&kryo::encode_kryo_long(2)); // result_ordinal = 2

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(5));
            assert_eq!(e.result_ordinal, Some(2));
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_without_result() {
        // bit 0 = 0 (executor_id present), bit 1 = 1 (result_ordinal absent) → flags = 0x02
        let mut data = vec![0x02];
        data.extend_from_slice(&kryo::encode_kryo_long(8)); // executor_id

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(8));
            assert_eq!(e.result_ordinal, None);
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_kryo_long_executor_id() {
        // executor_id should use Kryo-long encoding (same as TestCase wire 798),
        // not fixed 8-byte LE. This test encodes executor_id as Kryo-long.
        let mut data = vec![0x00]; // both fields present
        data.extend_from_slice(&kryo::encode_kryo_long(-9000000000i64)); // executor_id
        data.extend_from_slice(&kryo::encode_kryo_long(2)); // result_ordinal

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(-9000000000i64));
            assert_eq!(e.result_ordinal, Some(2));
        } else {
            panic!("expected TestResult");
        }
    }
}
