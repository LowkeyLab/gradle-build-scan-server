use error::ParseError;
use models::{FailureId, TaskId};

use super::{BodyDecoder, DecodedEvent, TestResultEvent};

/// Decoder for wire 284 (TestFinished_1_1) events.
///
/// The body encodes a flags byte followed by conditionally-present fixed 8-byte LE fields.
/// The test outcome (failed/skipped) is encoded directly in the flags bits, NOT as separate
/// data fields.
///
/// ```text
/// [0]     flags byte (inverted presence: bit=0 means condition is true)
///           bit 0: task != 0  → encode task as fixed 8-byte LE i64
///           bit 1: id != 0    → encode id as fixed 8-byte LE i64
///           bit 2: failed     (0 = failed, 1 = not failed)
///           bit 3: skipped    (0 = skipped, 1 = not skipped)
///           bit 4: failureId != null → encode failureId as fixed 8-byte LE i64
/// [1..]   task     (8 bytes LE, if bit 0 clear)
/// [..]    id       (8 bytes LE, if bit 1 clear)
/// [..]    failureId (8 bytes LE, if bit 4 clear)
/// ```
pub struct TestResultDecoder;

impl BodyDecoder for TestResultDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;

        let task = if kryo::is_field_present(flags as u16, 0) {
            TaskId::new(kryo::read_task_id(body, &mut pos)?)
        } else {
            TaskId::new(0)
        };
        let id = if kryo::is_field_present(flags as u16, 1) {
            kryo::read_task_id(body, &mut pos)?
        } else {
            0
        };
        // Bits 2 and 3 encode failed/skipped as inverted booleans in the flags.
        // bit=0 means the condition IS true (inverted presence logic).
        let failed = kryo::is_field_present(flags as u16, 2);
        let skipped = kryo::is_field_present(flags as u16, 3);

        // Bit 4 indicates the presence of a failureId field in the wire format.
        let failure_id = if kryo::is_field_present(flags as u16, 4) {
            Some(FailureId::new(kryo::read_task_id(body, &mut pos)?))
        } else {
            None
        };

        Ok(DecodedEvent::TestResult(TestResultEvent {
            task,
            id,
            failed,
            skipped,
            failure_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_passing_test() {
        // flags = 0x1c = 0b00011100
        // bit 0=0 (task present), bit 1=0 (id present),
        // bit 2=1 (not failed), bit 3=1 (not skipped), bit 4=1 (no failure)
        let mut data = vec![0x1c];
        data.extend_from_slice(&42i64.to_le_bytes()); // task
        data.extend_from_slice(&100i64.to_le_bytes()); // id

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.task, TaskId::new(42));
            assert_eq!(e.id, 100);
            assert!(!e.failed);
            assert!(!e.skipped);
            assert!(e.failure_id.is_none());
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_failed_test() {
        // flags = 0x18 = 0b00011000
        // bit 0=0 (task present), bit 1=0 (id present),
        // bit 2=0 (FAILED), bit 3=1 (not skipped), bit 4=1 (no failure)
        let mut data = vec![0x18];
        data.extend_from_slice(&7i64.to_le_bytes()); // task
        data.extend_from_slice(&55i64.to_le_bytes()); // id

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.task, TaskId::new(7));
            assert_eq!(e.id, 55);
            assert!(e.failed);
            assert!(!e.skipped);
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_skipped_test() {
        // flags = 0x14 = 0b00010100
        // bit 0=0 (task present), bit 1=0 (id present),
        // bit 2=1 (not failed), bit 3=0 (SKIPPED), bit 4=1 (no failure)
        let mut data = vec![0x14];
        data.extend_from_slice(&3i64.to_le_bytes()); // task
        data.extend_from_slice(&99i64.to_le_bytes()); // id

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert_eq!(e.task, TaskId::new(3));
            assert_eq!(e.id, 99);
            assert!(!e.failed);
            assert!(e.skipped);
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_with_failure_id() {
        // flags = 0x08 = 0b00001000
        // bit 0=0 (task present), bit 1=0 (id present),
        // bit 2=0 (FAILED), bit 3=1 (not skipped), bit 4=0 (failureId present)
        let mut data = vec![0x08];
        data.extend_from_slice(&7i64.to_le_bytes()); // task
        data.extend_from_slice(&55i64.to_le_bytes()); // id
        data.extend_from_slice(&999i64.to_le_bytes()); // failureId

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert!(e.failed);
            assert!(!e.skipped);
            assert_eq!(e.failure_id, Some(FailureId::new(999)));
        } else {
            panic!("expected TestResult");
        }
    }

    #[test]
    fn test_decode_real_payload_bytes() {
        // Real bytes from captured build scan: a passing test
        let data = vec![
            0x1c, 0x64, 0x30, 0x61, 0x7b, 0xbb, 0x68, 0x62, 0x10, 0x41, 0x2c, 0x5f, 0xe9, 0xc6,
            0xeb, 0xc4, 0x6a,
        ];

        let decoder = TestResultDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::TestResult(e) = result {
            assert!(!e.failed, "test should not be failed");
            assert!(!e.skipped, "test should not be skipped");
            // task = i64 from bytes [64,30,61,7b,bb,68,62,10]
            assert_eq!(
                e.task,
                TaskId::new(i64::from_le_bytes([
                    0x64, 0x30, 0x61, 0x7b, 0xbb, 0x68, 0x62, 0x10
                ]))
            );
        } else {
            panic!("expected TestResult");
        }
    }
}
