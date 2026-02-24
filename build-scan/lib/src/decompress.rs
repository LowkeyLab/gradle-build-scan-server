use flate2::read::GzDecoder;
use std::io::Read;

use error::ParseError;

pub struct Decompressor;

impl Decompressor {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn gzip_compress(data: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn test_decompress_valid_gzip() {
        let original = b"hello world";
        let compressed = gzip_compress(original);
        let result = Decompressor::decompress(&compressed).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_decompress_with_prefix() {
        let original = b"test data";
        let compressed = gzip_compress(original);
        let mut with_prefix = vec![0x00, 0x01, 0x02];
        with_prefix.extend_from_slice(&compressed);
        let result = Decompressor::decompress(&with_prefix).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_decompress_invalid_gzip() {
        let invalid = vec![0x00, 0x01, 0x02];
        let result = Decompressor::decompress(&invalid);
        assert!(result.is_err());
    }
}
