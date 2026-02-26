use error::ParseError;

use super::{BodyDecoder, DecodedEvent, OutputSpan, OutputStyledTextEvent};

pub struct OutputStyledTextEventDecoder;

impl BodyDecoder for OutputStyledTextEventDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        // Shared intern table across the composite event
        let mut table = kryo::StringInternTable::new();

        // Top-level flags FIRST (2 bits: bit 0 = spans, bit 1 = owner)
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        // Common sub-object: ALWAYS present (written before conditional fields)
        let common_flags = kryo::read_flags_byte(body, &mut pos)?;

        let category = if kryo::is_field_present(common_flags as u16, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let log_level = if kryo::is_field_present(common_flags as u16, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        // bit 0: spans → list of OutputSpan
        let spans = if kryo::is_field_present(flags as u16, 0) {
            let len = varint::read_unsigned_varint(body, &mut pos)? as usize;
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                // Each span has its own flags_byte (2 bits)
                let span_flags = kryo::read_flags_byte(body, &mut pos)?;
                let text = if kryo::is_field_present(span_flags as u16, 0) {
                    table.read_string(body, &mut pos)?
                } else {
                    String::new()
                };
                let style = if kryo::is_field_present(span_flags as u16, 1) {
                    Some(table.read_string(body, &mut pos)?)
                } else {
                    None
                };
                result.push(OutputSpan { text, style });
            }
            result
        } else {
            vec![]
        };

        // bit 1: owner → OutputOwnerRef (NO flags, both fields always written)
        let (owner_type, owner_id) = if kryo::is_field_present(flags as u16, 1) {
            let owner_type = kryo::read_enum_ordinal(body, &mut pos)?;
            let owner_id = table.read_string(body, &mut pos)?;
            (Some(owner_type), Some(owner_id))
        } else {
            (None, None)
        };

        Ok(DecodedEvent::OutputStyledText(OutputStyledTextEvent {
            category,
            log_level,
            spans,
            owner_type,
            owner_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_with_common_fields_and_one_span() {
        let mut data = vec![];
        // top-level flags = 0x00: spans and owner present
        data.push(0x00);
        // common_flags = 0x00: both category and log_level present
        data.push(0x00);
        // category = "LIFECYCLE" → zigzag(9)=18, then chars
        data.push(0x12);
        for &c in b"LIFECYCLE" {
            data.push(c);
        }
        // log_level = "INFO" → zigzag(4)=8, then chars
        data.push(0x08);
        for &c in b"INFO" {
            data.push(c);
        }
        // spans list: len=1
        data.push(0x01);
        // span_flags = 0x00: text and style present
        data.push(0x00);
        // text = "hello" → zigzag(5)=10, then chars
        data.push(0x0A);
        for &c in b"hello" {
            data.push(c);
        }
        // style = "bold" → zigzag(4)=8, then chars
        data.push(0x08);
        for &c in b"bold" {
            data.push(c);
        }
        // owner: type = 1 (enum ordinal)
        data.push(0x01);
        // owner id = "task1" → zigzag(5)=10, then chars
        data.push(0x0A);
        for &c in b"task1" {
            data.push(c);
        }

        let decoder = OutputStyledTextEventDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::OutputStyledText(e) = result {
            assert_eq!(e.category, Some("LIFECYCLE".to_string()));
            assert_eq!(e.log_level, Some("INFO".to_string()));
            assert_eq!(e.spans.len(), 1);
            assert_eq!(e.spans[0].text, "hello");
            assert_eq!(e.spans[0].style, Some("bold".to_string()));
            assert_eq!(e.owner_type, Some(1));
            assert_eq!(e.owner_id, Some("task1".to_string()));
        } else {
            panic!("expected OutputStyledText");
        }
    }

    #[test]
    fn test_decode_no_spans_no_owner() {
        let mut data = vec![];
        // top-level flags = 0x03: spans and owner absent
        data.push(0x03);
        // common_flags = 0x03: both category and log_level absent
        data.push(0x03);

        let decoder = OutputStyledTextEventDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::OutputStyledText(e) = result {
            assert_eq!(e.category, None);
            assert_eq!(e.log_level, None);
            assert!(e.spans.is_empty());
            assert_eq!(e.owner_type, None);
            assert_eq!(e.owner_id, None);
        } else {
            panic!("expected OutputStyledText");
        }
    }
}
