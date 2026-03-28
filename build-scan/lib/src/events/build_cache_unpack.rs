use error::ParseError;

use super::{
    BodyDecoder, BuildCacheUnpackFinishedEvent, BuildCacheUnpackStartedEvent, DecodedEvent,
};

/// Wire 47, 303: BuildCacheUnpackStarted — 4 fields: work_id, id, cache_key, archive_size
pub struct BuildCacheUnpackStartedDecoder;

impl BodyDecoder for BuildCacheUnpackStartedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let work_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let id = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let cache_key = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let archive_size = if kryo::is_field_present(flags as u16, 3) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildCacheUnpackStarted(
            BuildCacheUnpackStartedEvent {
                work_id,
                id,
                cache_key,
                archive_size,
            },
        ))
    }
}

/// Wire 48: BuildCacheUnpackFinished_1_0 — 3 fields: id, archive_entry_count, ExceptionTree (skip)
pub struct BuildCacheUnpackFinishedV1Decoder;

impl BodyDecoder for BuildCacheUnpackFinishedV1Decoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let archive_entry_count = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        // bit 2 = ExceptionTree — skip remaining bytes, failure_id stays None
        let _ = pos; // remaining bytes ignored

        Ok(DecodedEvent::BuildCacheUnpackFinished(
            BuildCacheUnpackFinishedEvent {
                id,
                archive_entry_count,
                failure_id: None,
            },
        ))
    }
}

/// Wire 304: BuildCacheUnpackFinished_1_1 — 3 fields: id, archive_entry_count, failure_id
pub struct BuildCacheUnpackFinishedDecoder;

impl BodyDecoder for BuildCacheUnpackFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let archive_entry_count = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let failure_id = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildCacheUnpackFinished(
            BuildCacheUnpackFinishedEvent {
                id,
                archive_entry_count,
                failure_id,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_interned_string(s: &str) -> Vec<u8> {
        let mut v = kryo::encode_unsigned_varint(s.len() as u64 * 2);
        v.extend_from_slice(s.as_bytes());
        v
    }

    #[test]
    fn test_decode_started_all_fields() {
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&5i64.to_le_bytes()); // work_id
        data.extend_from_slice(&6i64.to_le_bytes()); // id
        data.extend(encode_interned_string("unpack-key")); // cache_key
        data.extend(kryo::encode_unsigned_varint(2048)); // archive_size

        let decoder = BuildCacheUnpackStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackStarted(e) = result {
            assert_eq!(e.work_id, 5);
            assert_eq!(e.id, 6);
            assert_eq!(e.cache_key, Some("unpack-key".into()));
            assert_eq!(e.archive_size, Some(2048));
        } else {
            panic!("expected BuildCacheUnpackStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCacheUnpackStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
            assert_eq!(e.archive_size, None);
        } else {
            panic!("expected BuildCacheUnpackStarted");
        }
    }

    #[test]
    fn test_decode_finished_v1_all_fields() {
        let mut data = vec![0x00u8]; // bits 0-1 present
        data.extend_from_slice(&15i64.to_le_bytes()); // id
        data.extend(kryo::encode_unsigned_varint(25)); // archive_entry_count
        // bit 2 = ExceptionTree, remaining ignored

        let decoder = BuildCacheUnpackFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackFinished(e) = result {
            assert_eq!(e.id, 15);
            assert_eq!(e.archive_entry_count, Some(25));
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheUnpackFinished");
        }
    }

    #[test]
    fn test_decode_finished_v1_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheUnpackFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.archive_entry_count, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheUnpackFinished");
        }
    }

    #[test]
    fn test_decode_finished_all_fields() {
        let mut data = vec![0x00u8]; // all 3 bits present
        data.extend_from_slice(&20i64.to_le_bytes()); // id
        data.extend(kryo::encode_unsigned_varint(30)); // archive_entry_count
        data.extend(kryo::encode_zigzag_i64(88)); // failure_id

        let decoder = BuildCacheUnpackFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackFinished(e) = result {
            assert_eq!(e.id, 20);
            assert_eq!(e.archive_entry_count, Some(30));
            assert_eq!(e.failure_id, Some(88));
        } else {
            panic!("expected BuildCacheUnpackFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheUnpackFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheUnpackFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.archive_entry_count, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheUnpackFinished");
        }
    }
}
