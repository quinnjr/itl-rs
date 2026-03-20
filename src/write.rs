use crate::error::Result;
use crate::parse::{LibraryInfo, MsdhContent, ParsedLibrary, SectionRef};
use crate::types::*;

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self {
            buf: Vec::with_capacity(1024 * 1024),
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    fn write_u32_le(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    #[allow(dead_code)]
    fn write_u16_le(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn pos(&self) -> usize {
        self.buf.len()
    }

    fn patch_u32_le(&mut self, offset: usize, v: u32) {
        self.buf[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
    }
}

pub fn serialize_inner(library: &ParsedLibrary) -> Result<Vec<u8>> {
    let mut w = Writer::new();

    for section_ref in &library.section_order {
        write_section(&mut w, section_ref, library)?;
    }

    Ok(w.buf)
}

fn write_section(w: &mut Writer, section_ref: &SectionRef, library: &ParsedLibrary) -> Result<()> {
    match section_ref {
        SectionRef::Msdh {
            raw_header,
            subtype: _,
            content,
        } => {
            let msdh_start = w.pos();

            // Write the msdh header — we'll patch the associated data length after
            w.write_bytes(raw_header);

            match content {
                MsdhContent::InnerHeader => {
                    if let Some(ref header) = library.inner_header {
                        w.write_bytes(header);
                    }
                }
                MsdhContent::LibraryInfo => {
                    if let Some(ref info) = library.library_info {
                        write_library_info(w, info)?;
                    }
                }
                MsdhContent::TrackList { raw_header: master, range } => {
                    let master_start = w.pos();
                    w.write_bytes(master);
                    let items = &library.tracks[range.clone()];
                    let count_offset = master_start + 8;
                    w.patch_u32_le(count_offset, items.len() as u32);
                    for track in items {
                        write_track_item(w, track)?;
                    }
                }
                MsdhContent::AlbumList { raw_header: master, range } => {
                    let master_start = w.pos();
                    w.write_bytes(master);
                    let items = &library.albums[range.clone()];
                    let count_offset = master_start + 8;
                    w.patch_u32_le(count_offset, items.len() as u32);
                    for album in items {
                        write_album_item(w, album)?;
                    }
                }
                MsdhContent::ArtistList { raw_header: master, range } => {
                    let master_start = w.pos();
                    w.write_bytes(master);
                    let items = &library.artists[range.clone()];
                    let count_offset = master_start + 8;
                    w.patch_u32_le(count_offset, items.len() as u32);
                    for artist in items {
                        write_artist_item(w, artist)?;
                    }
                }
                MsdhContent::PlaylistList { raw_header: master, range } => {
                    w.write_bytes(master);
                    for playlist in &library.playlists[range.clone()] {
                        write_playlist(w, playlist)?;
                    }
                }
                MsdhContent::RawBlob(blob) => {
                    w.write_bytes(blob);
                }
                MsdhContent::Unknown(blob) => {
                    w.write_bytes(blob);
                }
            }

            // Patch the associated data length in the msdh header (offset 8 from start, LE u32)
            let total_size = (w.pos() - msdh_start) as u32;
            w.patch_u32_le(msdh_start + 8, total_size);
        }
    }
    Ok(())
}

fn write_library_info(w: &mut Writer, info: &LibraryInfo) -> Result<()> {
    let start = w.pos();
    w.write_bytes(&info.raw_header);

    // Patch mhoh count
    let count_offset = start + 8;
    w.patch_u32_le(count_offset, info.data_fields.len() as u32);

    for field in &info.data_fields {
        write_mhoh(w, field)?;
    }
    Ok(())
}

fn write_track_item(w: &mut Writer, track: &Track) -> Result<()> {
    let start = w.pos();

    // sig (4 bytes) + section_length (4 bytes)
    w.write_bytes(b"mith");
    let section_length_offset = w.pos();
    w.write_u32_le(0);

    // raw_header includes assoc_length (offset 0), mhoh_count (offset 4),
    // track_id (offset 8), and all remaining header fields
    w.write_bytes(&track.raw_header);

    let section_length = (w.pos() - start) as u32;
    w.patch_u32_le(section_length_offset, section_length);

    // Patch mhoh count at section offset 12 (= raw_header offset 4)
    w.patch_u32_le(start + 12, track.data_fields.len() as u32);

    for field in &track.data_fields {
        write_mhoh(w, field)?;
    }

    // Patch assoc_length at section offset 8 (= raw_header offset 0)
    let assoc_length = (w.pos() - start) as u32;
    w.patch_u32_le(start + 8, assoc_length);

    Ok(())
}

fn write_album_item(w: &mut Writer, album: &Album) -> Result<()> {
    let start = w.pos();

    w.write_bytes(b"miah");
    let section_length_offset = w.pos();
    w.write_u32_le(0);

    w.write_bytes(&album.raw_header);

    let section_length = (w.pos() - start) as u32;
    w.patch_u32_le(section_length_offset, section_length);
    w.patch_u32_le(start + 12, album.data_fields.len() as u32);

    for field in &album.data_fields {
        write_mhoh(w, field)?;
    }

    let assoc_length = (w.pos() - start) as u32;
    w.patch_u32_le(start + 8, assoc_length);

    Ok(())
}

fn write_artist_item(w: &mut Writer, artist: &Artist) -> Result<()> {
    let start = w.pos();

    w.write_bytes(b"miih");
    let section_length_offset = w.pos();
    w.write_u32_le(0);

    w.write_bytes(&artist.raw_header);

    let section_length = (w.pos() - start) as u32;
    w.patch_u32_le(section_length_offset, section_length);
    w.patch_u32_le(start + 12, artist.data_fields.len() as u32);

    for field in &artist.data_fields {
        write_mhoh(w, field)?;
    }

    let assoc_length = (w.pos() - start) as u32;
    w.patch_u32_le(start + 8, assoc_length);

    Ok(())
}

fn write_playlist(w: &mut Writer, playlist: &Playlist) -> Result<()> {
    // miph header
    w.write_bytes(b"miph");
    let len = playlist.raw_header.len() as u32 + 8;
    w.write_u32_le(len);
    w.write_bytes(&playlist.raw_header);

    // mhoh data fields
    for field in &playlist.data_fields {
        write_mhoh(w, field)?;
    }

    // mtph track references
    for &track_id in &playlist.track_ids {
        write_playlist_track(w, track_id)?;
    }

    Ok(())
}

fn write_playlist_track(w: &mut Writer, track_id: u32) -> Result<()> {
    w.write_bytes(b"mtph");
    w.write_u32_le(28); // typical section length
    w.write_bytes(&[0u8; 16]); // padding/unknown
    w.write_u32_le(track_id);
    Ok(())
}

fn write_mhoh(w: &mut Writer, field: &DataField) -> Result<()> {
    let start = w.pos();

    w.write_bytes(b"mhoh");
    w.write_u32_le(0x18); // dummy, always 24
    let total_length_offset = w.pos();
    w.write_u32_le(0); // placeholder for total length
    w.write_u32_le(field.subtype);
    w.write_bytes(&[0u8; 8]); // remaining common header

    match &field.content {
        DataContent::RawData(data) => {
            w.write_bytes(data);
        }
        DataContent::String { encoding, value } => {
            let encoded = encode_string(*encoding, value);
            w.write_u32_le(*encoding as u32);
            w.write_u32_le(encoded.len() as u32);
            w.write_bytes(&[0u8; 8]); // padding
            w.write_bytes(&encoded);
        }
    }

    let total_length = (w.pos() - start) as u32;
    w.patch_u32_le(total_length_offset, total_length);

    Ok(())
}

fn encode_string(encoding: StringEncoding, value: &str) -> Vec<u8> {
    match encoding {
        StringEncoding::Utf8 | StringEncoding::Uri | StringEncoding::EscapedUri => {
            value.as_bytes().to_vec()
        }
        StringEncoding::Utf16 => {
            let u16s: Vec<u16> = value.encode_utf16().collect();
            let mut bytes = Vec::with_capacity(u16s.len() * 2);
            for u in u16s {
                bytes.extend_from_slice(&u.to_le_bytes());
            }
            bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{self, LibraryInfo, MsdhContent, ParsedLibrary, SectionRef};
    #[allow(unused_imports)]
    use crate::types::*;

    fn make_string_field(subtype: u32, enc: StringEncoding, value: &str) -> DataField {
        DataField {
            raw_header: Vec::new(),
            subtype,
            content: DataContent::String {
                encoding: enc,
                value: value.to_string(),
            },
        }
    }

    fn make_raw_field(subtype: u32, data: &[u8]) -> DataField {
        DataField {
            raw_header: Vec::new(),
            subtype,
            content: DataContent::RawData(data.to_vec()),
        }
    }

    fn make_track(id: u32, fields: Vec<DataField>) -> Track {
        let mut raw_header = vec![0u8; 200];
        raw_header[0..4].copy_from_slice(&0u32.to_le_bytes()); // assoc_length placeholder
        raw_header[4..8].copy_from_slice(&(fields.len() as u32).to_le_bytes());
        raw_header[8..12].copy_from_slice(&id.to_le_bytes());
        Track {
            raw_header,
            data_fields: fields,
        }
    }

    fn make_album(fields: Vec<DataField>) -> Album {
        let mut raw_header = vec![0u8; 80];
        raw_header[4..8].copy_from_slice(&(fields.len() as u32).to_le_bytes());
        Album {
            raw_header,
            data_fields: fields,
        }
    }

    fn make_artist(fields: Vec<DataField>) -> Artist {
        let mut raw_header = vec![0u8; 80];
        raw_header[4..8].copy_from_slice(&(fields.len() as u32).to_le_bytes());
        Artist {
            raw_header,
            data_fields: fields,
        }
    }

    fn make_playlist(title: &str, track_ids: Vec<u32>) -> Playlist {
        let raw_header = vec![0u8; 80];
        Playlist {
            raw_header,
            data_fields: vec![make_string_field(
                DataFieldType::PlaylistTitle as u32,
                StringEncoding::Utf8,
                title,
            )],
            track_ids,
        }
    }

    fn make_empty_library() -> ParsedLibrary {
        ParsedLibrary {
            inner_header: None,
            library_info: None,
            tracks: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            playlists: Vec::new(),
            raw_sections: Vec::new(),
            section_order: Vec::new(),
        }
    }

    #[test]
    fn test_encode_string_utf8() {
        let result = encode_string(StringEncoding::Utf8, "hello");
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_encode_string_uri() {
        let result = encode_string(StringEncoding::Uri, "file://path");
        assert_eq!(result, b"file://path");
    }

    #[test]
    fn test_encode_string_escaped_uri() {
        let result = encode_string(StringEncoding::EscapedUri, "http://example.com");
        assert_eq!(result, b"http://example.com");
    }

    #[test]
    fn test_encode_string_utf16() {
        let result = encode_string(StringEncoding::Utf16, "AB");
        assert_eq!(result, &[0x41, 0x00, 0x42, 0x00]);
    }

    #[test]
    fn test_encode_string_utf16_multibyte() {
        let result = encode_string(StringEncoding::Utf16, "\u{00E9}"); // é
        assert_eq!(result, &[0xE9, 0x00]);
    }

    #[test]
    fn test_serialize_inner_empty() {
        let library = make_empty_library();
        let result = serialize_inner(&library).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_write_mhoh_string_roundtrip() {
        let field = make_string_field(0x0002, StringEncoding::Utf8, "Test Song");
        let mut w = Writer::new();
        write_mhoh(&mut w, &field).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_mhoh(&mut cursor).unwrap();
        assert_eq!(parsed.subtype, 0x0002);
        assert_eq!(parsed.as_str(), Some("Test Song"));
    }

    #[test]
    fn test_write_mhoh_utf16_roundtrip() {
        let field = make_string_field(0x0004, StringEncoding::Utf16, "Artist");
        let mut w = Writer::new();
        write_mhoh(&mut w, &field).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_mhoh(&mut cursor).unwrap();
        assert_eq!(parsed.subtype, 0x0004);
        assert_eq!(parsed.as_str(), Some("Artist"));
    }

    #[test]
    fn test_write_mhoh_raw_roundtrip() {
        let field = make_raw_field(0x0036, b"<xml>data</xml>");
        let mut w = Writer::new();
        write_mhoh(&mut w, &field).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_mhoh(&mut cursor).unwrap();
        assert_eq!(parsed.subtype, 0x0036);
        match &parsed.content {
            DataContent::RawData(d) => assert_eq!(d, b"<xml>data</xml>"),
            other => panic!("expected RawData, got {:?}", other),
        }
    }

    #[test]
    fn test_write_track_item_roundtrip() {
        let track = make_track(
            42,
            vec![
                make_string_field(0x0002, StringEncoding::Utf8, "My Song"),
                make_string_field(0x0004, StringEncoding::Utf8, "My Artist"),
            ],
        );

        let mut w = Writer::new();
        write_track_item(&mut w, &track).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_track_item(&mut cursor).unwrap();
        assert_eq!(parsed.id(), 42);
        assert_eq!(parsed.title(), Some("My Song"));
        assert_eq!(parsed.artist(), Some("My Artist"));
    }

    #[test]
    fn test_write_album_item_roundtrip() {
        let album = make_album(vec![make_string_field(
            DataFieldType::AlbumItemName as u32,
            StringEncoding::Utf8,
            "Test Album",
        )]);

        let mut w = Writer::new();
        write_album_item(&mut w, &album).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_album_item(&mut cursor).unwrap();
        assert_eq!(parsed.name(), Some("Test Album"));
    }

    #[test]
    fn test_write_artist_item_roundtrip() {
        let artist = make_artist(vec![make_string_field(
            DataFieldType::ArtistName as u32,
            StringEncoding::Utf8,
            "Test Artist",
        )]);

        let mut w = Writer::new();
        write_artist_item(&mut w, &artist).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let parsed = parse::parse_artist_item(&mut cursor).unwrap();
        assert_eq!(parsed.name(), Some("Test Artist"));
    }

    #[test]
    fn test_write_playlist_roundtrip() {
        let playlist = make_playlist("My Playlist", vec![10, 20, 30]);

        let mut w = Writer::new();
        write_playlist(&mut w, &playlist).unwrap();

        // Manually parse: miph header + mhoh fields + mtph entries
        let data = &w.buf;
        assert_eq!(&data[0..4], b"miph");
        // Verify mtph entries are present
        let mtph_count = data
            .windows(4)
            .filter(|w| *w == b"mtph")
            .count();
        assert_eq!(mtph_count, 3);
    }

    #[test]
    fn test_write_playlist_track_roundtrip() {
        let mut w = Writer::new();
        write_playlist_track(&mut w, 999).unwrap();

        let mut cursor = parse::Cursor::new(&w.buf);
        let id = parse::parse_playlist_track(&mut cursor).unwrap();
        assert_eq!(id, 999);
    }

    #[test]
    fn test_serialize_inner_with_sections() {
        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[8..12].copy_from_slice(&0u32.to_le_bytes()); // assoc_length placeholder
            buf[12..16].copy_from_slice(&16u32.to_le_bytes()); // subtype
            buf
        };

        let inner_header = vec![0u8; 144];

        let mut library = make_empty_library();
        library.inner_header = Some(inner_header.clone());
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 16,
            content: MsdhContent::InnerHeader,
        });

        let result = serialize_inner(&library).unwrap();
        assert!(result.len() > 96);
        assert_eq!(&result[0..4], b"msdh");
    }

    #[test]
    fn test_serialize_inner_raw_blob() {
        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&4u32.to_le_bytes()); // raw data subtype
            buf
        };

        let mut library = make_empty_library();
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 4,
            content: MsdhContent::RawBlob(b"raw data blob".to_vec()),
        });

        let result = serialize_inner(&library).unwrap();
        assert!(result.len() > 96);
    }

    #[test]
    fn test_serialize_inner_unknown() {
        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&99u32.to_le_bytes());
            buf
        };

        let mut library = make_empty_library();
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 99,
            content: MsdhContent::Unknown(b"unknown data".to_vec()),
        });

        let result = serialize_inner(&library).unwrap();
        assert!(result.len() > 96);
    }

    #[test]
    fn test_serialize_inner_library_info() {
        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&12u32.to_le_bytes());
            buf
        };

        let info = LibraryInfo {
            raw_header: {
                let mut h = vec![0u8; 280];
                h[0..4].copy_from_slice(b"mhgh");
                h[4..8].copy_from_slice(&280u32.to_le_bytes());
                h[8..12].copy_from_slice(&0u32.to_le_bytes());
                h
            },
            data_fields: vec![make_string_field(
                DataFieldType::LibraryShareName as u32,
                StringEncoding::Utf8,
                "My Library",
            )],
        };

        let mut library = make_empty_library();
        library.library_info = Some(info);
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 12,
            content: MsdhContent::LibraryInfo,
        });

        let result = serialize_inner(&library).unwrap();
        assert!(result.len() > 96 + 280);
    }

    #[test]
    fn test_serialize_track_list() {
        let master_header = {
            let section_length: u32 = 92;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"mlth");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[8..12].copy_from_slice(&0u32.to_le_bytes()); // count placeholder
            buf
        };

        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&1u32.to_le_bytes());
            buf
        };

        let track = make_track(
            7,
            vec![make_string_field(0x0002, StringEncoding::Utf8, "Song")],
        );

        let mut library = make_empty_library();
        library.tracks.push(track);
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 1,
            content: MsdhContent::TrackList {
                raw_header: master_header,
                range: 0..1,
            },
        });

        let result = serialize_inner(&library).unwrap();
        assert!(!result.is_empty());
        // Verify the track count was patched in the master header
        let master_offset = 96; // after msdh header
        let count = u32::from_le_bytes(result[master_offset + 8..master_offset + 12].try_into().unwrap());
        assert_eq!(count, 1);
    }

    #[test]
    fn test_serialize_album_list() {
        let master_header = {
            let section_length: u32 = 92;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"mlah");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf
        };

        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&9u32.to_le_bytes());
            buf
        };

        let album = make_album(vec![make_string_field(
            DataFieldType::AlbumItemName as u32,
            StringEncoding::Utf8,
            "Album",
        )]);

        let mut library = make_empty_library();
        library.albums.push(album);
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 9,
            content: MsdhContent::AlbumList {
                raw_header: master_header,
                range: 0..1,
            },
        });

        let result = serialize_inner(&library).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_serialize_artist_list() {
        let master_header = {
            let section_length: u32 = 92;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"mlih");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf
        };

        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&11u32.to_le_bytes());
            buf
        };

        let artist = make_artist(vec![make_string_field(
            DataFieldType::ArtistName as u32,
            StringEncoding::Utf8,
            "Artist",
        )]);

        let mut library = make_empty_library();
        library.artists.push(artist);
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 11,
            content: MsdhContent::ArtistList {
                raw_header: master_header,
                range: 0..1,
            },
        });

        let result = serialize_inner(&library).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_serialize_playlist_list() {
        let master_header = {
            let section_length: u32 = 92;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"mlph");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf
        };

        let msdh_header = {
            let section_length: u32 = 96;
            let mut buf = vec![0u8; section_length as usize];
            buf[0..4].copy_from_slice(b"msdh");
            buf[4..8].copy_from_slice(&section_length.to_le_bytes());
            buf[12..16].copy_from_slice(&2u32.to_le_bytes());
            buf
        };

        let playlist = make_playlist("Favs", vec![1, 2, 3]);

        let mut library = make_empty_library();
        library.playlists.push(playlist);
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header,
            subtype: 2,
            content: MsdhContent::PlaylistList {
                raw_header: master_header,
                range: 0..1,
            },
        });

        let result = serialize_inner(&library).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_writer_basics() {
        let mut w = Writer::new();
        assert_eq!(w.pos(), 0);

        w.write_bytes(b"test");
        assert_eq!(w.pos(), 4);

        w.write_u32_le(0x12345678);
        assert_eq!(w.pos(), 8);
        assert_eq!(&w.buf[4..8], &[0x78, 0x56, 0x34, 0x12]);

        w.write_u16_le(0xABCD);
        assert_eq!(w.pos(), 10);
        assert_eq!(&w.buf[8..10], &[0xCD, 0xAB]);

        w.patch_u32_le(4, 0xDEADBEEF);
        assert_eq!(&w.buf[4..8], &[0xEF, 0xBE, 0xAD, 0xDE]);
    }
}
