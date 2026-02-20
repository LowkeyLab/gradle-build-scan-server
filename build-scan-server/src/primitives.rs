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
}

impl<'a> Iterator for StreamDecoder<'a> {
    type Item = Result<Primitive, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let val = match self.read_leb128() {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let bit = val & 1;
        let shifted = val >> 1;

        if bit == 1 {
            // String ref
            Some(Ok(Primitive::StringRef(shifted as u32)))
        } else {
            // Heuristic for String: 2 <= len <= 500, and we have enough bytes
            if shifted >= 2 && shifted <= 500 && self.offset + (shifted as usize) <= self.data.len()
            {
                let len = shifted as usize;
                let str_bytes = &self.data[self.offset..self.offset + len];
                if let Ok(string) = std::str::from_utf8(str_bytes) {
                    self.offset += len;
                    return Some(Ok(Primitive::String(string.to_string())));
                }
            }

            if val >= 1_600_000_000_000 && val <= 1_900_000_000_000 {
                if let Some(dt) =
                    DateTime::from_timestamp((val / 1000) as i64, ((val % 1000) * 1_000_000) as u32)
                {
                    return Some(Ok(Primitive::Timestamp(dt)));
                }
            }

            Some(Ok(Primitive::Varint(val)))
        }
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
    fn test_next_string() {
        // "hello" -> len 5 -> val = 5 << 1 = 10 (0x0A)
        let mut data = vec![0x0A];
        data.extend_from_slice(b"hello");
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(
            decoder.next().unwrap().unwrap(),
            Primitive::String("hello".to_string())
        );
    }

    #[test]
    fn test_next_string_ref() {
        // ref 42 -> val = (42 << 1) | 1 = 85 (0x55)
        let data = vec![0x55];
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(decoder.next().unwrap().unwrap(), Primitive::StringRef(42));
    }

    #[test]
    fn test_next_varint() {
        // 0 -> len 0 -> val = 0. Heuristic expects len >= 2. So it should yield Varint(0).
        let data = vec![0x00];
        let mut decoder = StreamDecoder::new(&data);
        assert_eq!(decoder.next().unwrap().unwrap(), Primitive::Varint(0));
    }
}
