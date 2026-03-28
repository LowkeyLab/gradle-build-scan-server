use error::ParseError;

use super::{
    BodyDecoder, BuildCacheRemoteLoadFinishedEvent, BuildCacheRemoteLoadStartedEvent, DecodedEvent,
};

/// Wire 43, 299: BuildCacheRemoteLoadStarted — 3 fields: work_id, id, cache_key
pub struct BuildCacheRemoteLoadStartedDecoder;

impl BodyDecoder for BuildCacheRemoteLoadStartedDecoder {
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

        Ok(DecodedEvent::BuildCacheRemoteLoadStarted(
            BuildCacheRemoteLoadStartedEvent {
                work_id,
                id,
                cache_key,
            },
        ))
    }
}

/// Wire 44: BuildCacheRemoteLoadFinished_1_0 — 4 fields: id, hit, archive_size, ExceptionTree (skip)
pub struct BuildCacheRemoteLoadFinishedV1Decoder;

impl BodyDecoder for BuildCacheRemoteLoadFinishedV1Decoder {
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

        // bit 3 = ExceptionTree — skip remaining bytes, failure_id stays None
        let _ = pos; // remaining bytes ignored

        Ok(DecodedEvent::BuildCacheRemoteLoadFinished(
            BuildCacheRemoteLoadFinishedEvent {
                id,
                hit,
                archive_size,
                failure_id: None,
                remote_cache_location: None,
            },
        ))
    }
}

/// Wire 300: BuildCacheRemoteLoadFinished_1_1 (has_remote_cache_location=false)
/// Wire 556: BuildCacheRemoteLoadFinished_1_2 (has_remote_cache_location=true)
pub struct BuildCacheRemoteLoadFinishedDecoder {
    pub has_remote_cache_location: bool,
}

impl BodyDecoder for BuildCacheRemoteLoadFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

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

        let remote_cache_location =
            if self.has_remote_cache_location && kryo::is_field_present(flags as u16, 4) {
                Some(table.read_string(body, &mut pos)?)
            } else {
                None
            };

        Ok(DecodedEvent::BuildCacheRemoteLoadFinished(
            BuildCacheRemoteLoadFinishedEvent {
                id,
                hit,
                archive_size,
                failure_id,
                remote_cache_location,
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
        data.extend_from_slice(&10i64.to_le_bytes()); // work_id
        data.extend_from_slice(&20i64.to_le_bytes()); // id
        data.extend(encode_interned_string("key1")); // cache_key

        let decoder = BuildCacheRemoteLoadStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadStarted(e) = result {
            assert_eq!(e.work_id, 10);
            assert_eq!(e.id, 20);
            assert_eq!(e.cache_key, Some("key1".into()));
        } else {
            panic!("expected BuildCacheRemoteLoadStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheRemoteLoadStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
        } else {
            panic!("expected BuildCacheRemoteLoadStarted");
        }
    }

    #[test]
    fn test_decode_finished_v1_all_fields() {
        let mut data = vec![0x00u8]; // bits 0-2 present, bit 3 (ExceptionTree) also present
        data.extend_from_slice(&5i64.to_le_bytes()); // id
        data.push(0x01); // hit = true
        data.extend(kryo::encode_unsigned_varint(2048)); // archive_size
        // bit 3 = ExceptionTree, remaining bytes skipped

        let decoder = BuildCacheRemoteLoadFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadFinished(e) = result {
            assert_eq!(e.id, 5);
            assert_eq!(e.hit, Some(true));
            assert_eq!(e.archive_size, Some(2048));
            assert_eq!(e.failure_id, None);
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_v1_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCacheRemoteLoadFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.hit, None);
            assert_eq!(e.archive_size, None);
            assert_eq!(e.failure_id, None);
        } else {
            panic!("expected BuildCacheRemoteLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_v11_all_fields() {
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&3i64.to_le_bytes()); // id
        data.push(0x00); // hit = false
        data.extend(kryo::encode_unsigned_varint(512)); // archive_size
        data.extend(kryo::encode_zigzag_i64(99)); // failure_id

        let decoder = BuildCacheRemoteLoadFinishedDecoder {
            has_remote_cache_location: false,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadFinished(e) = result {
            assert_eq!(e.id, 3);
            assert_eq!(e.hit, Some(false));
            assert_eq!(e.archive_size, Some(512));
            assert_eq!(e.failure_id, Some(99));
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_v12_all_fields() {
        let mut data = vec![0x00u8]; // all 5 bits present
        data.extend_from_slice(&7i64.to_le_bytes()); // id
        data.push(0x01); // hit = true
        data.extend(kryo::encode_unsigned_varint(100)); // archive_size
        data.extend(kryo::encode_zigzag_i64(42)); // failure_id
        data.extend(encode_interned_string("https://cache.example.com")); // remote_cache_location

        let decoder = BuildCacheRemoteLoadFinishedDecoder {
            has_remote_cache_location: true,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadFinished(e) = result {
            assert_eq!(e.id, 7);
            assert_eq!(e.hit, Some(true));
            assert_eq!(
                e.remote_cache_location,
                Some("https://cache.example.com".into())
            );
        } else {
            panic!("expected BuildCacheRemoteLoadFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x1Fu8]; // bits 0-4 set = all 5 fields absent
        let decoder = BuildCacheRemoteLoadFinishedDecoder {
            has_remote_cache_location: true,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteLoadFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.hit, None);
            assert_eq!(e.archive_size, None);
            assert_eq!(e.failure_id, None);
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteLoadFinished");
        }
    }
}
