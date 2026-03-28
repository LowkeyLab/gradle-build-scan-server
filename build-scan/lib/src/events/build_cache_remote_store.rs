use error::ParseError;

use super::{
    BodyDecoder, BuildCacheRemoteStoreFinishedEvent, BuildCacheRemoteStoreStartedEvent,
    DecodedEvent,
};

/// Wire 46, 302: BuildCacheRemoteStoreStarted — 4 fields: work_id, id, cache_key, archive_size
pub struct BuildCacheRemoteStoreStartedDecoder;

impl BodyDecoder for BuildCacheRemoteStoreStartedDecoder {
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

        Ok(DecodedEvent::BuildCacheRemoteStoreStarted(
            BuildCacheRemoteStoreStartedEvent {
                work_id,
                id,
                cache_key,
                archive_size,
            },
        ))
    }
}

/// Wire 45: BuildCacheRemoteStoreFinished_1_0 — 3 fields: id, stored, ExceptionTree (skip)
pub struct BuildCacheRemoteStoreFinishedV1Decoder;

impl BodyDecoder for BuildCacheRemoteStoreFinishedV1Decoder {
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

        // bit 2 = ExceptionTree — skip remaining bytes, failure_id stays None
        let _ = pos; // remaining bytes ignored

        Ok(DecodedEvent::BuildCacheRemoteStoreFinished(
            BuildCacheRemoteStoreFinishedEvent {
                id,
                stored,
                failure_id: None,
                rejected_reason: None,
                maximum_artifact_size: None,
                remote_cache_location: None,
            },
        ))
    }
}

/// Wire 301: BuildCacheRemoteStoreFinished_1_1 (no extras)
/// Wire 557: BuildCacheRemoteStoreFinished_1_2 (+ rejected_reason)
/// Wire 813: BuildCacheRemoteStoreFinished_1_3 (+ maximum_artifact_size)
/// Wire 1069: BuildCacheRemoteStoreFinished_1_4 (+ remote_cache_location)
pub struct BuildCacheRemoteStoreFinishedDecoder {
    pub has_rejected_reason: bool,
    pub has_maximum_artifact_size: bool,
    pub has_remote_cache_location: bool,
}

impl BodyDecoder for BuildCacheRemoteStoreFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

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

        let rejected_reason = if self.has_rejected_reason && kryo::is_field_present(flags as u16, 3)
        {
            Some(kryo::read_enum_ordinal(body, &mut pos)?)
        } else {
            None
        };

        let maximum_artifact_size =
            if self.has_maximum_artifact_size && kryo::is_field_present(flags as u16, 4) {
                Some(kryo::read_positive_varint_i64(body, &mut pos)?)
            } else {
                None
            };

        let remote_cache_location =
            if self.has_remote_cache_location && kryo::is_field_present(flags as u16, 5) {
                Some(table.read_string(body, &mut pos)?)
            } else {
                None
            };

        Ok(DecodedEvent::BuildCacheRemoteStoreFinished(
            BuildCacheRemoteStoreFinishedEvent {
                id,
                stored,
                failure_id,
                rejected_reason,
                maximum_artifact_size,
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
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&8i64.to_le_bytes()); // work_id
        data.extend_from_slice(&9i64.to_le_bytes()); // id
        data.extend(encode_interned_string("remote-key")); // cache_key
        data.extend(kryo::encode_unsigned_varint(4096)); // archive_size

        let decoder = BuildCacheRemoteStoreStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreStarted(e) = result {
            assert_eq!(e.work_id, 8);
            assert_eq!(e.id, 9);
            assert_eq!(e.cache_key, Some("remote-key".into()));
            assert_eq!(e.archive_size, Some(4096));
        } else {
            panic!("expected BuildCacheRemoteStoreStarted");
        }
    }

    #[test]
    fn test_decode_started_no_fields() {
        let data = vec![0x0Fu8]; // bits 0-3 set = all 4 fields absent
        let decoder = BuildCacheRemoteStoreStartedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreStarted(e) = result {
            assert_eq!(e.work_id, 0);
            assert_eq!(e.id, 0);
            assert_eq!(e.cache_key, None);
            assert_eq!(e.archive_size, None);
        } else {
            panic!("expected BuildCacheRemoteStoreStarted");
        }
    }

    #[test]
    fn test_decode_finished_v1_all_fields() {
        let mut data = vec![0x00u8]; // bits 0-1 present
        data.extend_from_slice(&22i64.to_le_bytes()); // id
        data.push(0x01); // stored = true
        // bit 2 = ExceptionTree, remaining ignored

        let decoder = BuildCacheRemoteStoreFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.id, 22);
            assert_eq!(e.stored, Some(true));
            assert_eq!(e.failure_id, None);
            assert_eq!(e.rejected_reason, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_v1_no_fields() {
        let data = vec![0x07u8]; // bits 0-2 set = all 3 fields absent
        let decoder = BuildCacheRemoteStoreFinishedV1Decoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.stored, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_v11_all_fields() {
        let mut data = vec![0x00u8]; // all 3 bits present
        data.extend_from_slice(&30i64.to_le_bytes()); // id
        data.push(0x00); // stored = false
        data.extend(kryo::encode_zigzag_i64(55)); // failure_id

        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: false,
            has_maximum_artifact_size: false,
            has_remote_cache_location: false,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.id, 30);
            assert_eq!(e.stored, Some(false));
            assert_eq!(e.failure_id, Some(55));
            assert_eq!(e.rejected_reason, None);
            assert_eq!(e.maximum_artifact_size, None);
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_v12_with_rejected_reason() {
        let mut data = vec![0x00u8]; // all 4 bits present
        data.extend_from_slice(&31i64.to_le_bytes()); // id
        data.push(0x00); // stored = false
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id
        data.extend(kryo::encode_unsigned_varint(2)); // rejected_reason = ordinal 2

        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: true,
            has_maximum_artifact_size: false,
            has_remote_cache_location: false,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.rejected_reason, Some(2));
            assert_eq!(e.maximum_artifact_size, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_v13_with_maximum_artifact_size() {
        let mut data = vec![0x00u8]; // all 5 bits present
        data.extend_from_slice(&32i64.to_le_bytes()); // id
        data.push(0x00); // stored = false
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id
        data.extend(kryo::encode_unsigned_varint(1)); // rejected_reason
        data.extend(kryo::encode_unsigned_varint(100_000_000)); // maximum_artifact_size

        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: true,
            has_maximum_artifact_size: true,
            has_remote_cache_location: false,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.maximum_artifact_size, Some(100_000_000));
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_v14_all_fields() {
        let mut data = vec![0x00u8]; // all 6 bits present
        data.extend_from_slice(&33i64.to_le_bytes()); // id
        data.push(0x01); // stored = true
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id
        data.extend(kryo::encode_unsigned_varint(0)); // rejected_reason
        data.extend(kryo::encode_unsigned_varint(50_000)); // maximum_artifact_size
        data.extend(encode_interned_string("https://remote.example.com")); // remote_cache_location

        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: true,
            has_maximum_artifact_size: true,
            has_remote_cache_location: true,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.id, 33);
            assert_eq!(e.stored, Some(true));
            assert_eq!(
                e.remote_cache_location,
                Some("https://remote.example.com".into())
            );
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_no_fields() {
        let data = vec![0x3Fu8]; // bits 0-5 set = all 6 fields absent
        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: true,
            has_maximum_artifact_size: true,
            has_remote_cache_location: true,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.stored, None);
            assert_eq!(e.failure_id, None);
            assert_eq!(e.rejected_reason, None);
            assert_eq!(e.maximum_artifact_size, None);
            assert_eq!(e.remote_cache_location, None);
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }

    #[test]
    fn test_decode_finished_stored_true() {
        let mut data = vec![0x00u8]; // bits 0-2 present
        data.extend_from_slice(&1i64.to_le_bytes()); // id
        data.push(0x01); // stored = true
        data.extend(kryo::encode_zigzag_i64(0)); // failure_id

        let decoder = BuildCacheRemoteStoreFinishedDecoder {
            has_rejected_reason: false,
            has_maximum_artifact_size: false,
            has_remote_cache_location: false,
        };
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildCacheRemoteStoreFinished(e) = result {
            assert_eq!(e.stored, Some(true));
        } else {
            panic!("expected BuildCacheRemoteStoreFinished");
        }
    }
}
