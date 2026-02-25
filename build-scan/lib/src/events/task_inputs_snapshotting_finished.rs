use error::ParseError;

use super::{
    BodyDecoder, DecodedEvent, TaskInputsSnapshottingFinishedEvent, TaskInputsSnapshottingResult,
};

pub struct TaskInputsSnapshottingFinishedDecoder;

fn decode_result(body: &[u8], pos: &mut usize) -> Result<TaskInputsSnapshottingResult, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;
    let hash = if kryo::is_field_present(flags as u16, 0) {
        Some(kryo::read_byte_array(body, pos)?)
    } else {
        None
    };
    let implementation = if kryo::is_field_present(flags as u16, 1) {
        Some(kryo::read_task_id(body, pos)?)
    } else {
        None
    };
    let property_names = if kryo::is_field_present(flags as u16, 2) {
        Some(kryo::read_task_id(body, pos)?)
    } else {
        None
    };
    let value_inputs = if kryo::is_field_present(flags as u16, 3) {
        Some(kryo::read_task_id(body, pos)?)
    } else {
        None
    };
    let file_inputs = if kryo::is_field_present(flags as u16, 4) {
        kryo::read_list_of_i64(body, pos)?
    } else {
        vec![]
    };
    Ok(TaskInputsSnapshottingResult {
        hash,
        implementation,
        property_names,
        value_inputs,
        file_inputs,
    })
}

impl BodyDecoder for TaskInputsSnapshottingFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let task = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let result = if kryo::is_field_present(flags as u16, 1) {
            Some(decode_result(body, &mut pos)?)
        } else {
            None
        };
        let failure_id = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        Ok(DecodedEvent::TaskInputsSnapshottingFinished(
            TaskInputsSnapshottingFinishedEvent {
                task,
                result,
                failure_id,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_result() {
        let mut data = vec![0x04]; // bits 0,1 present, bit 2 absent
        data.extend_from_slice(&42i64.to_le_bytes());
        data.push(0x10); // inner flags: bits 0-3 present, bit 4 absent
        data.push(0x04);
        data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        data.extend_from_slice(&10i64.to_le_bytes());
        data.extend_from_slice(&11i64.to_le_bytes());
        data.extend_from_slice(&12i64.to_le_bytes());
        let decoder = TaskInputsSnapshottingFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsSnapshottingFinished(e) = result {
            assert_eq!(e.task, Some(42));
            let r = e.result.unwrap();
            assert_eq!(r.hash, Some(vec![0x01, 0x02, 0x03, 0x04]));
            assert_eq!(r.implementation, Some(10));
            assert_eq!(r.property_names, Some(11));
            assert_eq!(r.value_inputs, Some(12));
            assert!(r.file_inputs.is_empty());
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected TaskInputsSnapshottingFinished");
        }
    }
}
