use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskIdentityEvent};

pub struct TaskIdentityDecoder;

impl BodyDecoder for TaskIdentityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };
        let build_path = if kryo::is_field_present(flags as u16, 1) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let task_path = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        Ok(DecodedEvent::TaskIdentity(TaskIdentityEvent {
            id,
            build_path,
            task_path,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_fields_present() {
        // flags=0x00 (all present), id=1 as fixed 8-byte LE, buildPath=":", taskPath=":app:build"
        let mut data = vec![0x00]; // flags: all present
        // id: 1i64 as little-endian 8 bytes
        data.extend_from_slice(&1i64.to_le_bytes());
        // buildPath ":"  → zigzag(1)=2, then char ':'=58
        data.push(0x02);
        data.push(58);
        // taskPath ":app:build" → zigzag(10)=20, then 10 chars
        data.push(0x14);
        for &c in b":app:build" {
            data.push(c);
        }

        let decoder = TaskIdentityDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskIdentity(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.build_path, ":");
            assert_eq!(e.task_path, ":app:build");
        } else {
            panic!("expected TaskIdentity");
        }
    }
}
