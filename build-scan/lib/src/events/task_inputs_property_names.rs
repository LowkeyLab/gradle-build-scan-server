use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskInputsPropertyNamesEvent};

pub struct TaskInputsPropertyNamesDecoder;

impl BodyDecoder for TaskInputsPropertyNamesDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let value_inputs = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };
        let file_inputs = if kryo::is_field_present(flags as u16, 2) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };
        let outputs = if kryo::is_field_present(flags as u16, 3) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::TaskInputsPropertyNames(
            TaskInputsPropertyNamesEvent {
                id,
                value_inputs,
                file_inputs,
                outputs,
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
        data.extend_from_slice(&3i64.to_le_bytes());
        data.push(0x01);
        data.push(0x0E);
        data.extend_from_slice(b"enabled");
        data.push(0x01);
        data.push(0x12);
        data.extend_from_slice(b"classpath");
        data.push(0x01);
        data.push(0x12);
        data.extend_from_slice(b"outputDir");
        let decoder = TaskInputsPropertyNamesDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsPropertyNames(e) = result {
            assert_eq!(e.id, Some(3));
            assert_eq!(e.value_inputs, vec!["enabled"]);
            assert_eq!(e.file_inputs, vec!["classpath"]);
            assert_eq!(e.outputs, vec!["outputDir"]);
        } else {
            panic!("expected TaskInputsPropertyNames");
        }
    }
}
