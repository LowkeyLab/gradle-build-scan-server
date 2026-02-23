use error::ParseError;

pub fn read_unsigned_varint(data: &[u8], pos: &mut usize) -> Result<u64, ParseError> {
    let start = *pos;
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= data.len() {
            return Err(ParseError::UnexpectedEof { offset: *pos });
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(ParseError::MalformedLeb128 { offset: start });
        }
    }
}

pub fn zigzag_decode_i32(n: u32) -> i32 {
    ((n >> 1) as i32) ^ -((n & 1) as i32)
}

pub fn zigzag_decode_i64(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

pub fn read_zigzag_i32(data: &[u8], pos: &mut usize) -> Result<i32, ParseError> {
    let raw = read_unsigned_varint(data, pos)?;
    Ok(zigzag_decode_i32(raw as u32))
}

pub fn read_zigzag_i64(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    let raw = read_unsigned_varint(data, pos)?;
    Ok(zigzag_decode_i64(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_unsigned_varint_single_byte() {
        let data = [0x05];
        let mut pos = 0;
        assert_eq!(read_unsigned_varint(&data, &mut pos).unwrap(), 5);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_unsigned_varint_two_bytes() {
        let data = [0x92, 0x04];
        let mut pos = 0;
        assert_eq!(read_unsigned_varint(&data, &mut pos).unwrap(), 530);
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_read_unsigned_varint_eof() {
        let data = [0x80];
        let mut pos = 0;
        assert!(read_unsigned_varint(&data, &mut pos).is_err());
    }

    #[test]
    fn test_zigzag_decode_i32() {
        assert_eq!(zigzag_decode_i32(0), 0);
        assert_eq!(zigzag_decode_i32(1), -1);
        assert_eq!(zigzag_decode_i32(2), 1);
        assert_eq!(zigzag_decode_i32(3), -2);
        assert_eq!(zigzag_decode_i32(530), 265);
        assert_eq!(zigzag_decode_i32(517), -259);
    }

    #[test]
    fn test_zigzag_decode_i64() {
        assert_eq!(zigzag_decode_i64(0), 0);
        assert_eq!(zigzag_decode_i64(1), -1);
        assert_eq!(zigzag_decode_i64(2), 1);
    }

    #[test]
    fn test_read_zigzag_i32() {
        let data = [0x92, 0x04];
        let mut pos = 0;
        assert_eq!(read_zigzag_i32(&data, &mut pos).unwrap(), 265);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn encode_unsigned_varint(mut value: u64) -> Vec<u8> {
            let mut buf = Vec::new();
            loop {
                let mut byte = (value & 0x7F) as u8;
                value >>= 7;
                if value != 0 {
                    byte |= 0x80;
                }
                buf.push(byte);
                if value == 0 {
                    break;
                }
            }
            buf
        }

        fn zigzag_encode_i32(n: i32) -> u32 {
            ((n << 1) ^ (n >> 31)) as u32
        }

        fn zigzag_encode_i64(n: i64) -> u64 {
            ((n << 1) ^ (n >> 63)) as u64
        }

        proptest! {
            #[test]
            fn roundtrip_unsigned_varint(value: u64) {
                let encoded = encode_unsigned_varint(value);
                let mut pos = 0;
                let decoded = read_unsigned_varint(&encoded, &mut pos).unwrap();
                prop_assert_eq!(decoded, value);
                prop_assert_eq!(pos, encoded.len());
            }

            #[test]
            fn roundtrip_zigzag_i32(value: i32) {
                let encoded = zigzag_encode_i32(value);
                let decoded = zigzag_decode_i32(encoded);
                prop_assert_eq!(decoded, value);
            }

            #[test]
            fn roundtrip_zigzag_i64(value: i64) {
                let encoded = zigzag_encode_i64(value);
                let decoded = zigzag_decode_i64(encoded);
                prop_assert_eq!(decoded, value);
            }
        }
    }
}
