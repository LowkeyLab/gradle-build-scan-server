use error::ParseError;

use super::{
    BodyDecoder, BuildCacheLocalLoadFinishedEvent, BuildCacheLocalLoadStartedEvent, DecodedEvent,
};

/// Wire 144: BuildCacheLocalLoadStarted_1_0 — 3 fields: work_id, id, cache_key
pub struct BuildCacheLocalLoadStartedDecoder;

impl BodyDecoder for BuildCacheLocalLoadStartedDecoder {
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

        Ok(DecodedEvent::BuildCacheLocalLoadStarted(
            BuildCacheLocalLoadStartedEvent {
                work_id,
                id,
                cache_key,
            },
        ))
    }
}

/// Wire 145: BuildCacheLocalLoadFinished_1_0 — 4 fields: id, hit, archive_size, failure_id
pub struct BuildCacheLocalLoadFinishedDecoder;

impl BodyDecoder for BuildCacheLocalLoadFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let hit = if kryo::is_field_present(flags as u16, 1) {
            if pos >= body.len() {
                return Err(ParseError::UnexpectedEof { offset: pos });
            }
            let v = body[pos] != 0;
            pos += 1;
            Some(v)
        } else {
            None
        };

        let archive_size = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let failure_id = if kryo::is_field_present(flags as u16, 3) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildCacheLocalLoadFinished(
            BuildCacheLocalLoadFinishedEvent {
                id,
                hit,
                archive_size,
                failure_id,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_interned_string(s: &str) -> Vec<u8> {
        let mut v = kryo::encode_unsigned_varint(s.len() as u64 * 2); // zigzag(n) = 2n for n>=0
        v.extend_from_slice(s.as_bytes());
        v
    }

    #[test]
    fn test_decode_started_all_fields() {
        let mut data = vec![0x00u8]; // all 3 bits present
        data.extend_from_slice(&42i64.to_le_bytes()); // work_id
        data.extend_from_slice(&7i64.to_le_bytes()); // id
        data.extend(encode_interned_string("abc")); // cache_key

        let decoder = BuildCacheLocalLoadStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalLoadStarted(e) = result {
            assert_eq!(e.work_id, 42);
            assert_eq!(e.id, 7);
            assert_eq!(e.cache_key, Some("abc".into()));
        } else {
            panic!("expected BuildCacheLocalLoadStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheLocalLoadStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalLoadStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
        } else {
            panic!("expected BuildCacheLocalLoadStarted");
        }
    }

    #[test]
    fn test_decode_finished_all_fields() {
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&99i64.to_le_bytes()); // id
        data.push(0x01); // hit = true
        data.extend(kryo::encode_unsigned_varint(1024)); // archive_size
        data.extend(kryo::encode_zigzag_i64(55)); // failure_id

        let decoder = BuildCacheLocalLoadFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalLoadFinished(e) = result {
            assert_eq!(e.id, 99);
            assert_eq!(e.hit, Some(true));
            assert_eq!(e.archive_size, Some(1024));
            assert_eq!(e.failure_id, Some(55));
        } else {
            panic!("expected BuildCacheLocalLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCacheLocalLoadFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalLoadFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.hit, None);
            assert_eq!(e.archive_size, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheLocalLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_hit_false() {
        let mut data = vec![0x00u8]; // all present
        data.extend_from_slice(&1i64.to_le_bytes()); // id
        data.push(0x00); // hit = false
        data.extend(kryo::encode_unsigned_varint(512)); // archive_size
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id

        let decoder = BuildCacheLocalLoadFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheLocalLoadFinished(e) = result {
            assert_eq!(e.hit, Some(false));
        } else {
            panic!("expected BuildCacheLocalLoadFinished");
        }
    }
}
