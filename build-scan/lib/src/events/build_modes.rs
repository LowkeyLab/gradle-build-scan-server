use error::ParseError;

use super::{BodyDecoder, BuildModesEvent, DecodedEvent};

pub struct BuildModesDecoder;

/// Wire 516: BuildModes_1_2 — 9 booleans packed into flags bits + 1 int.
/// Bits 0-8: boolean values (bit=0 means true, bit=1 means false).
/// Bit 9: maxWorkers presence; if present, read a positive varint i32.
impl BodyDecoder for BuildModesDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_u16_be(body, &mut pos)?;

        let max_workers = if kryo::is_field_present(flags, 9) {
            Some(kryo::read_positive_varint_i32(body, &mut pos)?)
        } else {
            None
        };

        Ok(DecodedEvent::BuildModes(BuildModesEvent {
            refresh_dependencies: kryo::is_field_present(flags, 0),
            parallel_project_execution: kryo::is_field_present(flags, 1),
            rerun_tasks: kryo::is_field_present(flags, 2),
            continuous: kryo::is_field_present(flags, 3),
            continue_on_failure: kryo::is_field_present(flags, 4),
            configure_on_demand: kryo::is_field_present(flags, 5),
            daemon: kryo::is_field_present(flags, 6),
            offline: kryo::is_field_present(flags, 7),
            dry_run: kryo::is_field_present(flags, 8),
            max_workers,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_daemon_only_with_workers() {
        // bit 6 (daemon) = 0 (present/true), bit 9 (max_workers) = 0 (present)
        // All other bits = 1 (absent/false)
        // flags = 0b0000_0001_1011_1111 = 0x01BF
        let mut data = vec![0x01, 0xBF];
        data.push(0x04); // max_workers = 4

        let decoder = BuildModesDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildModes(e) = result {
            assert!(!e.refresh_dependencies);
            assert!(!e.parallel_project_execution);
            assert!(e.daemon);
            assert!(!e.offline);
            assert_eq!(e.max_workers, Some(4));
        } else {
            panic!("expected BuildModes");
        }
    }

    #[test]
    fn test_decode_all_false_no_workers() {
        // All bits set → all false/absent
        // flags = 0x03FF (10 bits all 1)
        let data = [0x03, 0xFF];
        let decoder = BuildModesDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BuildModes(e) = result {
            assert!(!e.refresh_dependencies);
            assert!(!e.daemon);
            assert_eq!(e.max_workers, None);
        } else {
            panic!("expected BuildModes");
        }
    }
}
