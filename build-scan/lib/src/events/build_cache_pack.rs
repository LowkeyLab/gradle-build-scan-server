use error::ParseError;

use super::{BodyDecoder, BuildCachePackFinishedEvent, BuildCachePackStartedEvent, DecodedEvent};

/// Wire 41, 297: BuildCachePackStarted — 3 fields: work_id, id, cache_key
pub struct BuildCachePackStartedDecoder;

impl BodyDecoder for BuildCachePackStartedDecoder {
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

        Ok(DecodedEvent::BuildCachePackStarted(
            BuildCachePackStartedEvent {
                work_id,
                id,
                cache_key,
            },
        ))
    }
}

/// Wire 42: BuildCachePackFinished_1_0 — 4 fields: id, archive_size, archive_entry_count, ExceptionTree (skip)
pub struct BuildCachePackFinishedV1Decoder;

impl BodyDecoder for BuildCachePackFinishedV1Decoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let archive_size = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let archive_entry_count = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        // bit 3 = ExceptionTree — skip remaining bytes, failure_id stays None
        let _ = pos; // remaining bytes ignored

        Ok(DecodedEvent::BuildCachePackFinished(
            BuildCachePackFinishedEvent {
                id,
                archive_size,
                archive_entry_count,
                failure_id: None,
            },
        ))
    }
}

/// Wire 298: BuildCachePackFinished_1_1 — 4 fields: id, archive_size, archive_entry_count, failure_id
pub struct BuildCachePackFinishedDecoder;

impl BodyDecoder for BuildCachePackFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let archive_size = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let archive_entry_count = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let failure_id = if kryo::is_field_present(flags as u16, 3) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildCachePackFinished(
            BuildCachePackFinishedEvent {
                id,
                archive_size,
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
        let mut data = vec![0x00u8];
        data.extend_from_slice(&1i64.to_le_bytes()); // work_id
        data.extend_from_slice(&2i64.to_le_bytes()); // id
        data.extend(encode_interned_string("pack-key")); // cache_key

        let decoder = BuildCachePackStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackStarted(e) = result {
            assert_eq!(e.work_id, 1);
            assert_eq!(e.id, 2);
            assert_eq!(e.cache_key, Some("pack-key".into()));
        } else {
            panic!("expected BuildCachePackStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCachePackStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
        } else {
            panic!("expected BuildCachePackStarted");
        }
    }

    #[test]
    fn test_decode_finished_v1_all_fields() {
        let mut data = vec![0x00u8]; // bits 0-2 present
        data.extend_from_slice(&10i64.to_le_bytes()); // id
        data.extend(kryo::encode_unsigned_varint(4096)); // archive_size
        data.extend(kryo::encode_unsigned_varint(50)); // archive_entry_count
        // bit 3 = ExceptionTree, remaining ignored

        let decoder = BuildCachePackFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackFinished(e) = result {
            assert_eq!(e.id, 10);
            assert_eq!(e.archive_size, Some(4096));
            assert_eq!(e.archive_entry_count, Some(50));
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCachePackFinished");
        }
    }

    #[test]
    fn test_decode_finished_v1_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCachePackFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.archive_size, None);
            assert_eq!(e.archive_entry_count, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCachePackFinished");
        }
    }

    #[test]
    fn test_decode_finished_all_fields() {
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&11i64.to_le_bytes()); // id
        data.extend(kryo::encode_unsigned_varint(8192)); // archive_size
        data.extend(kryo::encode_unsigned_varint(100)); // archive_entry_count
        data.extend(kryo::encode_zigzag_i64(77)); // failure_id

        let decoder = BuildCachePackFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackFinished(e) = result {
            assert_eq!(e.id, 11);
            assert_eq!(e.archive_size, Some(8192));
            assert_eq!(e.archive_entry_count, Some(100));
            assert_eq!(e.failure_id, Some(77));
        } else {
            panic!("expected BuildCachePackFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCachePackFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCachePackFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.archive_size, None);
            assert_eq!(e.archive_entry_count, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCachePackFinished");
        }
    }
}
