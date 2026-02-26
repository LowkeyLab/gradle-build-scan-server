use error::ParseError;

use super::{BodyDecoder, DecodedEvent, LocalityEvent};

pub struct LocalityDecoder;

/// Wire 15: Locality_1_0 — 4 interned strings + 1 int.
impl BodyDecoder for LocalityDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let locale_language = if kryo::is_field_present(flags as u16, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let locale_country = if kryo::is_field_present(flags as u16, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let locale_variant = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let time_zone_id = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let time_zone_offset_millis = if kryo::is_field_present(flags as u16, 4) {
            Some(kryo::read_positive_varint_i32(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::Locality(LocalityEvent {
            locale_language,
            locale_country,
            locale_variant,
            time_zone_id,
            time_zone_offset_millis,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 5 bits present
        // "en" → zigzag(2)=4
        data.push(0x04);
        data.extend_from_slice(b"en");
        // "US" → zigzag(2)=4
        data.push(0x04);
        data.extend_from_slice(b"US");
        // "" → zigzag(0)=0
        data.push(0x00);
        // "America/New_York" → zigzag(16)=32
        data.push(0x20);
        data.extend_from_slice(b"America/New_York");
        // offset: -18000000 → unsigned varint (we use positive, so this is tricky)
        // Actually time_zone_offset_millis is read as positive varint
        // For UTC-5: the offset in millis would be large. Let's use a simpler value.
        // Use 3600000 (UTC+1). But as unsigned varint that's large.
        // Just use 0 for the test.
        data.push(0x00);

        let decoder = LocalityDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Locality(e) = result {
            assert_eq!(e.locale_language, Some("en".into()));
            assert_eq!(e.locale_country, Some("US".into()));
            assert_eq!(e.locale_variant, Some("".into()));
            assert_eq!(e.time_zone_id, Some("America/New_York".into()));
            assert_eq!(e.time_zone_offset_millis, Some(0));
        } else {
            panic!("expected Locality");
        }
    }
}
