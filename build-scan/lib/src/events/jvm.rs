use error::ParseError;

use super::{BodyDecoder, DecodedEvent, JvmEvent};

pub struct JvmDecoder;

/// Wire 14: Jvm_1_0 — 9 interned strings, flags as u16 (9 bits).
impl BodyDecoder for JvmDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_u16_be(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let version = if kryo::is_field_present(flags, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let vendor = if kryo::is_field_present(flags, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let runtime_name = if kryo::is_field_present(flags, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let runtime_version = if kryo::is_field_present(flags, 3) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let class_version = if kryo::is_field_present(flags, 4) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let vm_info = if kryo::is_field_present(flags, 5) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let vm_name = if kryo::is_field_present(flags, 6) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let vm_version = if kryo::is_field_present(flags, 7) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let vm_vendor = if kryo::is_field_present(flags, 8) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::Jvm(JvmEvent {
            version,
            vendor,
            runtime_name,
            runtime_version,
            class_version,
            vm_info,
            vm_name,
            vm_version,
            vm_vendor,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_absent() {
        // flags = 0x01FF: all 9 bits set → all absent
        let data = [0x01, 0xFF];
        let decoder = JvmDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Jvm(e) = result {
            assert!(e.version.is_none());
            assert!(e.vendor.is_none());
            assert!(e.vm_vendor.is_none());
        } else {
            panic!("expected Jvm");
        }
    }

    #[test]
    fn test_decode_version_only() {
        // Only bit 0 present (=0), bits 1-8 absent (=1)
        // flags = 0b0000_0001_1111_1110 = 0x01FE
        let mut data = vec![0x01, 0xFE];
        // version: "21" → zigzag(2)=4
        data.push(0x04);
        data.extend_from_slice(b"21");

        let decoder = JvmDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::Jvm(e) = result {
            assert_eq!(e.version, Some("21".into()));
            assert!(e.vendor.is_none());
        } else {
            panic!("expected Jvm");
        }
    }
}
