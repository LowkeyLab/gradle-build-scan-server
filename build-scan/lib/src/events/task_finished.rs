use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TaskFinishedEvent};

pub struct TaskFinishedDecoder;

impl BodyDecoder for TaskFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_u16_be(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };

        let path = if kryo::is_field_present(flags, 1) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        let outcome = if kryo::is_field_present(flags, 2) {
            Some(kryo::read_enum_ordinal(body, &mut pos)?)
        } else {
            None
        };

        let _skip_message = if kryo::is_field_present(flags, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        // bit 4: cacheable (boolean — value IS the bit, no payload)
        let cacheable = if kryo::is_field_present(flags, 4) {
            Some(true)
        } else {
            Some(false)
        };

        let caching_disabled_reason_category = if kryo::is_field_present(flags, 5) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let caching_disabled_explanation = if kryo::is_field_present(flags, 6) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let origin_build_invocation_id = if kryo::is_field_present(flags, 7) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let origin_build_cache_key = if kryo::is_field_present(flags, 8) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else {
            None
        };

        if kryo::is_field_present(flags, 9) {
            // originExecutionTime — read and discard (zigzag long)
            let _ = varint::read_zigzag_i64(body, &mut pos)?;
        }

        // bit 10: actionable (boolean — value IS the bit, no payload)
        let actionable = if kryo::is_field_present(flags, 10) {
            Some(true)
        } else {
            Some(false)
        };

        if kryo::is_field_present(flags, 11) {
            // upToDateMessages list — read count then skip strings
            let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
            for _ in 0..count {
                let _ = table.read_string(body, &mut pos)?;
            }
        }

        let skip_reason_message = if kryo::is_field_present(flags, 12) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::TaskFinished(TaskFinishedEvent {
            id,
            path,
            outcome,
            cacheable,
            caching_disabled_reason_category,
            caching_disabled_explanation,
            origin_build_invocation_id,
            origin_build_cache_key,
            actionable,
            skip_reason_message,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_success_not_cacheable() {
        // bits that are ABSENT (=1): 3,4,5,6,7,8,9,10,11,12
        // bits that are PRESENT (=0): 0,1,2
        // flags = 0b0001_1111_1111_1000 = 0x1FF8, encoded as fixed 2-byte BE u16
        let mut data = vec![];
        // flags as fixed big-endian u16: 0x1FF8
        data.push(0x1F);
        data.push(0xF8);
        // id: 1i64 as fixed 8-byte LE
        data.extend_from_slice(&1i64.to_le_bytes());
        // path ":app:build" → zigzag(10)=20
        data.push(0x14);
        for &c in b":app:build" {
            data.push(c);
        }
        data.push(0x03); // outcome: ordinal 3 = SUCCESS

        let decoder = TaskFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskFinished(e) = result {
            assert_eq!(e.id, 1);
            assert_eq!(e.path, ":app:build");
            assert_eq!(e.outcome, Some(3)); // SUCCESS
            assert_eq!(e.cacheable, Some(false)); // bit4=1 means absent → false
        } else {
            panic!("expected TaskFinished");
        }
    }
}
