use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskInputsSnapshottingStartedEvent};

pub struct TaskInputsSnapshottingStartedDecoder;

impl BodyDecoder for TaskInputsSnapshottingStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let task = kryo::read_task_id(body, &mut pos)?;
        Ok(DecodedEvent::TaskInputsSnapshottingStarted(
            TaskInputsSnapshottingStartedEvent { task },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let data = 42i64.to_le_bytes();
        let decoder = TaskInputsSnapshottingStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsSnapshottingStarted(e) = result {
            assert_eq!(e.task, 42);
        } else {
            panic!("expected TaskInputsSnapshottingStarted");
        }
    }
}
