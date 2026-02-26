use error::ParseError;

use super::{BodyDecoder, DecodedEvent, JavaToolchainUsageEvent};

pub struct JavaToolchainUsageDecoder;

impl BodyDecoder for JavaToolchainUsageDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let task_id = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };

        let toolchain_id = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_zigzag_i64(body, &mut pos)?
        } else {
            0
        };

        let tool_name = if kryo::is_field_present(flags as u16, 2) {
            table.read_string(body, &mut pos)?
        } else {
            String::new()
        };

        Ok(DecodedEvent::JavaToolchainUsage(JavaToolchainUsageEvent {
            task_id,
            toolchain_id,
            tool_name,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kryo::encode_zigzag_i64;

    #[test]
    fn test_decode_all_present() {
        // flags = 0x00: all three bits present
        let mut data = vec![0x00];
        data.extend_from_slice(&encode_zigzag_i64(42)); // task_id = 42
        data.extend_from_slice(&encode_zigzag_i64(7)); // toolchain_id = 7
        // tool_name = "javac" → zigzag(5)=10, then chars
        data.push(0x0A);
        for &c in b"javac" {
            data.push(c);
        }

        let decoder = JavaToolchainUsageDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::JavaToolchainUsage(e) = result {
            assert_eq!(e.task_id, 42);
            assert_eq!(e.toolchain_id, 7);
            assert_eq!(e.tool_name, "javac");
        } else {
            panic!("expected JavaToolchainUsage");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        // flags = 0x07: all three bits set = all absent
        let data = vec![0x07];
        let decoder = JavaToolchainUsageDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::JavaToolchainUsage(e) = result {
            assert_eq!(e.task_id, 0);
            assert_eq!(e.toolchain_id, 0);
            assert_eq!(e.tool_name, "");
        } else {
            panic!("expected JavaToolchainUsage");
        }
    }
}
