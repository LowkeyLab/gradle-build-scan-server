use error::ParseError;

use super::{BodyDecoder, DecodedEvent, OsEvent};

pub struct OsDecoder;

/// Wire 16: Os_1_0 — 4 interned strings.
impl BodyDecoder for OsDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let family = if kryo::is_field_present(flags as u16, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let name = if kryo::is_field_present(flags as u16, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let version = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let arch = if kryo::is_field_present(flags as u16, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::Os(OsEvent {
            family,
            name,
            version,
            arch,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 4 bits present
        for s in &["linux", "Linux", "6.1.0", "amd64"] {
            let len = s.len();
            data.push((len as u8) * 2); // zigzag(len) = len*2
            data.extend_from_slice(s.as_bytes());
        }

        let decoder = OsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Os(e) = result {
            assert_eq!(e.family, Some("linux".into()));
            assert_eq!(e.name, Some("Linux".into()));
            assert_eq!(e.version, Some("6.1.0".into()));
            assert_eq!(e.arch, Some("amd64".into()));
        } else {
            panic!("expected Os");
        }
    }
}
