use error::ParseError;
use models::TaskId;

use super::{BodyDecoder, DecodedEvent, TaskFinishedEvent};

pub struct TaskFinishedDecoder;

impl BodyDecoder for TaskFinishedDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_u16_be(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let id = if kryo::is_field_present(flags, 0) {
            TaskId::new(kryo::read_task_id(body, &mut pos)?)
        } else {
            TaskId::new(0)
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

        let origin_execution_time = if kryo::is_field_present(flags, 9) {
            Some(varint::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        // bit 10: actionable (boolean — value IS the bit, no payload)
        let actionable = if kryo::is_field_present(flags, 10) {
            Some(true)
        } else {
            Some(false)
        };

        // bit 11: upToDateMessages
        let up_to_date_messages = if kryo::is_field_present(flags, 11) {
            let count = varint::read_unsigned_varint(body, &mut pos)? as usize;
            let mut messages = Vec::with_capacity(count);
            for _ in 0..count {
                messages.push(table.read_string(body, &mut pos)?);
            }
            Some(messages)
        } else {
            None
        };

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
            origin_execution_time,
            actionable,
            skip_reason_message,
            up_to_date_messages,
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
            assert_eq!(e.id, TaskId::new(1));
            assert_eq!(e.path, ":app:build");
            assert_eq!(e.outcome, Some(3)); // SUCCESS
            assert_eq!(e.cacheable, Some(false)); // bit4=1 means absent → false
            assert_eq!(e.up_to_date_messages, None); // bit11=1 means absent
        } else {
            panic!("expected TaskFinished");
        }
    }

    #[test]
    fn test_decode_up_to_date_messages_present() {
        // bits ABSENT (=1): 3,4,5,6,7,8,9,10,12  — i.e. everything except 0,1,2,11
        // bit 11 PRESENT (=0), bits 0,1,2 PRESENT (=0)
        // absent bits: 3,4,5,6,7,8,9,10,12
        // flags bits (1=absent): bit3=1,bit4=1,bit5=1,bit6=1,bit7=1,bit8=1,bit9=1,bit10=1,bit12=1
        // = 0b0001_0111_1111_1000 = 0x17F8
        let mut data = vec![];
        data.push(0x17);
        data.push(0xF8);
        // id: 1i64 as fixed 8-byte LE
        data.extend_from_slice(&1i64.to_le_bytes());
        // path ":lib:compile" → zigzag(12)=24=0x18
        data.push(0x18);
        for &c in b":lib:compile" {
            data.push(c);
        }
        data.push(0x05); // outcome ordinal 5 = UP_TO_DATE
        // upToDateMessages: count=2, then 2 strings
        data.push(0x02); // count = 2 (unsigned varint)
        // string "Input property 'x' has not changed" — 34 chars, zigzag(34)=68=0x44
        let msg1 = b"Input property 'x' has not changed";
        // zigzag(len): len=34, zigzag = 34*2 = 68 = 0x44
        data.push(0x44);
        data.extend_from_slice(msg1);
        // string "Output file has not changed" — 27 chars, zigzag(27)=54=0x36
        let msg2 = b"Output file has not changed";
        data.push(0x36);
        data.extend_from_slice(msg2);

        let decoder = TaskFinishedDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TaskFinished(e) = result {
            assert_eq!(e.id, TaskId::new(1));
            assert_eq!(e.path, ":lib:compile");
            assert_eq!(e.outcome, Some(5));
            let msgs = e
                .up_to_date_messages
                .expect("up_to_date_messages should be Some");
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0], "Input property 'x' has not changed");
            assert_eq!(msgs[1], "Output file has not changed");
        } else {
            panic!("expected TaskFinished");
        }
    }
}
