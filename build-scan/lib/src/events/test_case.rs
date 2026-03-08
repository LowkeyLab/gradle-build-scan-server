use error::ParseError;
use models::ExecutorId;

use super::{BodyDecoder, DecodedEvent, TestCaseEvent};

/// Decoder for wire 798 (TestCase) events.
///
/// Three event subtypes are encoded on wire 798, distinguished by the flags byte
/// and the count of Kryo-long ID fields present:
///
/// **flags = 0x00 (class-level)**
/// ```text
/// [0]     flags = 0x00
/// [1..]   ID1  (Kryo-long)
/// [..]    ID2  (Kryo-long)
/// [..]    ID3  (Kryo-long)
/// [..]    executor_id  (Kryo-long)
/// [..]    class_name  (Kryo interned string)
/// [..]    bool (1 byte, 0x01)
/// [..]    short_class_name (Kryo interned string)
/// [..]    null (1 byte, 0x00)
/// [..]    back_ref_id (Kryo-long, links back to executor)
/// [..]    null (1 byte, 0x00)
/// ```
///
/// **flags = 0x05, next byte after 3 IDs has bit7=1 (method-level)**
/// ```text
/// [0]     flags = 0x05
/// [1..]   ID1  (Kryo-long)
/// [..]    ID2  (Kryo-long)
/// [..]    ID3  (Kryo-long)
/// [..]    ID4  (Kryo-long)   ← present when next byte has bit7=1
/// [..]    method_name  (Kryo interned string)
/// [..]    class_name   (Kryo interned string)
/// [..]    executor_id  (Kryo-long)
/// ```
///
/// **flags = 0x05, next byte after 3 IDs has bit7=0 (executor-init)**
/// ```text
/// [0]     flags = 0x05
/// [1..]   ID1  (Kryo-long)
/// [..]    ID2  (Kryo-long)
/// [..]    ID3  (Kryo-long)
/// [..]    executor_name (Kryo interned string)
/// [..]    executor_id   (Kryo-long)
/// ```
pub struct TestCaseDecoder;

impl BodyDecoder for TestCaseDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        match flags {
            0x00 => {
                // Class-level event: 4 Kryo-long IDs, then class name, bool, short name,
                // null, executor_id, null
                let _id1 = kryo::read_kryo_long(body, &mut pos)?;
                let _id2 = kryo::read_kryo_long(body, &mut pos)?;
                let _id3 = kryo::read_kryo_long(body, &mut pos)?;
                let executor_id = ExecutorId::new(kryo::read_kryo_long(body, &mut pos)?);
                let class_name = table.read_string(body, &mut pos)?;
                // bool byte (0x01 = has short name)
                if pos < body.len() {
                    pos += 1;
                }
                let _short_class = table.read_string(body, &mut pos)?;
                // null terminator
                if pos < body.len() {
                    pos += 1;
                }
                // trailing executor_id Kryo-long (same as ID4 — links back to executor)
                let _trailing_id = kryo::read_kryo_long(body, &mut pos)?;
                // final null
                if pos < body.len() {
                    pos += 1;
                }
                let _ = pos;
                Ok(DecodedEvent::TestCase(TestCaseEvent {
                    executor_id,
                    class_name,
                    method_name: None,
                    executor_name: None,
                }))
            }
            0x05 => {
                // Method-level or executor-init event: 3 Kryo-long IDs always present.
                // If the byte after the 3rd ID has bit7=1, a 4th ID follows (method-level).
                // Otherwise the next data is a string (executor-init).
                let _id1 = kryo::read_kryo_long(body, &mut pos)?;
                let _id2 = kryo::read_kryo_long(body, &mut pos)?;
                let _id3 = kryo::read_kryo_long(body, &mut pos)?;

                // Peek: bit7=1 → 4th Kryo-long (method-level); bit7=0 → string starts
                let has_fourth_id = pos < body.len() && (body[pos] & 0x80 != 0);

                if has_fourth_id {
                    // Method-level: ID4 then method_name, class_name, executor_id
                    let executor_id = ExecutorId::new(kryo::read_kryo_long(body, &mut pos)?);
                    let method_name = table.read_string(body, &mut pos)?;
                    let class_name = table.read_string(body, &mut pos)?;
                    let _trailing_id = kryo::read_kryo_long(body, &mut pos)?;
                    let _ = pos;
                    Ok(DecodedEvent::TestCase(TestCaseEvent {
                        executor_id,
                        class_name,
                        method_name: Some(method_name),
                        executor_name: None,
                    }))
                } else {
                    // Executor-init: executor_name string, then trailing executor_id
                    let executor_name = table.read_string(body, &mut pos)?;
                    let _trailing_id = kryo::read_kryo_long(body, &mut pos)?;
                    let _ = pos;
                    Ok(DecodedEvent::TestCase(TestCaseEvent {
                        executor_id: ExecutorId::new(0),
                        class_name: String::new(),
                        method_name: None,
                        executor_name: Some(executor_name),
                    }))
                }
            }
            _ => Err(ParseError::InvalidHeader {
                reason: "unsupported TestCase flags value",
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_kryo_string(data: &mut Vec<u8>, s: &str) {
        // zigzag-encoded length (positive → ZigZag(n) = n*2), then per-char unsigned varints
        let len = s.len() as u64;
        let zigzag = len * 2;
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
        // ASCII chars are single-byte unsigned varints
        data.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn test_decode_method_level() {
        // flags=0x05, then 3 IDs, then ID4 (bit7=1 in first byte → method-level),
        // then method_name, class_name, trailing_id
        let mut data = vec![0x05u8];
        data.extend_from_slice(&kryo::encode_kryo_long(1111)); // ID1
        data.extend_from_slice(&kryo::encode_kryo_long(50)); // ID2 (1-byte)
        data.extend_from_slice(&kryo::encode_kryo_long(2222)); // ID3
        data.extend_from_slice(&kryo::encode_kryo_long(-9000000000i64)); // ID4 (9-byte, bit7=1 in first byte)
        push_kryo_string(&mut data, "testMethod()");
        push_kryo_string(&mut data, "com.example.MyTest");
        data.extend_from_slice(&kryo::encode_kryo_long(42)); // trailing executor_id

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(-9000000000i64));
            assert_eq!(e.class_name, "com.example.MyTest");
            assert_eq!(e.method_name.as_deref(), Some("testMethod()"));
            assert_eq!(e.executor_name, None);
        } else {
            panic!("expected TestCase");
        }
    }

    #[test]
    fn test_decode_class_level() {
        // flags=0x00, then 4 IDs, then class_name, bool, short_name, null, trailing_id, null
        let mut data = vec![0x00u8];
        data.extend_from_slice(&kryo::encode_kryo_long(32)); // ID1 (1-byte)
        data.extend_from_slice(&kryo::encode_kryo_long(1111)); // ID2
        data.extend_from_slice(&kryo::encode_kryo_long(2222)); // ID3
        data.extend_from_slice(&kryo::encode_kryo_long(3333)); // ID4 (executor_id)
        push_kryo_string(&mut data, "com.example.MyTest");
        data.push(0x01); // bool
        push_kryo_string(&mut data, "MyTest");
        data.push(0x00); // null
        data.extend_from_slice(&kryo::encode_kryo_long(999)); // trailing executor_id
        data.push(0x00); // final null

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(3333));
            assert_eq!(e.class_name, "com.example.MyTest");
            assert_eq!(e.method_name, None);
            assert_eq!(e.executor_name, None);
        } else {
            panic!("expected TestCase");
        }
    }

    #[test]
    fn test_decode_executor_init() {
        // flags=0x05, then 3 IDs, then executor_name (bit7=0 in first byte → executor-init),
        // then trailing_id
        let mut data = vec![0x05u8];
        data.extend_from_slice(&kryo::encode_kryo_long(1111)); // ID1
        data.extend_from_slice(&kryo::encode_kryo_long(50)); // ID2
        data.extend_from_slice(&kryo::encode_kryo_long(3333)); // ID3
        // executor name string — first byte has bit7=0 (it's a length, not a Kryo-long)
        push_kryo_string(&mut data, "Gradle Test Executor 1");
        data.extend_from_slice(&kryo::encode_kryo_long(304549416674991952i64)); // trailing executor_id

        let decoder = TestCaseDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestCase(e) = result {
            assert_eq!(e.executor_id, ExecutorId::new(0));
            assert_eq!(e.class_name, "");
            assert_eq!(e.method_name, None);
            assert_eq!(e.executor_name.as_deref(), Some("Gradle Test Executor 1"));
        } else {
            panic!("expected TestCase");
        }
    }
}
