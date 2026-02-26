use error::ParseError;

use super::{BodyDecoder, DecodedEvent, FileRefRootsEvent, FileRefRootEntry};

pub struct FileRefRootsDecoder;

/// Wire 49: FileRefRoots_1_0 — Map<Enum, String> with sorted keys, no flags.
impl BodyDecoder for FileRefRootsDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let mut table = kryo::StringInternTable::new();

        let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let root_type = kryo::read_enum_ordinal(body, &mut pos)?;
            let path = table.read_string(body, &mut pos)?;
            entries.push(FileRefRootEntry { root_type, path });
        }

        Ok(DecodedEvent::FileRefRoots(FileRefRootsEvent { entries }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_two_entries() {
        let mut data = vec![0x02]; // 2 entries
        // Entry 1: type=0 (WORKSPACE), path="/home/user/project"
        data.push(0x00); // enum ordinal 0
        let path1 = "/home/user/project";
        data.push((path1.len() as u8) * 2); // zigzag(len)
        data.extend_from_slice(path1.as_bytes());
        // Entry 2: type=1 (GRADLE_USER_HOME), path="/home/user/.gradle"
        data.push(0x01); // enum ordinal 1
        let path2 = "/home/user/.gradle";
        data.push((path2.len() as u8) * 2);
        data.extend_from_slice(path2.as_bytes());

        let decoder = FileRefRootsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::FileRefRoots(e) = result {
            assert_eq!(e.entries.len(), 2);
            assert_eq!(e.entries[0].root_type, 0);
            assert_eq!(e.entries[0].path, "/home/user/project");
            assert_eq!(e.entries[1].root_type, 1);
            assert_eq!(e.entries[1].path, "/home/user/.gradle");
        } else {
            panic!("expected FileRefRoots");
        }
    }

    #[test]
    fn test_decode_empty() {
        let data = [0x00]; // 0 entries
        let decoder = FileRefRootsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::FileRefRoots(e) = result {
            assert!(e.entries.is_empty());
        } else {
            panic!("expected FileRefRoots");
        }
    }
}
