use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to decompress Gzip stream")]
    InvalidGzip,
    #[error("Malformed LEB128 varint at offset {offset}")]
    MalformedLeb128 { offset: usize },
    #[error("Unexpected end of data at offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,
    #[error("Invalid outer header: {reason}")]
    InvalidHeader { reason: &'static str },
    #[error("Invalid string intern reference: index {index}")]
    InvalidStringRef { index: usize },
}
