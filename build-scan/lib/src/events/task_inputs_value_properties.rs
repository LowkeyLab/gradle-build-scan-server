use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskInputsValuePropertiesEvent};

pub struct TaskInputsValuePropertiesDecoder;

impl BodyDecoder for TaskInputsValuePropertiesDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let hashes = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_list_of_byte_arrays(body, &mut pos)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::TaskInputsValueProperties(
            TaskInputsValuePropertiesEvent { id, hashes },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00];
        data.extend_from_slice(&5i64.to_le_bytes());
        data.push(0x01); // 1 hash
        data.push(0x03);
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let decoder = TaskInputsValuePropertiesDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsValueProperties(e) = result {
            assert_eq!(e.id, Some(5));
            assert_eq!(e.hashes, vec![vec![0xAA, 0xBB, 0xCC]]);
        } else {
            panic!("expected TaskInputsValueProperties");
        }
    }
}
