use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskRegistrationSummaryEvent};

pub struct TaskRegistrationSummaryDecoder;

/// Wire 122: TaskRegistrationSummary_1_0 — single unconditional int, no flags.
impl BodyDecoder for TaskRegistrationSummaryDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let task_count = kryo::read_positive_varint_i32(body, &mut pos)?;

        Ok(DecodedEvent::TaskRegistrationSummary(
            TaskRegistrationSummaryEvent { task_count },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_task_count() {
        let data = [0x0a]; // unsigned varint 10
        let decoder = TaskRegistrationSummaryDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskRegistrationSummary(e) = result {
            assert_eq!(e.task_count, 10);
        } else {
            panic!("expected TaskRegistrationSummary");
        }
    }
}
