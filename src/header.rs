use crate::error::{ItlError, Result};

pub const ENVELOPE_MAGIC: &[u8; 4] = b"hdfm";
pub const ENVELOPE_LENGTH: usize = 0x90; // 144 bytes

/// The outer envelope header of an ITL file (big-endian).
#[derive(Debug, Clone)]
pub struct EnvelopeHeader {
    /// Raw 144 bytes for round-trip fidelity.
    pub(crate) raw: [u8; ENVELOPE_LENGTH],
}

impl EnvelopeHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < ENVELOPE_LENGTH {
            return Err(ItlError::UnexpectedEof(data.len()));
        }

        let magic = &data[0..4];
        if magic != ENVELOPE_MAGIC {
            return Err(ItlError::InvalidMagic {
                expected: ENVELOPE_MAGIC,
                got: magic.to_vec(),
            });
        }

        let mut raw = [0u8; ENVELOPE_LENGTH];
        raw.copy_from_slice(&data[..ENVELOPE_LENGTH]);
        Ok(Self { raw })
    }

    #[allow(dead_code)]
    pub fn envelope_length(&self) -> u32 {
        u32::from_be_bytes(self.raw[4..8].try_into().unwrap())
    }

    #[allow(dead_code)]
    pub fn file_length(&self) -> u32 {
        u32::from_be_bytes(self.raw[8..12].try_into().unwrap())
    }

    pub fn set_file_length(&mut self, len: u32) {
        self.raw[8..12].copy_from_slice(&len.to_be_bytes());
    }

    pub fn version(&self) -> &str {
        let vlen = self.raw[16] as usize;
        let end = 17 + vlen;
        std::str::from_utf8(&self.raw[17..end]).unwrap_or("unknown")
    }

    pub fn msdh_count(&self) -> u32 {
        u32::from_be_bytes(self.raw[48..52].try_into().unwrap())
    }

    pub fn library_persistent_id(&self) -> u64 {
        u64::from_be_bytes(self.raw[52..60].try_into().unwrap())
    }

    pub fn max_crypt_size(&self) -> u32 {
        u32::from_be_bytes(self.raw[92..96].try_into().unwrap())
    }

    pub fn tz_offset_seconds(&self) -> i32 {
        i32::from_be_bytes(self.raw[100..104].try_into().unwrap())
    }

    pub fn library_date_raw(&self) -> u32 {
        u32::from_be_bytes(self.raw[112..116].try_into().unwrap())
    }

    pub fn as_bytes(&self) -> &[u8; ENVELOPE_LENGTH] {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ItlError;

    fn valid_header_bytes() -> [u8; ENVELOPE_LENGTH] {
        let mut buf = [0u8; ENVELOPE_LENGTH];
        buf[0..4].copy_from_slice(b"hdfm");
        buf[4..8].copy_from_slice(&144u32.to_be_bytes());
        buf[8..12].copy_from_slice(&0x00280000u32.to_be_bytes());
        buf[16] = 9;
        buf[17..26].copy_from_slice(b"12.13.9.1");
        buf[48..52].copy_from_slice(&0x0000000Fu32.to_be_bytes());
        buf[52..60].copy_from_slice(&0x1234567890ABCDEFu64.to_be_bytes());
        buf[92..96].copy_from_slice(&0x00019000u32.to_be_bytes());
        buf[100..104].copy_from_slice(&(-18000i32).to_be_bytes());
        buf[112..116].copy_from_slice(&0xE571F1E1u32.to_be_bytes());
        buf
    }

    #[test]
    fn parse_valid_header() {
        let buf = valid_header_bytes();
        let h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.envelope_length(), 144);
        assert_eq!(h.file_length(), 0x00280000);
        assert_eq!(h.version(), "12.13.9.1");
        assert_eq!(h.msdh_count(), 15);
        assert_eq!(h.library_persistent_id(), 0x1234567890ABCDEF);
        assert_eq!(h.max_crypt_size(), 102_400);
        assert_eq!(h.tz_offset_seconds(), -18_000);
        assert_eq!(h.library_date_raw(), 0xE571F1E1);
    }

    #[test]
    fn parse_too_short() {
        let buf = [0u8; 100];
        let err = EnvelopeHeader::parse(&buf).unwrap_err();
        assert!(matches!(err, ItlError::UnexpectedEof(100)));
    }

    #[test]
    fn parse_wrong_magic() {
        let mut buf = [0u8; ENVELOPE_LENGTH];
        buf[0..4].copy_from_slice(b"XXXX");
        let err = EnvelopeHeader::parse(&buf).unwrap_err();
        match err {
            ItlError::InvalidMagic { expected, got } => {
                assert_eq!(expected, ENVELOPE_MAGIC);
                assert_eq!(got, b"XXXX".to_vec());
            }
            other => panic!("expected InvalidMagic, got {other:?}"),
        }
    }

    #[test]
    fn envelope_length_reads_bytes_four_to_eight() {
        let mut buf = valid_header_bytes();
        let expected = 0xDEADBEEFu32;
        buf[4..8].copy_from_slice(&expected.to_be_bytes());
        let h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.envelope_length(), expected);
    }

    #[test]
    fn file_length_and_set_file_length() {
        let buf = valid_header_bytes();
        let mut h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.file_length(), 0x00280000);
        h.set_file_length(999);
        assert_eq!(h.file_length(), 999);
    }

    #[test]
    fn version_empty_when_length_zero() {
        let mut buf = valid_header_bytes();
        buf[16] = 0;
        let h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.version(), "");
    }

    #[test]
    fn tz_offset_seconds_negative() {
        let buf = valid_header_bytes();
        let h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.tz_offset_seconds(), -18_000);
        assert_eq!(
            h.as_bytes()[100..104],
            (-18000i32).to_be_bytes(),
            "tz offset must be big-endian i32 at [100..104)"
        );
    }

    #[test]
    fn as_bytes_round_trip() {
        let buf = valid_header_bytes();
        let h = EnvelopeHeader::parse(&buf).unwrap();
        assert_eq!(h.as_bytes(), &buf);
    }
}
