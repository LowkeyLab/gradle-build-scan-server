use error::ParseError;

use super::{BasicMemoryStatsEvent, BodyDecoder, DecodedEvent, MemoryPoolSnapshotEvent};

pub struct BasicMemoryStatsDecoder;

/// Wire 257: BasicMemoryStats_1_1 — 3 longs + list of MemoryPoolSnapshot + gcTime.
impl BodyDecoder for BasicMemoryStatsDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let free = if kryo::is_field_present(flags as u16, 0) {
            Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
                body, &mut pos,
            )?))
        } else {
            None
        };

        let total = if kryo::is_field_present(flags as u16, 1) {
            Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
                body, &mut pos,
            )?))
        } else {
            None
        };

        let max = if kryo::is_field_present(flags as u16, 2) {
            Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
                body, &mut pos,
            )?))
        } else {
            None
        };

        let peak_snapshots = if kryo::is_field_present(flags as u16, 3) {
            let count = kryo::read_positive_varint_i32(body, &mut pos)? as usize;
            let mut snapshots = Vec::with_capacity(count);
            for _ in 0..count {
                snapshots.push(decode_memory_pool_snapshot(body, &mut pos, &mut table)?);
            }
            snapshots
        } else {
            vec![]
        };

        let gc_time = if kryo::is_field_present(flags as u16, 4) {
            Some(models::Duration::new(kryo::read_kryo_long_unsigned(
                body, &mut pos,
            )?))
        } else {
            None
        };

        Ok(DecodedEvent::BasicMemoryStats(BasicMemoryStatsEvent {
            free,
            total,
            max,
            peak_snapshots,
            gc_time,
        }))
    }
}

fn decode_memory_pool_snapshot(
    body: &[u8],
    pos: &mut usize,
    table: &mut kryo::StringInternTable,
) -> Result<MemoryPoolSnapshotEvent, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;

    let name = if kryo::is_field_present(flags as u16, 0) {
        Some(table.read_string(body, pos)?)
    } else {
        None
    };

    // bit 1: heap boolean — value IS the bit (is_field_present=true means heap=true)
    let heap = kryo::is_field_present(flags as u16, 1);

    let init = if kryo::is_field_present(flags as u16, 2) {
        Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
            body, pos,
        )?))
    } else {
        None
    };

    let used = if kryo::is_field_present(flags as u16, 3) {
        Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
            body, pos,
        )?))
    } else {
        None
    };

    let committed = if kryo::is_field_present(flags as u16, 4) {
        Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
            body, pos,
        )?))
    } else {
        None
    };

    let max = if kryo::is_field_present(flags as u16, 5) {
        Some(models::MemoryBytes::new(kryo::read_kryo_long_unsigned(
            body, pos,
        )?))
    } else {
        None
    };

    Ok(MemoryPoolSnapshotEvent {
        name,
        heap,
        init,
        used,
        committed,
        max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_all_absent() {
        // flags = 0b00011111 → all 5 bits set → all fields absent
        let data = [0x1F];
        let decoder = BasicMemoryStatsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BasicMemoryStats(e) = result {
            assert_eq!(e.free, None);
            assert_eq!(e.total, None);
            assert_eq!(e.max, None);
            assert!(e.peak_snapshots.is_empty());
            assert_eq!(e.gc_time, None);
        } else {
            panic!("expected BasicMemoryStats");
        }
    }

    #[test]
    fn test_decode_free_total_max_present() {
        // bits 3 and 4 absent (set), bits 0,1,2 present (clear) → 0b00011000 = 0x18
        let mut data = vec![0x18u8];
        // Kryo long unsigned: value is written directly as unsigned varint (no zigzag)
        // free = 100 → unsigned varint 100 = 0x64
        data.push(0x64);
        // total = 512 → unsigned varint 512 = 0x80 0x04
        data.push(0x80);
        data.push(0x04);
        // max = 1024 → unsigned varint 1024 = 0x80 0x08
        data.push(0x80);
        data.push(0x08);

        let decoder = BasicMemoryStatsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BasicMemoryStats(e) = result {
            assert_eq!(e.free, Some(models::MemoryBytes::new(100)));
            assert_eq!(e.total, Some(models::MemoryBytes::new(512)));
            assert_eq!(e.max, Some(models::MemoryBytes::new(1024)));
            assert!(e.peak_snapshots.is_empty());
            assert_eq!(e.gc_time, None);
        } else {
            panic!("expected BasicMemoryStats");
        }
    }

    #[test]
    fn test_decode_with_snapshots() {
        // flags: bit 3 present (peak_snapshots), all others absent → 0b00010111 = 0x17
        let mut data = vec![0x17u8];
        // count = 1
        data.push(0x01);
        // snapshot flags: all 6 bits set → all absent (heap = false)
        // bits 0..5 all set = 0b00111111 = 0x3F
        data.push(0x3F);

        let decoder = BasicMemoryStatsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BasicMemoryStats(e) = result {
            assert_eq!(e.peak_snapshots.len(), 1);
            let snap = &e.peak_snapshots[0];
            assert_eq!(snap.name, None);
            assert!(!snap.heap); // bit 1 set → absent → heap = false
            assert_eq!(snap.init, None);
            assert_eq!(snap.used, None);
            assert_eq!(snap.committed, None);
            assert_eq!(snap.max, None);
        } else {
            panic!("expected BasicMemoryStats");
        }
    }

    #[test]
    fn test_decode_all_fields_present() {
        // flags = 0b00000000 → all 5 bits clear → all fields present
        let mut data = vec![0x00u8];
        // Kryo long unsigned encoding (no zigzag)
        // free = 100 → unsigned varint 100 = 0x64
        data.push(0x64);
        // total = 512 → unsigned varint 512 = 0x80 0x04
        data.push(0x80);
        data.push(0x04);
        // max = 1024 → unsigned varint 1024 = 0x80 0x08
        data.push(0x80);
        data.push(0x08);
        // peak_snapshots: count = 2
        data.push(0x02);

        // Snapshot 1: all fields present (flags = 0x00), heap = true (bit 1 clear)
        data.push(0x00);
        // name = "Eden Space" (10 chars) → zigzag(10) = 20 = 0x14
        data.push(0x14);
        for &ch in b"Eden Space" {
            data.push(ch);
        }
        // heap = true (bit 1 clear, no payload)
        // init = 50 → unsigned varint 50 = 0x32
        data.push(0x32);
        // used = 200 → unsigned varint 200 = 0xC8 0x01
        data.push(0xC8);
        data.push(0x01);
        // committed = 256 → unsigned varint 256 = 0x80 0x02
        data.push(0x80);
        data.push(0x02);
        // max = 512 → unsigned varint 512 = 0x80 0x04
        data.push(0x80);
        data.push(0x04);

        // Snapshot 2: name back-refs to "Eden Space", heap = false (bit 1 set)
        // flags: bit 1 set (heap=false), rest clear → 0b00000010 = 0x02
        data.push(0x02);
        // name = back-ref to index 0 → zigzag(-1) = 1
        data.push(0x01);
        // init = 10 → unsigned varint 10 = 0x0A
        data.push(0x0A);
        // used = 30 → unsigned varint 30 = 0x1E
        data.push(0x1E);
        // committed = 64 → unsigned varint 64 = 0x40
        data.push(0x40);
        // max = 128 → unsigned varint 128 = 0x80 0x01
        data.push(0x80);
        data.push(0x01);

        // gc_time = 42 → unsigned varint 42 = 0x2A
        data.push(0x2A);

        let decoder = BasicMemoryStatsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BasicMemoryStats(e) = result {
            assert_eq!(e.free, Some(models::MemoryBytes::new(100)));
            assert_eq!(e.total, Some(models::MemoryBytes::new(512)));
            assert_eq!(e.max, Some(models::MemoryBytes::new(1024)));
            assert_eq!(e.gc_time, Some(models::Duration::new(42)));
            assert_eq!(e.peak_snapshots.len(), 2);

            let s1 = &e.peak_snapshots[0];
            assert_eq!(s1.name, Some("Eden Space".to_string()));
            assert!(s1.heap);
            assert_eq!(s1.init, Some(models::MemoryBytes::new(50)));
            assert_eq!(s1.used, Some(models::MemoryBytes::new(200)));
            assert_eq!(s1.committed, Some(models::MemoryBytes::new(256)));
            assert_eq!(s1.max, Some(models::MemoryBytes::new(512)));

            let s2 = &e.peak_snapshots[1];
            assert_eq!(s2.name, Some("Eden Space".to_string())); // back-ref
            assert!(!s2.heap);
            assert_eq!(s2.init, Some(models::MemoryBytes::new(10)));
            assert_eq!(s2.used, Some(models::MemoryBytes::new(30)));
            assert_eq!(s2.committed, Some(models::MemoryBytes::new(64)));
            assert_eq!(s2.max, Some(models::MemoryBytes::new(128)));
        } else {
            panic!("expected BasicMemoryStats");
        }
    }

    #[test]
    fn test_snapshot_heap_true() {
        // flags: bit 3 present (peak_snapshots), all others absent → 0x17
        let mut data = vec![0x17u8];
        // count = 1
        data.push(0x01);
        // snapshot flags: all bits set except bit 1 (heap) → heap = true, rest absent
        // bits 0,2,3,4,5 set, bit 1 clear → 0b00111101 = 0x3D
        data.push(0x3D);

        let decoder = BasicMemoryStatsDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::BasicMemoryStats(e) = result {
            assert_eq!(e.peak_snapshots.len(), 1);
            let snap = &e.peak_snapshots[0];
            assert!(snap.heap); // bit 1 clear → present → heap = true
        } else {
            panic!("expected BasicMemoryStats");
        }
    }
}
