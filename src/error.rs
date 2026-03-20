use thiserror::Error;

#[derive(Error, Debug)]
pub enum ItlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid magic: expected {expected:?}, got {got:?}")]
    InvalidMagic { expected: &'static [u8], got: Vec<u8> },

    #[error("decompression failed: {0}")]
    Decompression(String),

    #[error("compression failed: {0}")]
    Compression(String),

    #[error("unexpected end of data at offset {0:#x}")]
    UnexpectedEof(usize),

    #[error("parse error at offset {offset:#x}: {message}")]
    Parse { offset: usize, message: String },

    #[error("unknown section signature at offset {offset:#x}: {sig:?}")]
    UnknownSection { offset: usize, sig: [u8; 4] },

    #[error("invalid string encoding type {0}")]
    InvalidStringEncoding(u32),
}

pub type Result<T> = std::result::Result<T, ItlError>;
