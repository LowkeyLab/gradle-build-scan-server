use error::ParseError;

use super::{BodyDecoder, DecodedEvent, DaemonStateEvent};

pub struct DaemonStateDecoder;

/// Wire 265: DaemonState_1_1 — 2 longs, 2 ints, 1 nullable boolean.
impl BodyDecoder for DaemonStateDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let start_time = if kryo::is_field_present(flags as u16, 0) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        let build_number = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_positive_varint_i32(body, &mut pos)?)
        } else {
            None
        };

        let number_of_running_daemons = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_positive_varint_i32(body, &mut pos)?)
        } else {
            None
        };

        let idle_timeout = if kryo::is_field_present(flags as u16, 3) {
            Some(kryo::read_positive_varint_i64(body, &mut pos)?)
        } else {
            None
        };

        // bit 4: singleUse — nullable Boolean, bit IS the value (no payload)
        let single_use = if kryo::is_field_present(flags as u16, 4) {
            Some(true)
        } else {
            None
        };

        Ok(DecodedEvent::DaemonState(DaemonStateEvent {
            start_time,
            build_number,
            number_of_running_daemons,
            idle_timeout,
            single_use,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_present() {
        let mut data = vec![0x00]; // all 5 bits present
        data.push(0x64); // start_time = 100 (unsigned varint)
        data.push(0x03); // build_number = 3
        data.push(0x01); // running_daemons = 1
        data.push(0x80); // idle_timeout = 10800000
        data.push(0x97);
        data.push(0x93);
        data.push(0x05);
        // single_use: bit IS the value, no payload

        let decoder = DaemonStateDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::DaemonState(e) = result {
            assert_eq!(e.start_time, Some(100));
            assert_eq!(e.build_number, Some(3));
            assert_eq!(e.number_of_running_daemons, Some(1));
            assert_eq!(e.idle_timeout, Some(10800000));
            assert_eq!(e.single_use, Some(true));
        } else {
            panic!("expected DaemonState");
        }
    }

    #[test]
    fn test_decode_all_absent() {
        let data = [0x1F]; // all 5 bits set → absent
        let decoder = DaemonStateDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::DaemonState(e) = result {
            assert_eq!(e.start_time, None);
            assert_eq!(e.build_number, None);
            assert_eq!(e.number_of_running_daemons, None);
            assert_eq!(e.idle_timeout, None);
            assert_eq!(e.single_use, None);
        } else {
            panic!("expected DaemonState");
        }
    }
}
