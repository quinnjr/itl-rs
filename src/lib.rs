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
    /// Set when a mutable collection accessor has been handed out;
    /// `to_bytes` reindexes sections only in that case, so a pure
    /// read→save round trip preserves original section membership.
    dirty: bool,
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
        Ok(Self {
            header,
            library,
            dirty: false,
        })
    }

    /// Write the library back to a file.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Serialize the library to raw ITL bytes.
    pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
        if self.dirty {
            self.library.reindex();
            self.dirty = false;
        }
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
        self.dirty = true;
        &mut self.library.tracks
    }

    /// All playlists in the library.
    pub fn playlists(&self) -> &[Playlist] {
        &self.library.playlists
    }

    /// Mutable access to all playlists.
    pub fn playlists_mut(&mut self) -> &mut Vec<Playlist> {
        self.dirty = true;
        &mut self.library.playlists
    }

    /// All albums in the library.
    pub fn albums(&self) -> &[Album] {
        &self.library.albums
    }

    /// Mutable access to all albums.
    pub fn albums_mut(&mut self) -> &mut Vec<Album> {
        self.dirty = true;
        &mut self.library.albums
    }

    /// All artists in the library.
    pub fn artists(&self) -> &[Artist] {
        &self.library.artists
    }

    /// Mutable access to all artists.
    pub fn artists_mut(&mut self) -> &mut Vec<Artist> {
        self.dirty = true;
        &mut self.library.artists
    }

    /// Find a track by its short ID.
    pub fn track_by_id(&self, id: u32) -> Option<&Track> {
        self.library.tracks.iter().find(|t| t.id() == id)
    }

    /// Find a track by its short ID (mutable).
    pub fn track_by_id_mut(&mut self, id: u32) -> Option<&mut Track> {
        // Same contract as the *_mut collection accessors: any handed-out
        // mutable access marks the library dirty. reindex() is a no-op
        // when section membership is still intact, so this is cheap.
        self.dirty = true;
        self.library.tracks.iter_mut().find(|t| t.id() == id)
    }

    /// Resolve a playlist's track IDs into track references.
    pub fn playlist_tracks(&self, playlist: &Playlist) -> Vec<&Track> {
        // One O(n) index build instead of a linear scan per entry; first
        // occurrence wins, matching track_by_id.
        let mut index = std::collections::HashMap::with_capacity(self.library.tracks.len());
        for track in &self.library.tracks {
            index.entry(track.id()).or_insert(track);
        }
        playlist
            .track_ids()
            .iter()
            .filter_map(|id| index.get(id).copied())
            .collect()
    }

    /// Unparsed non-msdh top-level sections, preserved verbatim through
    /// a read→save round trip. Derived from the serialization order, the
    /// single owner of the bytes.
    pub fn raw_sections(&self) -> Vec<RawSection> {
        self.library
            .section_order
            .iter()
            .filter_map(|s| match s {
                parse::SectionRef::Raw { data } => Some(RawSection {
                    sig: data
                        .get(0..4)
                        .and_then(|s| s.try_into().ok())
                        .unwrap_or(*b"????"),
                    data: data.clone(),
                }),
                _ => None,
            })
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
            .field(
                "library_id",
                &format_args!("{:#018X}", self.library_persistent_id()),
            )
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
        assert!(matches!(result.unwrap_err(), ItlError::UnexpectedEof(_)),);
    }

    #[test]
    fn test_from_bytes_wrong_magic() {
        let mut data = [0u8; 200];
        data[0..4].copy_from_slice(b"XXXX");
        let result = ItlFile::from_bytes(&data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ItlError::InvalidMagic { .. }),);
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
            entries: vec![types::PlaylistEntry {
                track_id: 42,
                raw: None,
            }],
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

    fn build_itl_with_payload(inner_data: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 0x90];
        header[0..4].copy_from_slice(b"hdfm");
        header[4..8].copy_from_slice(&0x90u32.to_be_bytes());
        header[92..96].copy_from_slice(&1024u32.to_be_bytes());
        header[16] = 5;
        header[17..22].copy_from_slice(b"1.0.0");
        let encrypted = crypto::encrypt_payload(inner_data, 1024).unwrap();
        let file_len = 0x90 + encrypted.len();
        header[8..12].copy_from_slice(&(file_len as u32).to_be_bytes());
        let mut file = Vec::new();
        file.extend_from_slice(&header);
        file.extend_from_slice(&encrypted);
        file
    }

    #[test]
    fn test_unknown_toplevel_section_survives_round_trip() {
        // A non-msdh top-level section must be preserved byte-for-byte
        // through open → save.
        let mut inner = Vec::new();
        inner.extend_from_slice(b"zzzz");
        inner.extend_from_slice(&16u32.to_le_bytes());
        inner.extend_from_slice(&[0xEE; 8]);

        let data = build_itl_with_payload(&inner);
        let mut lib = ItlFile::from_bytes(&data).unwrap();
        assert_eq!(lib.raw_sections().len(), 1);
        assert_eq!(lib.raw_sections()[0].sig, *b"zzzz");

        let bytes = lib.to_bytes().unwrap();
        let lib2 = ItlFile::from_bytes(&bytes).unwrap();
        assert_eq!(lib2.raw_sections().len(), 1);
        assert_eq!(lib2.raw_sections()[0].data, lib.raw_sections()[0].data);
    }

    #[test]
    fn test_pure_round_trip_preserves_section_membership() {
        // Two track-list sections with one track each: an unmutated
        // open → to_bytes must NOT merge them into the first section.
        fn build_mlth_msdh(track_id: u32, subtype: u32) -> Vec<u8> {
            let mut mith = Vec::new();
            mith.extend_from_slice(b"mith");
            let section_length = 16 + 184u32;
            mith.extend_from_slice(&section_length.to_le_bytes());
            mith.extend_from_slice(&section_length.to_le_bytes()); // assoc_length, no mhohs
            mith.extend_from_slice(&0u32.to_le_bytes()); // mhoh_count
            let mut extra = vec![0u8; 184];
            extra[0..4].copy_from_slice(&track_id.to_le_bytes());
            mith.extend_from_slice(&extra);

            let mut mlth = Vec::new();
            mlth.extend_from_slice(b"mlth");
            mlth.extend_from_slice(&92u32.to_le_bytes());
            mlth.extend_from_slice(&1u32.to_le_bytes());
            mlth.extend_from_slice(&[0u8; 92 - 12]);

            let content_len = (mlth.len() + mith.len()) as u32;
            let mut msdh = Vec::new();
            msdh.extend_from_slice(b"msdh");
            msdh.extend_from_slice(&96u32.to_le_bytes());
            msdh.extend_from_slice(&(96 + content_len).to_le_bytes());
            msdh.extend_from_slice(&subtype.to_le_bytes());
            msdh.extend_from_slice(&[0u8; 96 - 16]);
            msdh.extend_from_slice(&mlth);
            msdh.extend_from_slice(&mith);
            msdh
        }

        fn track_list_ranges(bytes: &[u8]) -> Vec<std::ops::Range<usize>> {
            let decompressed = crypto::decrypt_payload(&bytes[ENVELOPE_LENGTH..], 1024).unwrap();
            let reparsed = parse::parse_inner(&decompressed).unwrap();
            reparsed
                .section_order
                .iter()
                .filter_map(|s| match s {
                    parse::SectionRef::Msdh {
                        content: parse::MsdhContent::TrackList { range, .. },
                        ..
                    } => Some(range.clone()),
                    _ => None,
                })
                .collect()
        }

        let inner = [build_mlth_msdh(1, 1), build_mlth_msdh(2, 13)].concat();
        let data = build_itl_with_payload(&inner);
        let mut lib = ItlFile::from_bytes(&data).unwrap();
        assert_eq!(lib.tracks().len(), 2);

        // No mutation: serialize and re-inspect the inner payload.
        let ranges = track_list_ranges(&lib.to_bytes().unwrap());
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].len(), 1, "first section must keep its one track");
        assert_eq!(ranges[1].len(), 1, "second section must keep its one track");

        // Field-only edit through tracks_mut(): membership must STILL be
        // preserved — reindex leaves intact partitions alone.
        lib.tracks_mut()[0].set_title("Edited");
        let ranges = track_list_ranges(&lib.to_bytes().unwrap());
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            ranges[0].len(),
            1,
            "field edit must not collapse section membership"
        );
        assert_eq!(ranges[1].len(), 1);

        // After a membership mutation, reindex consolidates as documented.
        lib.tracks_mut().remove(1);
        let bytes = lib.to_bytes().unwrap();
        let lib3 = ItlFile::from_bytes(&bytes).unwrap();
        assert_eq!(lib3.tracks().len(), 1);
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
