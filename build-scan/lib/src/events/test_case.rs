use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestCaseEvent};

pub struct TestCaseDecoder;

impl BodyDecoder for TestCaseDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };
        // Bit 1: secondary_id — read and discard
        if kryo::is_field_present(flags as u16, 1) {
            let _ = kryo::read_task_id(body, &mut pos)?;
        }
        let name1 = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };
        let name2 = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };
        let name3 = if kryo::is_field_present(flags as u16, 4) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        // If name3 is present → method-level: name1=method, name2=class, name3=executor
        // If name3 is absent  → class-level:  name1=class, name2=short_class, no method
        let (class_name, method_name, executor_name) = if name3.is_some() {
            (
                name2.unwrap_or_default(),
                name1,
                name3,
            )
        } else {
            (
                name1.unwrap_or_default(),
                None,
                None,
            )
        };

        Ok(DecodedEvent::TestCase(TestCaseEvent {
            executor_id,
            class_name,
            method_name,
            executor_name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_kryo_string(data: &mut Vec<u8>, s: &str) {
        // zigzag-encoded length then raw bytes
        let len = s.len() as u64;
        // zigzag(n) = n*2 for positive
        let zigzag = len * 2;
        // varint encode
        let mut v = zigzag;
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                data.push(byte);
                break;
            } else {
                data.push(byte | 0x80);
            }
        }
        data.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn test_decode_method_level() {
        // Flags are inverted: bit=0 means field IS present.
        // bit 0 (executor_id)=0 present, bit 1 (secondary_id)=1 absent,
        // bit 2 (name1)=0 present, bit 3 (name2)=0 present, bit 4 (name3)=0 present
        // → flags = 0b00000010 = 0x02
        let mut data = vec![0x02];
        data.extend_from_slice(&42i64.to_le_bytes()); // executor_id
        push_kryo_string(&mut data, "testMethod");
        push_kryo_string(&mut data, "com.example.MyTest");
        push_kryo_string(&mut data, "MyExecutor");

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.executor_id, 42);
            assert_eq!(e.class_name, "com.example.MyTest");
            assert_eq!(e.method_name.as_deref(), Some("testMethod"));
            assert_eq!(e.executor_name.as_deref(), Some("MyExecutor"));
        } else {
            panic!("expected TestCase");
        }
    }

    #[test]
    fn test_decode_class_level() {
        // bit 0 (executor_id)=0 present, bit 1 (secondary_id)=1 absent,
        // bit 2 (name1)=0 present, bit 3 (name2)=0 present, bit 4 (name3)=1 absent
        // → flags = 0b00010010 = 0x12
        let mut data = vec![0x12];
        data.extend_from_slice(&7i64.to_le_bytes()); // executor_id
        push_kryo_string(&mut data, "com.example.MyTest");
        push_kryo_string(&mut data, "MyTest");

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.executor_id, 7);
            assert_eq!(e.class_name, "com.example.MyTest");
            assert_eq!(e.method_name, None);
            assert_eq!(e.executor_name, None);
        } else {
            panic!("expected TestCase");
        }
    }
}
