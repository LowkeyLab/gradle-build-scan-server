use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TransformIdentificationEvent};

pub struct TransformIdentificationDecoder;

impl BodyDecoder for TransformIdentificationDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };

        let component_identity = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_positive_varint_i32(body, &mut pos)?
        } else {
            0
        };

        let input_artifact_name = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        let transform_action_class = if kryo::is_field_present(flags as u16, 3) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        let from_attributes = if kryo::is_field_present(flags as u16, 4) {
            kryo::read_list_of_positive_varint_i32(body, &mut pos)?
        } else {
            vec![]
        };

        let to_attributes = if kryo::is_field_present(flags as u16, 5) {
            kryo::read_list_of_positive_varint_i32(body, &mut pos)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::TransformIdentification(
            TransformIdentificationEvent {
                id,
                component_identity,
                input_artifact_name,
                transform_action_class,
                from_attributes,
                to_attributes,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_zigzag_i64(n: i64) -> Vec<u8> {
        let zigzag = ((n << 1) ^ (n >> 63)) as u64;
        let mut buf = Vec::new();
        let mut value = zigzag;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if value == 0 {
                break;
            }
        }
        buf
    }

    #[test]
    fn test_decode_all_present() {
        // flags = 0x00: all 6 bits present
        let mut data = vec![0x00];
        data.extend_from_slice(&encode_zigzag_i64(10)); // id = 10
        data.push(0x03); // component_identity = 3 (unsigned varint)
        // input_artifact_name = "in" → zigzag(2)=4, then 'i'=105, 'n'=110
        data.push(0x04);
        data.push(105);
        data.push(110);
        // transform_action_class = "TC" → zigzag(2)=4, then 'T'=84, 'C'=67
        data.push(0x04);
        data.push(84);
        data.push(67);
        // from_attributes: len=2, then 1, 2
        data.push(0x02);
        data.push(0x01);
        data.push(0x02);
        // to_attributes: len=1, then 5
        data.push(0x01);
        data.push(0x05);

        let decoder = TransformIdentificationDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformIdentification(e) = result {
            assert_eq!(e.id, 10);
            assert_eq!(e.component_identity, 3);
            assert_eq!(e.input_artifact_name, "in");
            assert_eq!(e.transform_action_class, "TC");
            assert_eq!(e.from_attributes, vec![1, 2]);
            assert_eq!(e.to_attributes, vec![5]);
        } else {
            panic!("expected TransformIdentification");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        // flags = 0x3F: all 6 bits set = all absent
        let data = vec![0x3F];
        let decoder = TransformIdentificationDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformIdentification(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.component_identity, 0);
            assert_eq!(e.input_artifact_name, "");
            assert_eq!(e.transform_action_class, "");
            assert!(e.from_attributes.is_empty());
            assert!(e.to_attributes.is_empty());
        } else {
            panic!("expected TransformIdentification");
        }
    }
}
