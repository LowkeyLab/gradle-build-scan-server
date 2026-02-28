use error::ParseError;

use super::{
    BodyDecoder, DecodedEvent, IndexedNormalizedSamplesEvent, NormalizedSamplesEvent, ProcessEvent,
    ResourceUsageEvent,
};

pub struct ResourceUsageDecoder;

/// Wire 407: ResourceUsage_2_0 — complex event with 4 conditional + 12 unconditional sub-fields.
impl BodyDecoder for ResourceUsageDecoder {
    fn decode(&self, body: &[u8]) -> Result<DecodedEvent, ParseError> {
        let mut pos = 0;
        let flags = kryo::read_flags_byte(body, &mut pos)?;
        let mut table = kryo::StringInternTable::new();

        let timestamps = if kryo::is_field_present(flags as u16, 0) {
            kryo::read_list_of_byte_arrays(body, &mut pos)?
        } else {
            vec![]
        };

        let build_process_cpu = decode_normalized_samples(body, &mut pos)?;
        let build_child_processes_cpu = decode_normalized_samples(body, &mut pos)?;
        let all_processes_cpu_sum = decode_normalized_samples(body, &mut pos)?;

        let all_processes_cpu = if kryo::is_field_present(flags as u16, 1) {
            Some(kryo::read_byte_array(body, &mut pos)?)
        } else {
            None
        };

        let build_process_memory = decode_normalized_samples(body, &mut pos)?;
        let build_child_processes_memory = decode_normalized_samples(body, &mut pos)?;
        let all_processes_memory = decode_normalized_samples(body, &mut pos)?;

        let total_system_memory = if kryo::is_field_present(flags as u16, 2) {
            Some(kryo::read_zigzag_i64(body, &mut pos)?)
        } else {
            None
        };

        let disk_read_speed = decode_normalized_samples(body, &mut pos)?;
        let disk_write_speed = decode_normalized_samples(body, &mut pos)?;
        let network_download_speed = decode_normalized_samples(body, &mut pos)?;
        let network_upload_speed = decode_normalized_samples(body, &mut pos)?;

        let processes = if kryo::is_field_present(flags as u16, 3) {
            let count = kryo::read_positive_varint_i32(body, &mut pos)? as usize;
            let mut procs = Vec::with_capacity(count);
            for _ in 0..count {
                procs.push(decode_process(body, &mut pos, &mut table)?);
            }
            procs
        } else {
            vec![]
        };

        let top_processes_by_cpu = decode_indexed_normalized_samples(body, &mut pos)?;
        let top_processes_by_memory = decode_indexed_normalized_samples(body, &mut pos)?;

        Ok(DecodedEvent::ResourceUsage(ResourceUsageEvent {
            timestamps,
            build_process_cpu,
            build_child_processes_cpu,
            all_processes_cpu_sum,
            all_processes_cpu,
            build_process_memory,
            build_child_processes_memory,
            all_processes_memory,
            total_system_memory,
            disk_read_speed,
            disk_write_speed,
            network_download_speed,
            network_upload_speed,
            processes,
            top_processes_by_cpu,
            top_processes_by_memory,
        }))
    }
}

fn decode_normalized_samples(
    body: &[u8],
    pos: &mut usize,
) -> Result<NormalizedSamplesEvent, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;

    let samples = if kryo::is_field_present(flags as u16, 0) {
        Some(kryo::read_byte_array(body, pos)?)
    } else {
        None
    };

    let max = if kryo::is_field_present(flags as u16, 1) {
        Some(kryo::read_zigzag_i64(body, pos)?)
    } else {
        None
    };

    Ok(NormalizedSamplesEvent { samples, max })
}

fn decode_indexed_normalized_samples(
    body: &[u8],
    pos: &mut usize,
) -> Result<IndexedNormalizedSamplesEvent, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;

    let indices = if kryo::is_field_present(flags as u16, 0) {
        kryo::read_list_of_list_of_i32(body, pos)?
    } else {
        vec![]
    };

    let samples = if kryo::is_field_present(flags as u16, 1) {
        kryo::read_list_of_byte_arrays(body, pos)?
    } else {
        vec![]
    };

    let max = if kryo::is_field_present(flags as u16, 2) {
        Some(kryo::read_zigzag_i64(body, pos)?)
    } else {
        None
    };

    Ok(IndexedNormalizedSamplesEvent {
        indices,
        samples,
        max,
    })
}

fn decode_process(
    body: &[u8],
    pos: &mut usize,
    table: &mut kryo::StringInternTable,
) -> Result<ProcessEvent, ParseError> {
    let flags = kryo::read_flags_byte(body, pos)?;

    let id = if kryo::is_field_present(flags as u16, 0) {
        Some(kryo::read_zigzag_i64(body, pos)?)
    } else {
        None
    };

    let name = if kryo::is_field_present(flags as u16, 1) {
        Some(table.read_string(body, pos)?)
    } else {
        None
    };

    let display_name = if kryo::is_field_present(flags as u16, 2) {
        Some(table.read_string(body, pos)?)
    } else {
        None
    };

    let process_type = if kryo::is_field_present(flags as u16, 3) {
        Some(kryo::read_enum_ordinal(body, pos)?)
    } else {
        None
    };

    Ok(ProcessEvent {
        id,
        name,
        display_name,
        process_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_normalized_samples_all_absent() -> Vec<u8> {
        // flags = 0b11 → both bits set → samples absent, max absent
        vec![0x03]
    }

    fn make_indexed_normalized_samples_all_absent() -> Vec<u8> {
        // flags = 0b111 → all 3 bits set → indices, samples, max all absent
        vec![0x07]
    }

    #[test]
    fn test_decode_all_fields_absent() {
        // Main flags: bits 0-3 all set → all 4 conditional fields absent
        // bit0=timestamps absent, bit1=allProcessesCpu absent, bit2=totalSystemMemory absent, bit3=processes absent
        let mut data = vec![0x0Fu8]; // 0b00001111

        // 12 unconditional NormalizedSamples/IndexedNormalizedSamples, all absent:
        // 8 NormalizedSamples (buildProcessCpu, buildChildProcessesCpu, allProcessesCpuSum,
        //   buildProcessMemory, buildChildProcessesMemory, allProcessesMemory,
        //   diskReadSpeed, diskWriteSpeed) + 2 more (networkDownloadSpeed, networkUploadSpeed)
        // = 10 NormalizedSamples total
        for _ in 0..10 {
            data.extend_from_slice(&make_normalized_samples_all_absent());
        }
        // 2 IndexedNormalizedSamples (topProcessesByCpu, topProcessesByMemory)
        for _ in 0..2 {
            data.extend_from_slice(&make_indexed_normalized_samples_all_absent());
        }

        let decoder = ResourceUsageDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::ResourceUsage(e) = result {
            assert!(e.timestamps.is_empty());
            assert!(e.build_process_cpu.samples.is_none());
            assert!(e.build_process_cpu.max.is_none());
            assert!(e.all_processes_cpu.is_none());
            assert!(e.total_system_memory.is_none());
            assert!(e.processes.is_empty());
            assert!(
                e.top_processes_by_cpu.indices.is_empty(),
                "indices should be empty when absent"
            );
            assert!(e.top_processes_by_memory.max.is_none());
        } else {
            panic!("expected ResourceUsage");
        }
    }

    #[test]
    fn test_decode_all_conditional_fields_present() {
        // Main flags: 0b0000 → all 4 conditional fields present
        let mut data = vec![0x00u8];

        // bit 0: timestamps = list of 1 byte array [0x01, 0x02]
        data.push(0x01); // list len=1
        data.push(0x02); // byte array len=2
        data.push(0x01);
        data.push(0x02);

        // 3 unconditional NormalizedSamples (all absent)
        for _ in 0..3 {
            data.push(0x03); // flags 0b11 → both absent
        }

        // bit 1: all_processes_cpu = byte array [0xAA]
        data.push(0x01); // byte array len=1
        data.push(0xAA);

        // 3 more unconditional NormalizedSamples (all absent)
        for _ in 0..3 {
            data.push(0x03);
        }

        // bit 2: total_system_memory = 8192 → zigzag(8192) = 16384 = 0x80 0x80 0x01
        data.push(0x80);
        data.push(0x80);
        data.push(0x01);

        // 4 more unconditional NormalizedSamples (all absent)
        for _ in 0..4 {
            data.push(0x03);
        }

        // bit 3: processes = list of 1 process
        data.push(0x01); // count=1
        // Process flags: only name present (bit 1 clear, bits 0,2,3 set) → 0b00001101 = 0x0D
        data.push(0x0D);
        // name = "java" (4 chars) → zigzag(4) = 8 = 0x08
        data.push(0x08);
        for &ch in b"java" {
            data.push(ch);
        }

        // 2 unconditional IndexedNormalizedSamples (all absent)
        for _ in 0..2 {
            data.push(0x07); // flags 0b111 → all 3 absent
        }

        let decoder = ResourceUsageDecoder;
        let result = decoder.decode(&data).unwrap();
        if let DecodedEvent::ResourceUsage(e) = result {
            assert_eq!(e.timestamps.len(), 1);
            assert_eq!(e.timestamps[0], vec![0x01, 0x02]);
            assert_eq!(e.all_processes_cpu, Some(vec![0xAA]));
            assert_eq!(e.total_system_memory, Some(8192));
            assert_eq!(e.processes.len(), 1);
            assert_eq!(e.processes[0].name, Some("java".to_string()));
            assert_eq!(e.processes[0].id, None);
            assert_eq!(e.processes[0].display_name, None);
            assert_eq!(e.processes[0].process_type, None);
        } else {
            panic!("expected ResourceUsage");
        }
    }

    #[test]
    fn test_decode_normalized_samples_both_present() {
        // flags = 0b00 → both present
        let mut data = vec![0x00u8];
        // samples = [0xAA, 0xBB] → length 2, then bytes
        data.push(0x02);
        data.push(0xAA);
        data.push(0xBB);
        // max = 42 → zigzag(42) = 84 = 0x54
        data.push(0x54);

        let mut pos = 0;
        let result = decode_normalized_samples(&data, &mut pos).unwrap();
        assert_eq!(result.samples, Some(vec![0xAA, 0xBB]));
        assert_eq!(result.max, Some(42));
        assert_eq!(pos, 5); // flags(1) + len_prefix(1) + 2 bytes + zigzag_max(1) = 5
    }

    #[test]
    fn test_decode_indexed_normalized_samples_all_present() {
        // flags = 0b000 → all present
        let mut data = vec![0x00u8];
        // indices: outer count=1, inner list: count=2, values 10, 20
        data.push(0x01); // outer len=1
        data.push(0x02); // inner len=2
        data.push(0x0A); // 10
        data.push(0x14); // 20
        // samples: outer count=1, inner byte array: len=2, bytes 0xAA, 0xBB
        data.push(0x01); // outer len=1
        data.push(0x02); // byte array len=2
        data.push(0xAA);
        data.push(0xBB);
        // max = 99 → zigzag(99) = 198 = 0xC6 0x01
        data.push(0xC6);
        data.push(0x01);

        let mut pos = 0;
        let result = decode_indexed_normalized_samples(&data, &mut pos).unwrap();
        assert_eq!(result.indices, vec![vec![10, 20]]);
        assert_eq!(result.samples, vec![vec![0xAA, 0xBB]]);
        assert_eq!(result.max, Some(99));
    }

    #[test]
    fn test_decode_process_all_present() {
        // flags = 0b0000 → all 4 fields present
        let mut data = vec![0x00u8];
        // id = 5 → zigzag(5) = 10 = 0x0A
        data.push(0x0A);
        // name = "foo" → zigzag(3)=6, then f=0x66, o=0x6f, o=0x6f
        data.push(0x06);
        data.push(0x66);
        data.push(0x6f);
        data.push(0x6f);
        // display_name = back-ref to "foo" → zigzag(-1)=1
        data.push(0x01);
        // process_type = ordinal 2 → unsigned varint 2
        data.push(0x02);

        let mut pos = 0;
        let mut table = kryo::StringInternTable::new();
        let result = decode_process(&data, &mut pos, &mut table).unwrap();
        assert_eq!(result.id, Some(5));
        assert_eq!(result.name, Some("foo".to_string()));
        assert_eq!(result.display_name, Some("foo".to_string()));
        assert_eq!(result.process_type, Some(2));
    }

    #[test]
    fn test_decode_process_all_absent() {
        // flags = 0b1111 → all 4 fields absent
        let data = vec![0x0Fu8];
        let mut pos = 0;
        let mut table = kryo::StringInternTable::new();
        let result = decode_process(&data, &mut pos, &mut table).unwrap();
        assert_eq!(result.id, None);
        assert_eq!(result.name, None);
        assert_eq!(result.display_name, None);
        assert_eq!(result.process_type, None);
    }
}
