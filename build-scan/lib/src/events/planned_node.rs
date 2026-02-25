use error::ParseError;

use super::{BodyDecoder, DecodedEvent, PlannedNodeEvent};

pub struct PlannedNodeDecoder;

impl BodyDecoder for PlannedNodeDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let dependencies = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_list_of_i64(body, &mut pos)?
        } else {
            vec![]
        };
        let must_run_after = if kryo::is_field_present(flags as u16, 2) {
            kryo::read_list_of_i64(body, &mut pos)?
        } else {
            vec![]
        };
        let should_run_after = if kryo::is_field_present(flags as u16, 3) {
            kryo::read_list_of_i64(body, &mut pos)?
        } else {
            vec![]
        };
        let finalized_by = if kryo::is_field_present(flags as u16, 4) {
            kryo::read_list_of_i64(body, &mut pos)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::PlannedNode(PlannedNodeEvent {
            id,
            dependencies,
            must_run_after,
            should_run_after,
            finalized_by,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_dependencies() {
        let mut data = vec![0x1C]; // bits 0,1 present, bits 2-4 absent
        data.extend_from_slice(&7i64.to_le_bytes());
        data.push(0x02); // 2 deps
        data.extend_from_slice(&10i64.to_le_bytes());
        data.extend_from_slice(&20i64.to_le_bytes());
        let decoder = PlannedNodeDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::PlannedNode(e) = result {
            assert_eq!(e.id, Some(7));
            assert_eq!(e.dependencies, vec![10, 20]);
            assert!(e.must_run_after.is_empty());
        } else {
            panic!("expected PlannedNode");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        let data = vec![0x1F];
        let decoder = PlannedNodeDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::PlannedNode(e) = result {
            assert_eq!(e.id, None);
            assert!(e.dependencies.is_empty());
        } else {
            panic!("expected PlannedNode");
        }
    }
}
