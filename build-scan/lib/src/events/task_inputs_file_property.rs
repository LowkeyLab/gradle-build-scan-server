use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskInputsFilePropertyEvent};

pub struct TaskInputsFilePropertyDecoder;

impl BodyDecoder for TaskInputsFilePropertyDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let attributes = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };
        let hash = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else {
            None
        };
        let roots = if kryo::is_field_present(flags as u16, 3) {
            kryo::read_list_of_i64(body, &mut pos)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::TaskInputsFileProperty(
            TaskInputsFilePropertyEvent {
                id,
                attributes,
                hash,
                roots,
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
        data.extend_from_slice(&4i64.to_le_bytes());
        data.push(0x01);
        data.push(0x16);
        data.extend_from_slice(b"INCREMENTAL");
        data.push(0x02);
        data.extend_from_slice(&[0xFF, 0x00]);
        data.push(0x01);
        data.extend_from_slice(&100i64.to_le_bytes());
        let decoder = TaskInputsFilePropertyDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsFileProperty(e) = result {
            assert_eq!(e.id, Some(4));
            assert_eq!(e.attributes, vec!["INCREMENTAL"]);
            assert_eq!(e.hash, Some(vec![0xFF, 0x00]));
            assert_eq!(e.roots, vec![100]);
        } else {
            panic!("expected TaskInputsFileProperty");
        }
    }
}
