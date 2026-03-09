use error::ParseError;
use models::TaskId;

use super::{BodyDecoder, DecodedEvent, TaskInputsImplementationEvent};

pub struct TaskInputsImplementationDecoder;

impl BodyDecoder for TaskInputsImplementationDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(TaskId::new(kryo::read_task_id(body, &mut pos)?))
        } else {
            None
        };
        let class_loader_hash = if kryo::is_field_present(flags as u16, 1) {
            let hash = kryo::read_byte_array(body, &mut pos)?;
            // Trailing varint after the hash bytes (type/version indicator, discarded)
            let _ = kryo::read_positive_varint_i32(body, &mut pos)?;
            Some(hash)
        } else {
            None
        };
        let action_class_loader_hashes = if kryo::is_field_present(flags as u16, 2) {
            kryo::read_list_of_byte_arrays(body, &mut pos)?
        } else {
            vec![]
        };
        let action_class_names = if kryo::is_field_present(flags as u16, 3) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::TaskInputsImplementation(
            TaskInputsImplementationEvent {
                id,
                class_loader_hash,
                action_class_loader_hashes,
                action_class_names,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 4 bits present
        data.extend_from_slice(&9i64.to_le_bytes()); // id (8-byte LE)
        data.push(0x02); // class_loader_hash: 2 bytes
        data.extend_from_slice(&[0xDE, 0xAD]);
        data.push(0x00); // trailing type indicator for class_loader_hash
        data.push(0x01); // action_class_loader_hashes: 1 element
        data.push(0x02); // first element: 2 bytes
        data.extend_from_slice(&[0xBE, 0xEF]);
        data.push(0x01); // action_class_names: 1 element
        data.push(0x10); // string: zigzag(16)=8 chars
        data.extend_from_slice(b"MyAction");
        let decoder = TaskInputsImplementationDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsImplementation(e) = result {
            assert_eq!(e.id, Some(TaskId::new(9)));
            assert_eq!(e.class_loader_hash, Some(vec![0xDE, 0xAD]));
            assert_eq!(e.action_class_loader_hashes, vec![vec![0xBE, 0xEF]]);
            assert_eq!(e.action_class_names, vec!["MyAction"]);
        } else {
            panic!("expected TaskInputsImplementation");
        }
    }
}
