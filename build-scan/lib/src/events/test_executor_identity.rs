use error::ParseError;

use super::{BodyDecoder, DecodedEvent, TestExecutorIdentityEvent};

pub struct TestExecutorIdentityDecoder;

impl BodyDecoder for TestExecutorIdentityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let executor_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };
        // Bit 1: hash — read and discard
        if kryo::is_field_present(flags as u16, 1) {
            let _ = kryo::read_task_id(body, &mut pos)?;
        }
        let name = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        Ok(DecodedEvent::TestExecutorIdentity(
            TestExecutorIdentityEvent { executor_id, name },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_fields() {
        // Flags are inverted: bit=0 means field IS present.
        // executor_id (bit 0)=0, hash (bit 1)=1 (absent/skip), name (bit 2)=0 → flags = 0x02
        let mut data = vec![0x02];
        data.extend_from_slice(&99i64.to_le_bytes()); // executor_id
        // name "Gradle Test Executor 1" → len=22, zigzag(22)=44=0x2C
        let name = "Gradle Test Executor 1";
        let zigzag_len = (name.len() as u64) * 2;
        data.push(zigzag_len as u8);
        data.extend_from_slice(name.as_bytes());

        let decoder = TestExecutorIdentityDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestExecutorIdentity(e) = result {
            assert_eq!(e.executor_id, 99);
            assert_eq!(e.name, "Gradle Test Executor 1");
        } else {
            panic!("expected TestExecutorIdentity");
        }
    }
}
