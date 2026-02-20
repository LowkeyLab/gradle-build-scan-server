use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use std::io::Read;

use error::ParseError;

#[derive(Debug, PartialEq, Clone)]
pub enum Primitive {
    Varint(u64),
    String(String),
    StringRef(u32),
    Timestamp(DateTime<Utc>),
}

pub struct StreamDecoder<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> StreamDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn decompress(raw_data: &[u8]) -> Result<Vec<u8>, ParseError> {
        let mut start_idx = 0;
        for i in 0..raw_data.len().saturating_sub(2) {
            if raw_data[i] == 0x1f && raw_data[i + 1] == 0x8b && raw_data[i + 2] == 0x08 {
                start_idx = i;
                break;
            }
        }

        let mut decoder = GzDecoder::new(&raw_data[start_idx..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|_| ParseError::InvalidGzip)?;

        Ok(decompressed)
    }

    pub fn read_leb128(&mut self) -> Result<u64, ParseError> {
        let mut result = 0u64;
        let mut shift = 0;

        loop {
            if self.offset >= self.data.len() {
                return Err(ParseError::UnexpectedEof);
            }

            let byte = self.data[self.offset];
            self.offset += 1;

            result |= ((byte & 0x7F) as u64) << shift;

            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err(ParseError::MalformedLeb128 {
                    offset: self.offset - 1,
                });
            }
        }

        Ok(result)
    }
    pub fn read_raw_varint(&mut self) -> Result<u64, ParseError> {
        self.read_leb128()
    }

    pub fn read_varint(&mut self) -> Result<Primitive, ParseError> {
        self.read_raw_varint().map(Primitive::Varint)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ParseError> {
        if self.offset + len > self.data.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let bytes = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_string(&mut self) -> Result<Primitive, ParseError> {
        let val = self.read_raw_varint()?;
        let bit = val & 1;
        let shifted = val >> 1;

        if bit == 1 {
            Ok(Primitive::StringRef(shifted as u32))
        } else {
            let len = shifted as usize;
            let bytes = self.read_bytes(len)?;
            let s = std::str::from_utf8(bytes).map_err(|_| ParseError::InvalidUtf8)?;
            Ok(Primitive::String(s.to_string()))
        }
    }

    pub fn read_timestamp(&mut self) -> Result<Primitive, ParseError> {
        let val = self.read_raw_varint()?;
        let dt = chrono::DateTime::from_timestamp(
            (val / 1000) as i64,
            ((val % 1000) * 1_000_000) as u32,
        )
        .ok_or(ParseError::InvalidTimestamp)?;
        Ok(Primitive::Timestamp(dt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leb128_decode() {
        let data = vec![0xE5, 0x8E, 0x26];
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(decoder.read_leb128().unwrap(), 624485);
    }

    #[test]
    fn test_explicit_string() {
        let mut data = vec![0x0A]; // length 5, shifted 1 = 10 (0x0A)
        data.extend_from_slice(b"hello");
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(
            decoder.read_string().unwrap(),
            Primitive::String("hello".to_string())
        );
    }

    #[test]
    fn test_explicit_string_ref() {
        let data = vec![0x55]; // ref 42 -> (42 << 1) | 1 = 85 (0x55)
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(decoder.read_string().unwrap(), Primitive::StringRef(42));
    }

    #[test]
    fn test_explicit_timestamp() {
        let ts_val: u64 = 1771622196842;
        // encode ts_val as varint bytes
        let mut data = Vec::new();
        let mut val = ts_val;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            data.push(byte);
            if val == 0 {
                break;
            }
        }
        let mut decoder = StreamDecoder::new(&data);
        let prim = decoder.read_timestamp().unwrap();
        if let Primitive::Timestamp(dt) = prim {
            assert_eq!(dt.timestamp_millis(), ts_val as i64);
        } else {
            panic!("Expected Timestamp primitive");
        }
    }
}
