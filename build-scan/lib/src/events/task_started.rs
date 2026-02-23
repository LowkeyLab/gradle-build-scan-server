use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskStartedEvent};

pub struct TaskStartedDecoder;

impl BodyDecoder for TaskStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            varint::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };
        let build_path = if kryo::is_field_present(flags as u16, 1) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let path = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };
        let class_name = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };
        // bit 4: parent (ConfigurationParentRef) — skip if present
        if kryo::is_field_present(flags as u16, 4) {
            // Read and discard: flags byte + optional enum + optional long
            let parent_flags = kryo::read_flags_byte(body, &mut pos)?;
            if kryo::is_field_present(parent_flags as u16, 0) {
                let _ = varint::read_unsigned_varint(body, &mut pos)?; // enum ordinal
            }
            if kryo::is_field_present(parent_flags as u16, 1) {
                let _ = varint::read_zigzag_i64(body, &mut pos)?; // id
            }
        }

        Ok(DecodedEvent::TaskStarted(TaskStartedEvent {
            id,
            build_path,
            path,
            class_name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_without_parent() {
        // flags=0x10 (bit4=1 → parent absent, bits 0-3 = 0 → present)
        let mut data = vec![0x10]; // flags
        data.push(0x02); // id: zigzag(1)=2
        data.push(0x02);
        data.push(58); // buildPath ":"
        // path ":app:compileKotlin" (18 chars) → zigzag(18)=36
        data.push(0x24);
        for &c in b":app:compileKotlin" {
            data.push(c);
        }
        // className → zigzag(47)=94
        data.push(0x5e);
        for &c in b"org.jetbrains.kotlin.gradle.tasks.KotlinCompile" {
            data.push(c);
        }

        let decoder = TaskStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskStarted(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.path, ":app:compileKotlin");
            assert_eq!(
                e.class_name.as_deref(),
                Some("org.jetbrains.kotlin.gradle.tasks.KotlinCompile")
            );
        } else {
            panic!("expected TaskStarted");
        }
    }
}
