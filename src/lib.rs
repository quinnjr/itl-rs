//! # itl-rs
//!
//! A library for reading and writing iTunes Library.itl files.
//!
//! The ITL format is a proprietary binary format used by Apple iTunes to store
//! library metadata including tracks, playlists, albums, and artists. The file
//! consists of a big-endian envelope header followed by an AES-128-ECB encrypted
//! and zlib-compressed payload containing little-endian data sections.
//!
//! ## Usage
//!
//! ```no_run
//! use itl_rs::ItlFile;
//!
//! let library = ItlFile::open("/path/to/iTunes Library.itl").unwrap();
//!
//! println!("iTunes version: {}", library.version());
//! println!("Tracks: {}", library.tracks().len());
//!
//! for track in library.tracks() {
//!     if let Some(title) = track.title() {
//!         println!("  {} - {}", title, track.artist().unwrap_or("Unknown"));
//!     }
//! }
//! ```

mod crypto;
mod error;
mod header;
mod parse;
mod types;
mod write;

pub use error::{ItlError, Result};
pub use types::{
    Album, Artist, DataContent, DataField, DataFieldType, Playlist, RawSection, StringEncoding,
    Track, apple_to_unix, unix_to_apple,
};

use header::{ENVELOPE_LENGTH, EnvelopeHeader};
use parse::ParsedLibrary;
use std::path::Path;

/// Handle to a parsed iTunes Library.itl file.
///
/// Provides access to tracks, playlists, albums, artists, and library
/// metadata. Supports round-trip read/write.
pub struct ItlFile {
    header: EnvelopeHeader,
    library: ParsedLibrary,
}

impl ItlFile {
    /// Open and parse an iTunes Library.itl file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let raw = std::fs::read(path)?;
        Self::from_bytes(&raw)
    }

    /// Parse an ITL file from raw bytes.
    pub fn from_bytes(raw: &[u8]) -> Result<Self> {
        let header = EnvelopeHeader::parse(raw)?;
        let payload = &raw[ENVELOPE_LENGTH..];
        let decompressed = crypto::decrypt_payload(payload, header.max_crypt_size())?;
        let library = parse::parse_inner(&decompressed)?;
        Ok(Self { header, library })
    }

    /// Write the library back to a file.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Serialize the library to raw ITL bytes.
    pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
        self.library.reindex();
        let inner = write::serialize_inner(&self.library)?;
        let encrypted = crypto::encrypt_payload(&inner, self.header.max_crypt_size())?;

        let total_len = ENVELOPE_LENGTH + encrypted.len();
        self.header.set_file_length(total_len as u32);

        let mut out = Vec::with_capacity(total_len);
        out.extend_from_slice(self.header.as_bytes());
        out.extend_from_slice(&encrypted);
        Ok(out)
    }

    /// iTunes version string from the envelope header (e.g. "12.13.9.1").
    pub fn version(&self) -> &str {
        self.header.version()
    }

    /// 64-bit library persistent ID.
    pub fn library_persistent_id(&self) -> u64 {
        self.header.library_persistent_id()
    }

    /// Library date as an Apple timestamp (seconds since 1904-01-01).
    pub fn library_date_raw(&self) -> u32 {
        self.header.library_date_raw()
    }

    /// Library date as a Unix timestamp.
    pub fn library_date_unix(&self) -> i64 {
        apple_to_unix(self.library_date_raw())
    }

    /// Timezone offset in seconds (e.g. -18000 for US Eastern).
    pub fn tz_offset_seconds(&self) -> i32 {
        self.header.tz_offset_seconds()
    }

    /// Library share name, if present.
    pub fn share_name(&self) -> Option<&str> {
        self.library.library_info.as_ref()?.share_name()
    }

    /// All tracks in the library.
    pub fn tracks(&self) -> &[Track] {
        &self.library.tracks
    }

    /// Mutable access to all tracks.
    pub fn tracks_mut(&mut self) -> &mut Vec<Track> {
        &mut self.library.tracks
    }

    /// All playlists in the library.
    pub fn playlists(&self) -> &[Playlist] {
        &self.library.playlists
    }

    /// Mutable access to all playlists.
    pub fn playlists_mut(&mut self) -> &mut Vec<Playlist> {
        &mut self.library.playlists
    }

    /// All albums in the library.
    pub fn albums(&self) -> &[Album] {
        &self.library.albums
    }

    /// Mutable access to all albums.
    pub fn albums_mut(&mut self) -> &mut Vec<Album> {
        &mut self.library.albums
    }

    /// All artists in the library.
    pub fn artists(&self) -> &[Artist] {
        &self.library.artists
    }

    /// Mutable access to all artists.
    pub fn artists_mut(&mut self) -> &mut Vec<Artist> {
        &mut self.library.artists
    }

    /// Find a track by its short ID.
    pub fn track_by_id(&self, id: u32) -> Option<&Track> {
        self.library.tracks.iter().find(|t| t.id() == id)
    }

    /// Find a track by its short ID (mutable).
    pub fn track_by_id_mut(&mut self, id: u32) -> Option<&mut Track> {
        self.library.tracks.iter_mut().find(|t| t.id() == id)
    }

    /// Resolve a playlist's track IDs into track references.
    pub fn playlist_tracks(&self, playlist: &Playlist) -> Vec<&Track> {
        playlist
            .track_ids()
            .iter()
            .filter_map(|&id| self.track_by_id(id))
            .collect()
    }

    /// Number of msdh sections reported by the header.
    pub fn msdh_count(&self) -> u32 {
        self.header.msdh_count()
    }

    /// Access the raw envelope header.
    pub fn envelope_header(&self) -> &[u8; ENVELOPE_LENGTH] {
        self.header.as_bytes()
    }
}

impl std::fmt::Debug for ItlFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItlFile")
            .field("version", &self.version())
            .field("library_id", &format_args!("{:#018X}", self.library_persistent_id()))
            .field("tracks", &self.tracks().len())
            .field("playlists", &self.playlists().len())
            .field("albums", &self.albums().len())
            .field("artists", &self.artists().len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_apple_epoch_conversion() {
        let apple = 3_659_329_801u32;
        let unix = apple_to_unix(apple);
        assert_eq!(unix, 1_576_485_001);

        let back = unix_to_apple(unix);
        assert_eq!(back, apple);
    }

    #[test]
    fn test_from_bytes_too_short() {
        let result = ItlFile::from_bytes(&[0u8; 50]);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ItlError::UnexpectedEof(_)),
        );
    }

    #[test]
    fn test_from_bytes_wrong_magic() {
        let mut data = [0u8; 200];
        data[0..4].copy_from_slice(b"XXXX");
        let result = ItlFile::from_bytes(&data);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ItlError::InvalidMagic { .. }),
        );
    }

    fn build_minimal_itl() -> Vec<u8> {
        // Build a minimal valid ITL: header + encrypted(compressed(inner_data))
        let mut header = [0u8; 0x90];
        header[0..4].copy_from_slice(b"hdfm");
        header[4..8].copy_from_slice(&0x90u32.to_be_bytes());
        // max_crypt_size at offset 92
        header[92..96].copy_from_slice(&1024u32.to_be_bytes());
        // version string
        header[16] = 5;
        header[17..22].copy_from_slice(b"1.0.0");
        // msdh count
        header[48..52].copy_from_slice(&0u32.to_be_bytes());
        // library persistent id
        header[52..60].copy_from_slice(&0xAABBCCDD11223344u64.to_be_bytes());
        // tz offset
        header[100..104].copy_from_slice(&(-3600i32).to_be_bytes());
        // library date
        header[112..116].copy_from_slice(&3_659_329_801u32.to_be_bytes());

        // Inner data: empty (no sections)
        let inner_data: &[u8] = &[];
        let encrypted = crypto::encrypt_payload(inner_data, 1024).unwrap();

        let file_len = 0x90 + encrypted.len();
        header[8..12].copy_from_slice(&(file_len as u32).to_be_bytes());

        let mut file = Vec::new();
        file.extend_from_slice(&header);
        file.extend_from_slice(&encrypted);
        file
    }

    #[test]
    fn test_from_bytes_minimal_valid() {
        let data = build_minimal_itl();
        let lib = ItlFile::from_bytes(&data).unwrap();
        assert_eq!(lib.version(), "1.0.0");
        assert_eq!(lib.library_persistent_id(), 0xAABBCCDD11223344);
        assert_eq!(lib.tz_offset_seconds(), -3600);
        assert_eq!(lib.library_date_raw(), 3_659_329_801);
        assert!(lib.library_date_unix() > 0);
        assert_eq!(lib.msdh_count(), 0);
        assert!(lib.tracks().is_empty());
        assert!(lib.playlists().is_empty());
        assert!(lib.albums().is_empty());
        assert!(lib.artists().is_empty());
        assert!(lib.share_name().is_none());
    }

    #[test]
    fn test_to_bytes_roundtrip() {
        let data = build_minimal_itl();
        let mut lib = ItlFile::from_bytes(&data).unwrap();
        let bytes = lib.to_bytes().unwrap();

        let lib2 = ItlFile::from_bytes(&bytes).unwrap();
        assert_eq!(lib2.version(), "1.0.0");
        assert_eq!(lib2.library_persistent_id(), 0xAABBCCDD11223344);
    }

    #[test]
    fn test_mutable_accessors() {
        let data = build_minimal_itl();
        let mut lib = ItlFile::from_bytes(&data).unwrap();

        // tracks_mut
        lib.tracks_mut().push(Track {
            raw_header: {
                let mut h = vec![0u8; 200];
                h[8..12].copy_from_slice(&42u32.to_le_bytes());
                h
            },
            data_fields: vec![],
        });
        assert_eq!(lib.tracks().len(), 1);
        assert_eq!(lib.tracks()[0].id(), 42);

        // track_by_id
        assert!(lib.track_by_id(42).is_some());
        assert!(lib.track_by_id(999).is_none());

        // track_by_id_mut
        assert!(lib.track_by_id_mut(42).is_some());
        assert!(lib.track_by_id_mut(999).is_none());

        // playlists_mut
        lib.playlists_mut().push(Playlist {
            raw_header: vec![0u8; 20],
            data_fields: vec![DataField {
                raw_header: Vec::new(),
                subtype: DataFieldType::PlaylistTitle as u32,
                content: DataContent::String {
                    encoding: StringEncoding::Utf8,
                    value: "Test PL".to_string(),
                },
            }],
            track_ids: vec![42],
        });
        assert_eq!(lib.playlists().len(), 1);
        assert_eq!(lib.playlists()[0].title(), Some("Test PL"));

        // playlist_tracks
        let resolved = lib.playlist_tracks(&lib.playlists()[0].clone());
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].id(), 42);

        // albums_mut
        lib.albums_mut().push(Album {
            raw_header: vec![0u8; 40],
            data_fields: vec![],
        });
        assert_eq!(lib.albums().len(), 1);

        // artists_mut
        lib.artists_mut().push(Artist {
            raw_header: vec![0u8; 24],
            data_fields: vec![],
        });
        assert_eq!(lib.artists().len(), 1);
    }

    #[test]
    fn test_envelope_header_accessor() {
        let data = build_minimal_itl();
        let lib = ItlFile::from_bytes(&data).unwrap();
        let header = lib.envelope_header();
        assert_eq!(&header[0..4], b"hdfm");
    }

    #[test]
    fn test_debug_impl() {
        let data = build_minimal_itl();
        let lib = ItlFile::from_bytes(&data).unwrap();
        let debug_str = format!("{:?}", lib);
        assert!(debug_str.contains("ItlFile"));
        assert!(debug_str.contains("1.0.0"));
        assert!(debug_str.contains("tracks"));
        assert!(debug_str.contains("playlists"));
        assert!(debug_str.contains("albums"));
        assert!(debug_str.contains("artists"));
    }

    #[test]
    fn test_save_and_reopen() {
        let data = build_minimal_itl();
        let mut lib = ItlFile::from_bytes(&data).unwrap();

        let tmp = std::env::temp_dir().join("itl_rs_test_save.itl");
        lib.save(&tmp).unwrap();

        let lib2 = ItlFile::open(&tmp).unwrap();
        assert_eq!(lib2.version(), "1.0.0");
        assert_eq!(lib2.library_persistent_id(), 0xAABBCCDD11223344);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_open_nonexistent_file() {
        let result = ItlFile::open("/nonexistent/path/file.itl");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ItlError::Io(_)));
    }
}
