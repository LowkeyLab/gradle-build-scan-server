use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TransformExecutionFinishedEvent};

pub struct TransformExecutionFinishedDecoder;

impl BodyDecoder for TransformExecutionFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_u16_be(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags, 0) {
            kryo::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };

        let failure_id = if kryo::is_field_present(flags, 1) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let outcome = if kryo::is_field_present(flags, 2) {
            Some(kryo::read_enum_ordinal(body, &mut pos)?)
        } else {
            None
        };

        let execution_reasons = if kryo::is_field_present(flags, 3) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        let caching_disabled_reason_category = if kryo::is_field_present(flags, 4) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let caching_disabled_explanation = if kryo::is_field_present(flags, 5) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let origin_build_invocation_id = if kryo::is_field_present(flags, 6) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let origin_build_cache_key = if kryo::is_field_present(flags, 7) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else {
            None
        };

        let origin_execution_time = if kryo::is_field_present(flags, 8) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::TransformExecutionFinished(
            TransformExecutionFinishedEvent {
                id,
                failure_id,
                outcome,
                execution_reasons,
                caching_disabled_reason_category,
                caching_disabled_explanation,
                origin_build_invocation_id,
                origin_build_cache_key,
                origin_execution_time,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryo::encode_zigzag_i64;

    #[test]
    fn test_decode_id_and_outcome_only() {
        // bits present (=0): 0 (id), 2 (outcome)
        // bits absent (=1): 1,3,4,5,6,7,8
        // flags = 0b0000_0001_1111_1010 = 0x01FA
        let mut data = vec![0x01, 0xFA];
        data.extend_from_slice(&encode_zigzag_i64(99)); // id = 99
        data.push(0x02); // outcome ordinal = 2

        let decoder = TransformExecutionFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionFinished(e) = result {
            assert_eq!(e.id, 99);
            assert_eq!(e.failure_id, None);
            assert_eq!(e.outcome, Some(2));
            assert!(e.execution_reasons.is_empty());
            assert_eq!(e.caching_disabled_reason_category, None);
            assert_eq!(e.caching_disabled_explanation, None);
            assert_eq!(e.origin_build_invocation_id, None);
            assert_eq!(e.origin_build_cache_key, None);
            assert_eq!(e.origin_execution_time, None);
        } else {
            panic!("expected TransformExecutionFinished");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        // flags = 0x01FF: all 9 bits set = all absent
        let data = vec![0x01, 0xFF];
        let decoder = TransformExecutionFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TransformExecutionFinished(e) = result {
            assert_eq!(e.id, 0);
            assert_eq!(e.failure_id, None);
            assert_eq!(e.outcome, None);
            assert!(e.execution_reasons.is_empty());
        } else {
            panic!("expected TransformExecutionFinished");
        }
    }
}
