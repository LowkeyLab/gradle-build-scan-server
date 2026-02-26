use error::ParseError;

use super::{BodyDecoder, DecodedEvent, JvmArgsEvent};

pub struct JvmArgsDecoder;

/// Wire 13: JvmArgs_1_0 — unconditional List<String>, no flags.
impl BodyDecoder for JvmArgsDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let mut table = kryo::StringInternTable::new();
        let effective = kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?;

        Ok(DecodedEvent::JvmArgs(JvmArgsEvent { effective }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_two_args() {
        let mut data = vec![0x02]; // 2 elements
        // "-Xmx512m" → zigzag(8)=16
        data.push(0x10);
        data.extend_from_slice(b"-Xmx512m");
        // "-Xms256m" → zigzag(8)=16
        data.push(0x10);
        data.extend_from_slice(b"-Xms256m");

        let decoder = JvmArgsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::JvmArgs(e) = result {
            assert_eq!(e.effective, vec!["-Xmx512m", "-Xms256m"]);
        } else {
            panic!("expected JvmArgs");
        }
    }

    #[test]
    fn test_decode_empty() {
        let data = [0x00]; // 0 elements
        let decoder = JvmArgsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::JvmArgs(e) = result {
            assert!(e.effective.is_empty());
        } else {
            panic!("expected JvmArgs");
        }
    }
}
