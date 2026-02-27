use error::ParseError;

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
            let char_count = raw as usize;
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
    let len = varint::read_unsigned_varint(data, pos)? as usize;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(read_task_id(data, pos)?);
    }
    Ok(result)
}

/// Read a list of byte arrays: varint length prefix, then N byte arrays
pub fn read_list_of_byte_arrays(data: &[u8], pos: &mut usize) -> Result<Vec<Vec<u8>>, ParseError> {
    let len = varint::read_unsigned_varint(data, pos)? as usize;
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
    let len = varint::read_unsigned_varint(data, pos)? as usize;
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
    let len = varint::read_unsigned_varint(data, pos)? as usize;
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push(read_positive_varint_i32(data, pos)?);
    }
    Ok(result)
}

/// Read a list of lists of varint i32: outer varint count, then for each inner list: varint count + N varints
pub fn read_list_of_list_of_i32(data: &[u8], pos: &mut usize) -> Result<Vec<Vec<i32>>, ParseError> {
    let outer_len = varint::read_unsigned_varint(data, pos)? as usize;
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
