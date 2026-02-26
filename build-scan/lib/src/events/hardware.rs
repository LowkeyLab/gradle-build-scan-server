use error::ParseError;

use super::{BodyDecoder, DecodedEvent, HardwareEvent};

pub struct HardwareDecoder;

/// Wire 12: Hardware_1_0 — single unconditional int field, no flags.
impl BodyDecoder for HardwareDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let num_processors = kryo::read_positive_varint_i32(body, &mut pos)?;

        Ok(DecodedEvent::Hardware(HardwareEvent { num_processors }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_num_processors() {
        let data = [0x08]; // unsigned varint 8
        let decoder = HardwareDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Hardware(e) = result {
            assert_eq!(e.num_processors, 8);
        } else {
            panic!("expected Hardware");
        }
    }
}
