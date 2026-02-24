use error::ParseError;
use events::DecodedEvent;
use framing::FramedEvent;
use models::BuildScanPayload;

pub fn parse(raw_bytes: &[u8]) -> Result<BuildScanPayload, ParseError> {
    let header = outer_header::OuterHeader::parse(raw_bytes)?;
    let decompressed = decompress::Decompressor::decompress(&raw_bytes[header.gzip_offset..])?;
    let registry = events::DecoderRegistry::new();

    let decoded_events: Result<Vec<(FramedEvent, DecodedEvent)>, ParseError> =
        framing::EventFrameReader::new(&decompressed)
            .map(|frame_result| {
                let frame = frame_result?;
                let decoded = registry.decode(frame.wire_id, &frame.body)?;
                Ok((frame, decoded))
            })
            .collect();

    Ok(assembly::assemble(decoded_events?))
}
