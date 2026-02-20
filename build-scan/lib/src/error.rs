use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to decompress Gzip stream")]
    InvalidGzip,
    #[error("Malformed LEB128 varint encountered at offset {offset}")]
    MalformedLeb128 { offset: usize },
    #[error("Unexpected primitive type: expected {expected}")]
    UnexpectedPrimitive { expected: &'static str },
    #[error("Unknown or unhandled event schema ID: {0}")]
    UnknownEventSchema(u32),
    #[error("Unknown Event ID encountered: {id}")]
    UnknownEvent { id: u64 },
    #[error("Invalid string reference: index {0} not found in dictionary")]
    InvalidStringRef(u32),
    #[error("Unexpected End Of File")]
    UnexpectedEof,
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}
