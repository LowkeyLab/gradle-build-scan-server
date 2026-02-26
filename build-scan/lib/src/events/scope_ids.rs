use error::ParseError;

use super::{BodyDecoder, DecodedEvent, ScopeIdsEvent};

pub struct ScopeIdsDecoder;

/// Wire 39: ScopeIds_1_0 — 3 interned strings.
impl BodyDecoder for ScopeIdsDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let build_invocation_id = if kryo::is_field_present(flags as u16, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let workspace_id = if kryo::is_field_present(flags as u16, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let user_id = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::ScopeIds(ScopeIdsEvent {
            build_invocation_id,
            workspace_id,
            user_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 3 bits present
        // "abc" → zigzag(3)=6
        data.push(0x06);
        data.extend_from_slice(b"abc");
        // "def" → zigzag(3)=6
        data.push(0x06);
        data.extend_from_slice(b"def");
        // "ghi" → zigzag(3)=6
        data.push(0x06);
        data.extend_from_slice(b"ghi");

        let decoder = ScopeIdsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::ScopeIds(e) = result {
            assert_eq!(e.build_invocation_id, Some("abc".into()));
            assert_eq!(e.workspace_id, Some("def".into()));
            assert_eq!(e.user_id, Some("ghi".into()));
        } else {
            panic!("expected ScopeIds");
        }
    }
}
