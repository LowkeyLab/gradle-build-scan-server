use error::ParseError;

use super::{
    BodyDecoder, DecodedEvent, FilePropertyRootChild, FileRef, TaskInputsFilePropertyRootEvent,
};

pub struct TaskInputsFilePropertyRootDecoder;

fn decode_file_ref(
    body: &[u8],
    pos: &mut usize,
    table: &mut kryo::StringInternTable,
) -> Result<FileRef, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;
    let root = if kryo::is_field_present(flags as u16, 0) {
        Some(kryo::read_enum_ordinal(body, pos)?)
    } else {
        None
    };
    let path = if kryo::is_field_present(flags as u16, 1) {
        Some(table.read_string(body, pos)?)
    } else {
        None
    };
    Ok(FileRef { root, path })
}

fn decode_child(
    body: &[u8],
    pos: &mut usize,
    table: &mut kryo::StringInternTable,
) -> Result<FilePropertyRootChild, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;
    let name = if kryo::is_field_present(flags as u16, 0) {
        Some(table.read_string(body, pos)?)
    } else {
        None
    };
    let hash = if kryo::is_field_present(flags as u16, 1) {
        Some(kryo::read_byte_array(body, pos)?)
    } else {
        None
    };
    let parent = if kryo::is_field_present(flags as u16, 2) {
        Some(varint::read_zigzag_i32(body, pos)?)
    } else {
        None
    };
    Ok(FilePropertyRootChild { name, hash, parent })
}

impl BodyDecoder for TaskInputsFilePropertyRootDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_task_id(body, &mut pos)?)
        } else {
            None
        };
        let file = decode_file_ref(body, &mut pos, &mut table)?;
        let root_hash = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else {
            None
        };
        let children = if kryo::is_field_present(flags as u16, 2) {
            let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
            let mut children = Vec::with_capacity(count);
            for _ in 0..count {
                children.push(decode_child(body, &mut pos, &mut table)?);
            }
            children
        } else {
            vec![]
        };

        Ok(DecodedEvent::TaskInputsFilePropertyRoot(
            TaskInputsFilePropertyRootEvent {
                id,
                file,
                root_hash,
                children,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_file_and_children() {
        let mut data = vec![0x00]; // all present
        data.extend_from_slice(&8i64.to_le_bytes());
        // FileRef: flags 0x00, root=0, path="src"
        data.push(0x00);
        data.push(0x00);
        data.push(0x06);
        data.extend_from_slice(b"src");
        // rootHash
        data.push(0x02);
        data.extend_from_slice(&[0xAB, 0xCD]);
        // 1 child
        data.push(0x01);
        data.push(0x00); // child flags: all present
        data.push(0x0E);
        data.extend_from_slice(b"Main.kt"); // zigzag(7)=14
        data.push(0x02);
        data.extend_from_slice(&[0x11, 0x22]);
        data.push(0x00); // parent = zigzag(0) = 0
        let decoder = TaskInputsFilePropertyRootDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskInputsFilePropertyRoot(e) = result {
            assert_eq!(e.id, Some(8));
            assert_eq!(e.file.root, Some(0));
            assert_eq!(e.file.path.as_deref(), Some("src"));
            assert_eq!(e.root_hash, Some(vec![0xAB, 0xCD]));
            assert_eq!(e.children.len(), 1);
            assert_eq!(e.children[0].name.as_deref(), Some("Main.kt"));
            assert_eq!(e.children[0].hash, Some(vec![0x11, 0x22]));
            assert_eq!(e.children[0].parent, Some(0));
        } else {
            panic!("expected TaskInputsFilePropertyRoot");
        }
    }
}
