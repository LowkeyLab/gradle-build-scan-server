use error::ParseError;

use super::{BodyDecoder, DecodedEvent, BuildRequestedTasksEvent};

pub struct BuildRequestedTasksDecoder;

/// Wire 5: BuildRequestedTasks_1_0 — two lists of interned strings.
impl BodyDecoder for BuildRequestedTasksDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let requested = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        let excluded = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::BuildRequestedTasks(
            BuildRequestedTasksEvent { requested, excluded },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_tasks() {
        let mut data = vec![0x00]; // both bits present
        // requested: ["build"]
        data.push(0x01); // list len = 1
        data.push(0x0A); // zigzag(5) = 10
        data.extend_from_slice(b"build");
        // excluded: []
        data.push(0x00); // list len = 0

        let decoder = BuildRequestedTasksDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildRequestedTasks(e) = result {
            assert_eq!(e.requested, vec!["build"]);
            assert!(e.excluded.is_empty());
        } else {
            panic!("expected BuildRequestedTasks");
        }
    }
}
