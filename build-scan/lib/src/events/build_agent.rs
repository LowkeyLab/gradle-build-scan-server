use error::ParseError;

use super::{BodyDecoder, BuildAgentEvent, DecodedEvent};

pub struct BuildAgentDecoder;

/// Wire 2: BuildAgent_1_0 — 3 nullable strings + 1 list of strings.
impl BodyDecoder for BuildAgentDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let username = if kryo::is_field_present(flags as u16, 0) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let local_hostname = if kryo::is_field_present(flags as u16, 1) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let public_hostname = if kryo::is_field_present(flags as u16, 2) {
            Some(table.read_string(body, &mut pos)?)
        } else {
            None
        };

        let ip_addresses = if kryo::is_field_present(flags as u16, 3) {
            kryo::read_list_of_interned_strings(body, &mut pos, &mut table)?
        } else {
            vec![]
        };

        Ok(DecodedEvent::BuildAgent(BuildAgentEvent {
            username,
            local_hostname,
            public_hostname,
            ip_addresses,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 4 bits present
        // username: "user1" → zigzag(5)=10
        data.push(0x0A);
        data.extend_from_slice(b"user1");
        // local_hostname: "host" → zigzag(4)=8
        data.push(0x08);
        data.extend_from_slice(b"host");
        // public_hostname: "pub" → zigzag(3)=6
        data.push(0x06);
        data.extend_from_slice(b"pub");
        // ip_addresses: ["1.2.3.4"]
        data.push(0x01); // 1 element
        data.push(0x0E); // zigzag(7)=14
        data.extend_from_slice(b"1.2.3.4");

        let decoder = BuildAgentDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildAgent(e) = result {
            assert_eq!(e.username, Some("user1".into()));
            assert_eq!(e.local_hostname, Some("host".into()));
            assert_eq!(e.public_hostname, Some("pub".into()));
            assert_eq!(e.ip_addresses, vec!["1.2.3.4"]);
        } else {
            panic!("expected BuildAgent");
        }
    }
}
