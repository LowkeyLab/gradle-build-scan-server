use error::ParseError;

/// Cap a decoded length against the remaining buffer to prevent OOM from corrupt varints.
fn check_len(len: usize, data: &[u8], pos: usize) -> Result<usize, ParseError> {
    let remaining = data.len().saturating_sub(pos);
    if len > remaining {
        return Err(ParseError::UnexpectedEof { offset: pos });
    }
    Ok(len)
}

pub struct StringInternTable {
    strings: Vec<String>,
}

impl Default for StringInternTable {
    fn default() -> Self {
        Self::new()
    }
}

impl StringInternTable {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    /// ZigZag varint: >= 0 = new string (char count), < 0 = back-ref (index = -1 - value)
    /// Characters: unsigned LEB128 varints (ASCII = 1 byte each)
    /// Scope: per-event body (fresh table per decode call)
    pub fn read_string(&mut self, data: &[u8], pos: &mut usize) -> Result<String, ParseError> {
        let raw = varint::read_zigzag_i32(data, pos)?;
        if raw < 0 {
            // Back-reference: index = -1 - raw
            let index = (-1 - raw) as usize;
            self.strings
                .get(index)
                .cloned()
                .ok_or(ParseError::InvalidStringRef { index })
        } else {
            // New string: raw = character count
            let char_count = check_len(raw as usize, data, *pos)?;
            let mut s = String::with_capacity(char_count);
            for _ in 0..char_count {
                let ch = varint::read_unsigned_varint(data, pos)? as u32;
                let c = char::from_u32(ch).ok_or(ParseError::InvalidUtf8)?;
                s.push(c);
            }
            self.strings.push(s.clone());
            Ok(s)
        }
    }
}

/// Read flags as unsigned varint, return as u8 (for <= 8 fields)
pub fn read_flags_byte(data: &[u8], pos: &mut usize) -> Result<u8, ParseError> {
    Ok(varint::read_unsigned_varint(data, pos)? as u8)
}

/// Read flags as fixed big-endian u16 (2 bytes, for TaskFinished-style bodies)
pub fn read_flags_u16_be(data: &[u8], pos: &mut usize) -> Result<u16, ParseError> {
    if *pos + 2 > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let flags = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(flags)
}

/// Read a task identity/correlation id as a fixed little-endian i64 (8 bytes)
pub fn read_task_id(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    if *pos + 8 > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let id = i64::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
        data[*pos + 4],
        data[*pos + 5],
        data[*pos + 6],
        data[*pos + 7],
    ]);
    *pos += 8;
    Ok(id)
}

/// Read a Kryo-encoded long (zigzag varint, up to 9 bytes).
///
/// Kryo's long encoding differs from standard LEB128 in one key way: bytes 0–7
/// use bit 7 as a continuation flag, but byte 8 (the 9th byte) is consumed with
/// **all 8 bits** as data — there is no 10th byte. The encoded bits are zigzag-decoded
/// at the end: `decoded = (raw >> 1) ^ -(raw & 1)`.
pub fn read_kryo_long(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    if *pos >= data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let b0 = data[*pos];
    *pos += 1;
    let mut result = (b0 & 0x7F) as u64;
    if b0 & 0x80 != 0 {
        if *pos >= data.len() {
            return Err(ParseError::UnexpectedEof { offset: *pos });
        }
        let b1 = data[*pos];
        *pos += 1;
        result |= ((b1 & 0x7F) as u64) << 7;
        if b1 & 0x80 != 0 {
            if *pos >= data.len() {
                return Err(ParseError::UnexpectedEof { offset: *pos });
            }
            let b2 = data[*pos];
            *pos += 1;
            result |= ((b2 & 0x7F) as u64) << 14;
            if b2 & 0x80 != 0 {
                if *pos >= data.len() {
                    return Err(ParseError::UnexpectedEof { offset: *pos });
                }
                let b3 = data[*pos];
                *pos += 1;
                result |= ((b3 & 0x7F) as u64) << 21;
                if b3 & 0x80 != 0 {
                    if *pos >= data.len() {
                        return Err(ParseError::UnexpectedEof { offset: *pos });
                    }
                    let b4 = data[*pos];
                    *pos += 1;
                    result |= ((b4 & 0x7F) as u64) << 28;
                    if b4 & 0x80 != 0 {
                        if *pos >= data.len() {
                            return Err(ParseError::UnexpectedEof { offset: *pos });
                        }
                        let b5 = data[*pos];
                        *pos += 1;
                        result |= ((b5 & 0x7F) as u64) << 35;
                        if b5 & 0x80 != 0 {
                            if *pos >= data.len() {
                                return Err(ParseError::UnexpectedEof { offset: *pos });
                            }
                            let b6 = data[*pos];
                            *pos += 1;
                            result |= ((b6 & 0x7F) as u64) << 42;
                            if b6 & 0x80 != 0 {
                                if *pos >= data.len() {
                                    return Err(ParseError::UnexpectedEof { offset: *pos });
                                }
                                let b7 = data[*pos];
                                *pos += 1;
                                result |= ((b7 & 0x7F) as u64) << 49;
                                if b7 & 0x80 != 0 {
                                    // 9th byte: all 8 bits are data (no continuation flag)
                                    if *pos >= data.len() {
                                        return Err(ParseError::UnexpectedEof { offset: *pos });
                                    }
                                    let b8 = data[*pos];
                                    *pos += 1;
                                    result |= (b8 as u64) << 56;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(varint::zigzag_decode_i64(result))
}

/// Read a Kryo-encoded long with `optimizePositive=true` (no zigzag, up to 9 bytes).
///
/// Same wire format as `read_kryo_long` but the raw value is reinterpreted
/// directly as `i64` instead of being zigzag-decoded. This matches Kryo's
/// `Input.readLong(true)` / `Output.writeLong(value, true)`. For example,
/// nine `0xFF` bytes → `u64::MAX` → `i64` value `-1`, which the JVM uses
/// as a sentinel for "undefined" memory pool max.
pub fn read_kryo_long_unsigned(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    if *pos >= data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let b0 = data[*pos];
    *pos += 1;
    let mut result = (b0 & 0x7F) as u64;
    if b0 & 0x80 != 0 {
        if *pos >= data.len() {
            return Err(ParseError::UnexpectedEof { offset: *pos });
        }
        let b1 = data[*pos];
        *pos += 1;
        result |= ((b1 & 0x7F) as u64) << 7;
        if b1 & 0x80 != 0 {
            if *pos >= data.len() {
                return Err(ParseError::UnexpectedEof { offset: *pos });
            }
            let b2 = data[*pos];
            *pos += 1;
            result |= ((b2 & 0x7F) as u64) << 14;
            if b2 & 0x80 != 0 {
                if *pos >= data.len() {
                    return Err(ParseError::UnexpectedEof { offset: *pos });
                }
                let b3 = data[*pos];
                *pos += 1;
                result |= ((b3 & 0x7F) as u64) << 21;
                if b3 & 0x80 != 0 {
                    if *pos >= data.len() {
                        return Err(ParseError::UnexpectedEof { offset: *pos });
                    }
                    let b4 = data[*pos];
                    *pos += 1;
                    result |= ((b4 & 0x7F) as u64) << 28;
                    if b4 & 0x80 != 0 {
                        if *pos >= data.len() {
                            return Err(ParseError::UnexpectedEof { offset: *pos });
                        }
                        let b5 = data[*pos];
                        *pos += 1;
                        result |= ((b5 & 0x7F) as u64) << 35;
                        if b5 & 0x80 != 0 {
                            if *pos >= data.len() {
                                return Err(ParseError::UnexpectedEof { offset: *pos });
                            }
                            let b6 = data[*pos];
                            *pos += 1;
                            result |= ((b6 & 0x7F) as u64) << 42;
                            if b6 & 0x80 != 0 {
                                if *pos >= data.len() {
                                    return Err(ParseError::UnexpectedEof { offset: *pos });
                                }
                                let b7 = data[*pos];
                                *pos += 1;
                                result |= ((b7 & 0x7F) as u64) << 49;
                                if b7 & 0x80 != 0 {
                                    if *pos >= data.len() {
                                        return Err(ParseError::UnexpectedEof { offset: *pos });
                                    }
                                    let b8 = data[*pos];
                                    *pos += 1;
                                    result |= (b8 as u64) << 56;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(result as i64)
}

/// Inverted: bit=0 means field IS present
pub fn is_field_present(flags: u16, bit: u8) -> bool {
    (flags >> bit) & 1 == 0
}

/// Read enum ordinal as unsigned varint
pub fn read_enum_ordinal(data: &[u8], pos: &mut usize) -> Result<u64, ParseError> {
    varint::read_unsigned_varint(data, pos)
}

/// Read a byte array: unsigned varint length, then that many bytes
pub fn read_byte_array(data: &[u8], pos: &mut usize) -> Result<Vec<u8>, ParseError> {
    let len = varint::read_unsigned_varint(data, pos)? as usize;
    if *pos + len > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let bytes = data[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(bytes)
}

/// Read a list of fixed 8-byte LE i64 values: varint length prefix, then N × 8 bytes
pub fn read_list_of_i64(data: &[u8], pos: &mut usize) -> Result<Vec<i64>, ParseError> {
    let len = check_len(
        varint::read_unsigned_varint(data, pos)? as usize,
        data,
        *pos,
    )?;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(read_task_id(data, pos)?);
    }
    Ok(result)
}

/// Read a list of byte arrays: varint length prefix, then N byte arrays
pub fn read_list_of_byte_arrays(data: &[u8], pos: &mut usize) -> Result<Vec<Vec<u8>>, ParseError> {
    let len = check_len(
        varint::read_unsigned_varint(data, pos)? as usize,
        data,
        *pos,
    )?;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(read_byte_array(data, pos)?);
    }
    Ok(result)
}

/// Read a list of interned strings: varint length prefix, then N interned strings
pub fn read_list_of_interned_strings(
    data: &[u8],
    pos: &mut usize,
    table: &mut StringInternTable,
) -> Result<Vec<String>, ParseError> {
    let len = check_len(
        varint::read_unsigned_varint(data, pos)? as usize,
        data,
        *pos,
    )?;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(table.read_string(data, pos)?);
    }
    Ok(result)
}

/// Read a zigzag-encoded varint i64 (used by events that encode IDs as zigzag varints)
pub fn read_zigzag_i64(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    varint::read_zigzag_i64(data, pos)
}

/// Read an unsigned varint as i64. Matches Kryo's readLong(optimizePositive=true).
pub fn read_positive_varint_i64(data: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    Ok(varint::read_unsigned_varint(data, pos)? as i64)
}

/// Read an unsigned varint as i32. Matches Kryo's readInt(optimizePositive=true). The u64→i32 cast wraps via truncation, correctly recovering negative values encoded as their unsigned 32-bit representation.
pub fn read_positive_varint_i32(data: &[u8], pos: &mut usize) -> Result<i32, ParseError> {
    Ok(varint::read_unsigned_varint(data, pos)? as i32)
}

/// Read a list of unsigned varint i32 values: varint length prefix, then N unsigned varints
pub fn read_list_of_positive_varint_i32(
    data: &[u8],
    pos: &mut usize,
) -> Result<Vec<i32>, ParseError> {
    let len = check_len(
        varint::read_unsigned_varint(data, pos)? as usize,
        data,
        *pos,
    )?;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(read_positive_varint_i32(data, pos)?);
    }
    Ok(result)
}

/// Read a nested list of varint i32 (e.g. IndexedNormalizedSamples.indices = List<List<Integer>>):
/// outer varint count, then for each inner list: varint count + N varints
pub fn read_list_of_list_of_i32(data: &[u8], pos: &mut usize) -> Result<Vec<Vec<i32>>, ParseError> {
    let outer_len = check_len(
        varint::read_unsigned_varint(data, pos)? as usize,
        data,
        *pos,
    )?;
    let mut result = Vec::with_capacity(outer_len);
    for _ in 0..outer_len {
        result.push(read_list_of_positive_varint_i32(data, pos)?);
    }
    Ok(result)
}

pub fn encode_zigzag_i64(n: i64) -> Vec<u8> {
    let zigzag = ((n << 1) ^ (n >> 63)) as u64;
    let mut buf = Vec::new();
    let mut value = zigzag;
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

/// Encode an i64 as a Kryo-long (zigzag varint, up to 9 bytes).
/// Matches `read_kryo_long` — the inverse operation used in tests.
pub fn encode_kryo_long(n: i64) -> Vec<u8> {
    let zigzag = ((n << 1) ^ (n >> 63)) as u64;
    let mut result = zigzag;
    let mut buf = Vec::new();
    for _ in 0..8 {
        let b = (result & 0x7F) as u8;
        result >>= 7;
        if result != 0 {
            buf.push(b | 0x80);
        } else {
            buf.push(b);
            return buf;
        }
    }
    // 9th byte: all 8 bits are data
    buf.push((result & 0xFF) as u8);
    buf
}

pub fn encode_unsigned_varint(n: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut value = n;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_kryo_long_small_positive() {
        // ZigZag(50) = 100 = 0x64, single byte, bit7=0
        let data = [0x64u8];
        let mut pos = 0;
        assert_eq!(read_kryo_long(&data, &mut pos).unwrap(), 50);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_kryo_long_small_negative() {
        // ZigZag(-49) = 97 = 0x61, single byte
        let data = [0x61u8];
        let mut pos = 0;
        assert_eq!(read_kryo_long(&data, &mut pos).unwrap(), -49);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_kryo_long_nine_byte_roundtrip() {
        // A value that requires all 9 bytes: large magnitude i64
        let value: i64 = -8568844071650005894;
        let encoded = encode_kryo_long(value);
        assert_eq!(
            encoded.len(),
            9,
            "expected 9-byte encoding for large magnitude"
        );
        // The first 8 bytes must all have bit7=1 (continuation), 9th must not be checked
        for b in &encoded[..8] {
            assert!(b & 0x80 != 0, "bytes 0-7 must have bit7=1");
        }
        let mut pos = 0;
        assert_eq!(read_kryo_long(&encoded, &mut pos).unwrap(), value);
        assert_eq!(pos, 9);
    }

    #[test]
    fn test_encode_decode_kryo_long_roundtrip() {
        for &n in &[
            0i64,
            1,
            -1,
            50,
            -49,
            i64::MAX,
            i64::MIN,
            -9000000000i64,
            304549416674991952i64,
        ] {
            let encoded = encode_kryo_long(n);
            let mut pos = 0;
            let decoded = read_kryo_long(&encoded, &mut pos).unwrap();
            assert_eq!(decoded, n, "roundtrip failed for {n}");
            assert_eq!(pos, encoded.len());
        }
    }

    #[test]
    fn test_read_kryo_long_real_payload_bytes() {
        // Bytes from real wire 798 event #3 body[11:20] — known to decode to 2348116984554971701
        let data = [0xea, 0x88, 0xaf, 0xb7, 0x9c, 0xfd, 0x97, 0x96, 0x41];
        let mut pos = 0;
        assert_eq!(
            read_kryo_long(&data, &mut pos).unwrap(),
            2348116984554971701i64
        );
        assert_eq!(pos, 9);
    }

    #[test]
    fn test_read_kryo_long_real_payload_bytes_negative() {
        // Bytes from real wire 798 event #3 body[20:29] — known to decode to -8568844071650005894
        let data = [0x8b, 0xce, 0xc8, 0xa6, 0x92, 0x92, 0xd3, 0xea, 0xed];
        let mut pos = 0;
        assert_eq!(
            read_kryo_long(&data, &mut pos).unwrap(),
            -8568844071650005894i64
        );
        assert_eq!(pos, 9);
    }

    #[test]
    fn test_read_kryo_long_unsigned_single_byte() {
        // 100 fits in 7 data bits → single byte 0x64
        let data = [0x64u8];
        let mut pos = 0;
        assert_eq!(read_kryo_long_unsigned(&data, &mut pos).unwrap(), 100);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_kryo_long_unsigned_multi_byte() {
        // 512 → 0x80 0x04 (continuation bit set on first byte)
        let data = [0x80u8, 0x04];
        let mut pos = 0;
        assert_eq!(read_kryo_long_unsigned(&data, &mut pos).unwrap(), 512);
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_read_kryo_long_unsigned_nine_byte_sentinel() {
        // JVM "-1 = undefined" sentinel: nine 0xFF bytes → u64::MAX → i64 -1
        let data = [0xFFu8; 9];
        let mut pos = 0;
        assert_eq!(read_kryo_long_unsigned(&data, &mut pos).unwrap(), -1);
        assert_eq!(pos, 9);
    }

    #[test]
    fn test_flags_inverted() {
        assert!(is_field_present(0x00, 0));
        assert!(is_field_present(0x00, 1));
        assert!(!is_field_present(0x01, 0));
        assert!(is_field_present(0x01, 1));
    }

    #[test]
    fn test_string_intern_new_ascii() {
        // ZigZag(3) = 6, then chars 'f'=102, 'o'=111, 'o'=111
        let data = [0x06, 0x66, 0x6f, 0x6f];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_string_intern_back_reference() {
        // First: write "foo" (zigzag(3)=6, then chars)
        // Then: back-ref to index 0 → zigzag(-1) = 1
        let data = [0x06, 0x66, 0x6f, 0x6f, 0x01];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "foo");
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_string_intern_empty_string() {
        // ZigZag(0) = 0, no chars follow
        let data = [0x00];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "");
    }

    #[test]
    fn test_string_intern_multiple_refs() {
        // "abc" then "xyz" then ref(0) then ref(1)
        let data = [
            0x06, 97, 98, 99, // "abc"
            0x06, 120, 121, 122,  // "xyz"
            0x01, // ref(0) = "abc"
            0x03, // ref(1) = "xyz"
        ];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "abc");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "xyz");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "abc");
        assert_eq!(table.read_string(&data, &mut pos).unwrap(), "xyz");
    }

    #[test]
    fn test_read_byte_array() {
        let data = [0x03, 0xAA, 0xBB, 0xCC];
        let mut pos = 0;
        assert_eq!(
            read_byte_array(&data, &mut pos).unwrap(),
            vec![0xAA, 0xBB, 0xCC]
        );
    }

    #[test]
    fn test_read_flags_u16_be() {
        // 0x1FF8 big-endian = [0x1F, 0xF8]
        let data = [0x1F, 0xF8, 0x00];
        let mut pos = 0;
        assert_eq!(read_flags_u16_be(&data, &mut pos).unwrap(), 0x1FF8);
        assert_eq!(pos, 2);
    }

    #[test]
    fn test_read_flags_u16_be_eof() {
        let data = [0x1F];
        let mut pos = 0;
        assert!(read_flags_u16_be(&data, &mut pos).is_err());
    }

    #[test]
    fn test_read_task_id() {
        // 1i64 in little-endian
        let id_bytes = 1i64.to_le_bytes();
        let mut pos = 0;
        assert_eq!(read_task_id(&id_bytes, &mut pos).unwrap(), 1i64);
        assert_eq!(pos, 8);
    }

    #[test]
    fn test_read_task_id_negative() {
        // -6048516917597647557i64 in little-endian (from reference payload)
        let id: i64 = -6048516917597647557;
        let id_bytes = id.to_le_bytes();
        let mut pos = 0;
        assert_eq!(read_task_id(&id_bytes, &mut pos).unwrap(), id);
        assert_eq!(pos, 8);
    }

    #[test]
    fn test_read_task_id_eof() {
        let data = [0x01, 0x02, 0x03]; // only 3 bytes, need 8
        let mut pos = 0;
        assert!(read_task_id(&data, &mut pos).is_err());
    }

    #[test]
    fn test_read_list_of_i64_empty() {
        let data = [0x00]; // length = 0
        let mut pos = 0;
        assert_eq!(read_list_of_i64(&data, &mut pos).unwrap(), vec![]);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_read_list_of_i64_two_elements() {
        let mut data = vec![0x02]; // length = 2
        data.extend_from_slice(&1i64.to_le_bytes());
        data.extend_from_slice(&(-5i64).to_le_bytes());
        let mut pos = 0;
        let result = read_list_of_i64(&data, &mut pos).unwrap();
        assert_eq!(result, vec![1i64, -5i64]);
        assert_eq!(pos, 17);
    }

    #[test]
    fn test_read_list_of_byte_arrays_empty() {
        let data = [0x00]; // length = 0
        let mut pos = 0;
        assert_eq!(
            read_list_of_byte_arrays(&data, &mut pos).unwrap(),
            Vec::<Vec<u8>>::new()
        );
    }

    #[test]
    fn test_read_list_of_byte_arrays_one() {
        let data = [0x01, 0x02, 0xAA, 0xBB]; // 1 array of 2 bytes
        let mut pos = 0;
        assert_eq!(
            read_list_of_byte_arrays(&data, &mut pos).unwrap(),
            vec![vec![0xAA, 0xBB]]
        );
    }

    #[test]
    fn test_read_list_of_interned_strings_empty() {
        let data = [0x00]; // length = 0
        let mut pos = 0;
        let mut table = StringInternTable::new();
        assert_eq!(
            read_list_of_interned_strings(&data, &mut pos, &mut table).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_read_list_of_interned_strings_with_backrefs() {
        // 2 strings: "foo" (new), then back-ref to "foo"
        let data = [
            0x02, // length = 2
            0x06, 0x66, 0x6f, 0x6f, // "foo" (zigzag(3)=6, then f,o,o)
            0x01, // back-ref to index 0
        ];
        let mut pos = 0;
        let mut table = StringInternTable::new();
        let result = read_list_of_interned_strings(&data, &mut pos, &mut table).unwrap();
        assert_eq!(result, vec!["foo".to_string(), "foo".to_string()]);
    }

    #[test]
    fn test_read_zigzag_i64_positive() {
        let data = encode_zigzag_i64(42);
        let mut pos = 0;
        assert_eq!(read_zigzag_i64(&data, &mut pos).unwrap(), 42);
    }

    #[test]
    fn test_read_zigzag_i64_negative() {
        let data = encode_zigzag_i64(-100);
        let mut pos = 0;
        assert_eq!(read_zigzag_i64(&data, &mut pos).unwrap(), -100);
    }

    #[test]
    fn test_read_zigzag_i64_zero() {
        let data = encode_zigzag_i64(0);
        let mut pos = 0;
        assert_eq!(read_zigzag_i64(&data, &mut pos).unwrap(), 0);
    }

    #[test]
    fn test_read_positive_varint_i64() {
        let data = encode_unsigned_varint(12345);
        let mut pos = 0;
        assert_eq!(read_positive_varint_i64(&data, &mut pos).unwrap(), 12345);
    }

    #[test]
    fn test_read_positive_varint_i64_zero() {
        let data = encode_unsigned_varint(0);
        let mut pos = 0;
        assert_eq!(read_positive_varint_i64(&data, &mut pos).unwrap(), 0);
    }

    #[test]
    fn test_read_positive_varint_i32() {
        let data = encode_unsigned_varint(999);
        let mut pos = 0;
        assert_eq!(read_positive_varint_i32(&data, &mut pos).unwrap(), 999);
    }

    #[test]
    fn test_read_positive_varint_i32_zero() {
        let data = encode_unsigned_varint(0);
        let mut pos = 0;
        assert_eq!(read_positive_varint_i32(&data, &mut pos).unwrap(), 0);
    }

    #[test]
    fn test_read_list_of_positive_varint_i32_empty() {
        let data = encode_unsigned_varint(0); // length = 0
        let mut pos = 0;
        assert_eq!(
            read_list_of_positive_varint_i32(&data, &mut pos).unwrap(),
            vec![]
        );
    }

    #[test]
    fn test_read_list_of_positive_varint_i32_multiple() {
        let mut data = encode_unsigned_varint(3); // length = 3
        data.extend_from_slice(&encode_unsigned_varint(10));
        data.extend_from_slice(&encode_unsigned_varint(20));
        data.extend_from_slice(&encode_unsigned_varint(30));
        let mut pos = 0;
        assert_eq!(
            read_list_of_positive_varint_i32(&data, &mut pos).unwrap(),
            vec![10, 20, 30]
        );
    }
}
