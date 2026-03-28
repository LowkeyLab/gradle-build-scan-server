use error::ParseError;

use super::{
    BodyDecoder, BuildCacheLocalStoreFinishedEvent, BuildCacheLocalStoreStartedEvent, DecodedEvent,
};

/// Wire 146: BuildCacheLocalStoreStarted_1_0 — 4 fields: work_id, id, cache_key, archive_size
pub struct BuildCacheLocalStoreStartedDecoder;

impl BodyDecoder for BuildCacheLocalStoreStartedDecoder {
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

        Ok(DecodedEvent::BuildCacheLocalStoreStarted(
            BuildCacheLocalStoreStartedEvent {
                work_id,
                id,
                cache_key,
                archive_size,
            },
        ))
    }
}

/// Wire 147: BuildCacheLocalStoreFinished_1_0 — 3 fields: id, stored, failure_id
pub struct BuildCacheLocalStoreFinishedDecoder;

impl BodyDecoder for BuildCacheLocalStoreFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let stored = if kryo::is_field_present(flags as u16, 1) {
            if pos >= body.len() {
                return Err(ParseError::UnexpectedEof { offset: pos });
            }
            let v = body[pos] != 0;
            pos += 1;
            Some(v)
        } else {
            None
        };

        let failure_id = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildCacheLocalStoreFinished(
            BuildCacheLocalStoreFinishedEvent {
                id,
                stored,
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
        data.extend_from_slice(&3i64.to_le_bytes()); // work_id
        data.extend_from_slice(&4i64.to_le_bytes()); // id
        data.extend(encode_interned_string("store-key")); // cache_key
        data.extend(kryo::encode_unsigned_varint(1024)); // archive_size

        let decoder = BuildCacheLocalStoreStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreStarted(e) = result {
            assert_eq!(e.work_id, 3);
            assert_eq!(e.id, 4);
            assert_eq!(e.cache_key, Some("store-key".into()));
            assert_eq!(e.archive_size, Some(1024));
        } else {
            panic!("expected BuildCacheLocalStoreStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCacheLocalStoreStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
            assert_eq!(e.archive_size, None);
        } else {
            panic!("expected BuildCacheLocalStoreStarted");
        }
    }

    #[test]
    fn test_decode_finished_all_fields() {
        let mut data = vec![0x00u8]; // all 3 bits present
        data.extend_from_slice(&50i64.to_le_bytes()); // id
        data.push(0x01); // stored = true
        data.extend(kryo::encode_zigzag_i64(123)); // failure_id

        let decoder = BuildCacheLocalStoreFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreFinished(e) = result {
            assert_eq!(e.id, 50);
            assert_eq!(e.stored, Some(true));
            assert_eq!(e.failure_id, Some(123));
        } else {
            panic!("expected BuildCacheLocalStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheLocalStoreFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.stored, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheLocalStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_stored_true() {
        let mut data = vec![0x00u8]; // all present
        data.extend_from_slice(&1i64.to_le_bytes()); // id
        data.push(0x01); // stored = true
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id = 0

        let decoder = BuildCacheLocalStoreFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreFinished(e) = result {
            assert_eq!(e.stored, Some(true));
        } else {
            panic!("expected BuildCacheLocalStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_stored_false() {
        let mut data = vec![0x00u8]; // all present
        data.extend_from_slice(&2i64.to_le_bytes()); // id
        data.push(0x00); // stored = false
        data.extend(kryo::encode_zigzag_i64(1)); // failure_id

        let decoder = BuildCacheLocalStoreFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalStoreFinished(e) = result {
            assert_eq!(e.stored, Some(false));
            assert_eq!(e.failure_id, Some(1));
        } else {
            panic!("expected BuildCacheLocalStoreFinished");
        }
    }
}
