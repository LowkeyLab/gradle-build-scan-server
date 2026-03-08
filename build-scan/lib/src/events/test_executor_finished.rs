use error::ParseError;
use models::ExecutorId;

use super::{BodyDecoder, DecodedEvent, TestExecutorFinishedEvent};

pub struct TestExecutorFinishedDecoder;

impl BodyDecoder for TestExecutorFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            ExecutorId::new(kryo::read_task_id(body, &mut pos)?)
        } else {
            ExecutorId::new(0)
        };

        Ok(DecodedEvent::TestExecutorFinished(
            TestExecutorFinishedEvent { executor_id },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_executor_id() {
        // flags bit 0 = executor_id present → 0x00
        let mut data = vec![0x00];
        data.extend_from_slice(&21i64.to_le_bytes()); // executor_id

        let decoder = TestExecutorFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorFinished(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(21));
        } else {
            panic!("expected TestExecutorFinished");
        }
    }
}
