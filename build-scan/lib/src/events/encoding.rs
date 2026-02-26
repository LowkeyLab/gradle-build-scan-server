use error::ParseError;

use super::{BodyDecoder, DecodedEvent, EncodingEvent};

pub struct EncodingDecoder;

/// Wire 56: Encoding_1_0 — single unconditional interned string, no flags.
impl BodyDecoder for EncodingDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let mut table = kryo::StringInternTable::new();
        let default_charset = table.read_string(body, &mut pos)?;

        Ok(DecodedEvent::Encoding(EncodingEvent { default_charset }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_utf8() {
        // "UTF-8" → zigzag(5)=10, then chars
        let mut data = vec![0x0A];
        for &c in b"UTF-8" {
            data.push(c);
        }
        let decoder = EncodingDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Encoding(e) = result {
            assert_eq!(e.default_charset, "UTF-8");
        } else {
            panic!("expected Encoding");
        }
    }
}
