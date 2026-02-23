use error::ParseError;

pub struct OuterHeader {
    pub version: u16,
    pub tool_type: String,
    pub tool_version: String,
    pub plugin_version: String,
    pub gzip_offset: usize,
}

impl OuterHeader {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 6 {
            return Err(ParseError::InvalidHeader {
                reason: "too short for magic+version+blob_len",
            });
        }
        let magic = u16::from_be_bytes([data[0], data[1]]);
        if magic != 0x28C5 {
            return Err(ParseError::InvalidHeader {
                reason: "bad magic bytes",
            });
        }
        let version = u16::from_be_bytes([data[2], data[3]]);
        let blob_len = u16::from_be_bytes([data[4], data[5]]) as usize;
        let blob_end = 6 + blob_len;
        if data.len() < blob_end {
            return Err(ParseError::InvalidHeader {
                reason: "truncated tool version blob",
            });
        }
        let mut pos = 6;
        let tool_type = Self::read_utf(&data, &mut pos, blob_end)?;
        let tool_version = Self::read_utf(&data, &mut pos, blob_end)?;
        let plugin_version = Self::read_utf(&data, &mut pos, blob_end)?;
        Ok(Self {
            version,
            tool_type,
            tool_version,
            plugin_version,
            gzip_offset: blob_end,
        })
    }

    fn read_utf(data: &[u8], pos: &mut usize, limit: usize) -> Result<String, ParseError> {
        if *pos + 2 > limit {
            return Err(ParseError::InvalidHeader {
                reason: "truncated UTF string length",
            });
        }
        let len = u16::from_be_bytes([data[*pos], data[*pos + 1]]) as usize;
        *pos += 2;
        if *pos + len > limit {
            return Err(ParseError::InvalidHeader {
                reason: "truncated UTF string data",
            });
        }
        let s =
            std::str::from_utf8(&data[*pos..*pos + len]).map_err(|_| ParseError::InvalidUtf8)?;
        *pos += len;
        Ok(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference outer header bytes:
    // 28 c5 00 02 00 16 00 06 47 52 41 44 4c 45 00 05 39 2e 33 2e 31 00 05 34 2e 33 2e 32
    const HEADER_BYTES: [u8; 28] = [
        0x28, 0xc5, 0x00, 0x02, 0x00, 0x16, 0x00, 0x06, 0x47, 0x52, 0x41, 0x44, 0x4c,
        0x45, // "GRADLE"
        0x00, 0x05, 0x39, 0x2e, 0x33, 0x2e, 0x31, // "9.3.1"
        0x00, 0x05, 0x34, 0x2e, 0x33, 0x2e, 0x32, // "4.3.2"
    ];

    #[test]
    fn test_parse_reference_header() {
        let header = OuterHeader::parse(&HEADER_BYTES).unwrap();
        assert_eq!(header.version, 2);
        assert_eq!(header.tool_type, "GRADLE");
        assert_eq!(header.tool_version, "9.3.1");
        assert_eq!(header.plugin_version, "4.3.2");
        assert_eq!(header.gzip_offset, 28);
    }

    #[test]
    fn test_bad_magic() {
        let mut bad = HEADER_BYTES;
        bad[0] = 0x00;
        assert!(OuterHeader::parse(&bad).is_err());
    }

    #[test]
    fn test_truncated_header() {
        assert!(OuterHeader::parse(&HEADER_BYTES[..4]).is_err());
    }
}
