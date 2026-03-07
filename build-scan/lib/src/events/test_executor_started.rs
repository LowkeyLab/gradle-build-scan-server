use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestExecutorStartedEvent};

pub struct TestExecutorStartedDecoder;

impl BodyDecoder for TestExecutorStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        Ok(DecodedEvent::TestExecutorStarted(
            TestExecutorStartedEvent { executor_id },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_executor_id() {
        // flags bit 0 = executor_id present → 0b00000000 = 0x00
        let mut data = vec![0x00];
        data.extend_from_slice(&13i64.to_le_bytes()); // executor_id

        let decoder = TestExecutorStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorStarted(e) = result {
            assert_eq!(e.executor_id, 13);
        } else {
            panic!("expected TestExecutorStarted");
        }
    }
}
