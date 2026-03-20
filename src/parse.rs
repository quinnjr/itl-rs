use crate::error::{ItlError, Result};
use crate::types::*;

/// Binary cursor for reading little-endian data from a byte slice.
pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn ensure(&self, n: usize) -> Result<()> {
        if self.pos + n > self.data.len() {
            Err(ItlError::UnexpectedEof(self.pos))
        } else {
            Ok(())
        }
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        self.ensure(n)?;
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn peek_bytes(&self, n: usize) -> Result<&'a [u8]> {
        if self.pos + n > self.data.len() {
            Err(ItlError::UnexpectedEof(self.pos))
        } else {
            Ok(&self.data[self.pos..self.pos + n])
        }
    }

    pub fn read_sig(&mut self) -> Result<[u8; 4]> {
        let bytes = self.read_bytes(4)?;
        let mut sig = [0u8; 4];
        sig.copy_from_slice(bytes);
        Ok(sig)
    }

    #[allow(dead_code)]
    pub fn read_u8(&mut self) -> Result<u8> {
        self.ensure(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    #[allow(dead_code)]
    pub fn read_u16_le(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
    }

    pub fn read_u32_le(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    #[allow(dead_code)]
    pub fn read_u64_le(&mut self) -> Result<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.ensure(n)?;
        self.pos += n;
        Ok(())
    }

    pub fn slice_from(&self, start: usize) -> &'a [u8] {
        &self.data[start..self.pos]
    }
}

/// Subtypes of msdh sections that contain raw data blobs (no subsections).
fn msdh_is_raw_data_subtype(subtype: u32) -> bool {
    matches!(subtype, 3 | 4 | 19 | 22)
}

/// Parsed representation of the inner (decrypted/decompressed) data.
#[derive(Debug, Clone)]
pub struct ParsedLibrary {
    pub inner_header: Option<Vec<u8>>,
    pub library_info: Option<LibraryInfo>,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
    pub raw_sections: Vec<RawSection>,
    /// Ordered list of top-level sections for round-trip serialization.
    pub(crate) section_order: Vec<SectionRef>,
}

#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub(crate) raw_header: Vec<u8>,
    pub(crate) data_fields: Vec<DataField>,
}

impl LibraryInfo {
    pub fn share_name(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::LibraryShareName as u32)
            .and_then(|f| f.as_str())
    }
}

/// References to sections in order, for serialization.
#[derive(Debug, Clone)]
pub(crate) enum SectionRef {
    Msdh {
        raw_header: Vec<u8>,
        #[allow(dead_code)]
        subtype: u32,
        content: MsdhContent,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum MsdhContent {
    InnerHeader,
    LibraryInfo,
    TrackList { raw_header: Vec<u8>, range: std::ops::Range<usize> },
    AlbumList { raw_header: Vec<u8>, range: std::ops::Range<usize> },
    ArtistList { raw_header: Vec<u8>, range: std::ops::Range<usize> },
    PlaylistList { raw_header: Vec<u8>, range: std::ops::Range<usize> },
    RawBlob(Vec<u8>),
    Unknown(Vec<u8>),
}

impl ParsedLibrary {
    /// Reassign section ranges so the first section of each collection type
    /// owns all items and subsequent sections of the same type are empty.
    /// Must be called before serialization if items have been added/removed
    /// through the public mutable accessors.
    pub(crate) fn reindex(&mut self) {
        let mut track_assigned = false;
        let mut album_assigned = false;
        let mut artist_assigned = false;
        let mut playlist_assigned = false;

        for section in &mut self.section_order {
            let SectionRef::Msdh { content, .. } = section;
            match content {
                MsdhContent::TrackList { range, .. } => {
                    if !track_assigned {
                        *range = 0..self.tracks.len();
                        track_assigned = true;
                    } else {
                        *range = 0..0;
                    }
                }
                MsdhContent::AlbumList { range, .. } => {
                    if !album_assigned {
                        *range = 0..self.albums.len();
                        album_assigned = true;
                    } else {
                        *range = 0..0;
                    }
                }
                MsdhContent::ArtistList { range, .. } => {
                    if !artist_assigned {
                        *range = 0..self.artists.len();
                        artist_assigned = true;
                    } else {
                        *range = 0..0;
                    }
                }
                MsdhContent::PlaylistList { range, .. } => {
                    if !playlist_assigned {
                        *range = 0..self.playlists.len();
                        playlist_assigned = true;
                    } else {
                        *range = 0..0;
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn parse_inner(data: &[u8]) -> Result<ParsedLibrary> {
    let mut cursor = Cursor::new(data);
    let mut library = ParsedLibrary {
        inner_header: None,
        library_info: None,
        tracks: Vec::new(),
        albums: Vec::new(),
        artists: Vec::new(),
        playlists: Vec::new(),
        raw_sections: Vec::new(),
        section_order: Vec::new(),
    };

    while cursor.remaining() >= 8 {
        let section_start = cursor.pos();
        let sig = match cursor.read_sig() {
            Ok(s) => s,
            Err(_) => break,
        };

        match &sig {
            b"msdh" => {
                parse_msdh(&mut cursor, section_start, &mut library)?;
            }
            _ => {
                // Unknown top-level section — try to skip it using its length field
                if cursor.remaining() >= 4 {
                    let len = cursor.read_u32_le()?;
                    let skip = (len as usize).saturating_sub(8);
                    if skip <= cursor.remaining() {
                        cursor.skip(skip)?;
                    } else {
                        break;
                    }
                    library.raw_sections.push(RawSection {
                        sig,
                        data: cursor.slice_from(section_start).to_vec(),
                    });
                } else {
                    break;
                }
            }
        }
    }

    Ok(library)
}

fn parse_msdh(cursor: &mut Cursor, msdh_start: usize, library: &mut ParsedLibrary) -> Result<()> {
    let section_length = cursor.read_u32_le()? as usize;
    let assoc_length = cursor.read_u32_le()? as usize;
    let subtype = cursor.read_u32_le()?;

    let msdh_header_bytes = cursor.data[msdh_start..msdh_start + section_length]
        .to_vec();

    // Skip to end of msdh header
    let remaining_header = section_length.saturating_sub(16);
    cursor.skip(remaining_header)?;

    let content_end = msdh_start + assoc_length;

    if msdh_is_raw_data_subtype(subtype) {
        let blob_size = content_end.saturating_sub(cursor.pos());
        let blob = if blob_size > 0 && blob_size <= cursor.remaining() {
            cursor.read_bytes(blob_size)?.to_vec()
        } else {
            Vec::new()
        };
        library.section_order.push(SectionRef::Msdh {
            raw_header: msdh_header_bytes,
            subtype,
            content: MsdhContent::RawBlob(blob),
        });
        return Ok(());
    }

    match subtype {
        // mfdh inner header
        16 => {
            let header = parse_mfdh(cursor)?;
            library.inner_header = Some(header);
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::InnerHeader,
            });
        }
        // mhgh library info
        12 => {
            let info = parse_mhgh(cursor)?;
            library.library_info = Some(info);
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::LibraryInfo,
            });
        }
        // mlah albums
        9 => {
            let master_header = parse_master_header(cursor, b"mlah")?;
            let count = u32::from_le_bytes(
                master_header[8..12].try_into().unwrap()
            );
            let start_idx = library.albums.len();
            for _ in 0..count {
                if cursor.remaining() < 8 || cursor.pos() >= content_end {
                    break;
                }
                let album = parse_album_item(cursor)?;
                library.albums.push(album);
            }
            let end_idx = library.albums.len();
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::AlbumList {
                    raw_header: master_header,
                    range: start_idx..end_idx,
                },
            });
        }
        // mlih artists
        11 => {
            let master_header = parse_master_header(cursor, b"mlih")?;
            let count = u32::from_le_bytes(
                master_header[8..12].try_into().unwrap()
            );
            let start_idx = library.artists.len();
            for _ in 0..count {
                if cursor.remaining() < 8 || cursor.pos() >= content_end {
                    break;
                }
                let artist = parse_artist_item(cursor)?;
                library.artists.push(artist);
            }
            let end_idx = library.artists.len();
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::ArtistList {
                    raw_header: master_header,
                    range: start_idx..end_idx,
                },
            });
        }
        // mlth tracks (subtypes 1 and 13)
        1 | 13 => {
            let master_header = parse_master_header(cursor, b"mlth")?;
            let count = u32::from_le_bytes(
                master_header[8..12].try_into().unwrap()
            );
            let start_idx = library.tracks.len();
            for _ in 0..count {
                if cursor.remaining() < 8 || cursor.pos() >= content_end {
                    break;
                }
                let track = parse_track_item(cursor)?;
                library.tracks.push(track);
            }
            let end_idx = library.tracks.len();
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::TrackList {
                    raw_header: master_header,
                    range: start_idx..end_idx,
                },
            });
        }
        // mlph playlists (subtypes 2 and 14)
        2 | 14 => {
            let master_header = parse_master_header(cursor, b"mlph")?;
            let start_idx = library.playlists.len();
            parse_playlists(cursor, content_end, library)?;
            let end_idx = library.playlists.len();
            library.section_order.push(SectionRef::Msdh {
                raw_header: msdh_header_bytes,
                subtype,
                content: MsdhContent::PlaylistList {
                    raw_header: master_header,
                    range: start_idx..end_idx,
                },
            });
        }
        _ => {
            // Skip unknown msdh content
            let skip = content_end.saturating_sub(cursor.pos());
            if skip <= cursor.remaining() {
                let blob = cursor.read_bytes(skip)?.to_vec();
                library.section_order.push(SectionRef::Msdh {
                    raw_header: msdh_header_bytes,
                    subtype,
                    content: MsdhContent::Unknown(blob),
                });
            }
        }
    }

    // Ensure cursor is at content_end
    if cursor.pos() < content_end && content_end <= cursor.data.len() {
        cursor.set_pos(content_end);
    }

    Ok(())
}

fn parse_mfdh(cursor: &mut Cursor) -> Result<Vec<u8>> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"mfdh" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected mfdh, got {:?}", sig),
        });
    }
    let section_length = cursor.read_u32_le()? as usize;
    let remaining = section_length.saturating_sub(8);
    cursor.skip(remaining)?;
    Ok(cursor.slice_from(start).to_vec())
}

fn parse_mhgh(cursor: &mut Cursor) -> Result<LibraryInfo> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"mhgh" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected mhgh, got {:?}", sig),
        });
    }
    let section_length = cursor.read_u32_le()? as usize;
    let mhoh_count = cursor.read_u32_le()?;

    let skip = section_length.saturating_sub(12);
    cursor.skip(skip)?;
    let raw_header = cursor.slice_from(start).to_vec();

    let mut data_fields = Vec::new();
    for _ in 0..mhoh_count {
        if cursor.remaining() < 8 {
            break;
        }
        if let Ok(field) = parse_mhoh(cursor) {
            data_fields.push(field);
        } else {
            break;
        }
    }

    Ok(LibraryInfo {
        raw_header,
        data_fields,
    })
}

fn parse_master_header(cursor: &mut Cursor, expected_sig: &[u8; 4]) -> Result<Vec<u8>> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != expected_sig {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected {:?}, got {:?}", expected_sig, sig),
        });
    }
    let section_length = cursor.read_u32_le()? as usize;
    let remaining = section_length.saturating_sub(8);
    cursor.skip(remaining)?;
    Ok(cursor.slice_from(start).to_vec())
}

pub(crate) fn parse_track_item(cursor: &mut Cursor) -> Result<Track> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"mith" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected mith, got {:?}", sig),
        });
    }

    let section_length = cursor.read_u32_le()? as usize;
    let assoc_length = cursor.read_u32_le()? as usize;
    let mhoh_count = cursor.read_u32_le()?;

    let header_remaining = section_length.saturating_sub(16);
    cursor.skip(header_remaining)?;
    let raw_header = cursor.data[start + 8..start + section_length].to_vec();

    let item_end = start + assoc_length;
    let mut data_fields = Vec::new();
    for _ in 0..mhoh_count {
        if cursor.remaining() < 8 || cursor.pos() >= item_end {
            break;
        }
        match parse_mhoh(cursor) {
            Ok(field) => data_fields.push(field),
            Err(_) => break,
        }
    }

    if cursor.pos() < item_end && item_end <= cursor.data.len() {
        cursor.set_pos(item_end);
    }

    Ok(Track {
        raw_header,
        data_fields,
    })
}

pub(crate) fn parse_album_item(cursor: &mut Cursor) -> Result<Album> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"miah" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected miah, got {:?}", sig),
        });
    }

    let section_length = cursor.read_u32_le()? as usize;
    let assoc_length = cursor.read_u32_le()? as usize;
    let mhoh_count = cursor.read_u32_le()?;

    let header_remaining = section_length.saturating_sub(16);
    cursor.skip(header_remaining)?;
    let raw_header = cursor.data[start + 8..start + section_length].to_vec();

    let item_end = start + assoc_length;
    let mut data_fields = Vec::new();
    for _ in 0..mhoh_count {
        if cursor.remaining() < 8 || cursor.pos() >= item_end {
            break;
        }
        match parse_mhoh(cursor) {
            Ok(field) => data_fields.push(field),
            Err(_) => break,
        }
    }

    if cursor.pos() < item_end && item_end <= cursor.data.len() {
        cursor.set_pos(item_end);
    }

    Ok(Album {
        raw_header,
        data_fields,
    })
}

pub(crate) fn parse_artist_item(cursor: &mut Cursor) -> Result<Artist> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"miih" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected miih, got {:?}", sig),
        });
    }

    let section_length = cursor.read_u32_le()? as usize;
    let assoc_length = cursor.read_u32_le()? as usize;
    let mhoh_count = cursor.read_u32_le()?;

    let header_remaining = section_length.saturating_sub(16);
    cursor.skip(header_remaining)?;
    let raw_header = cursor.data[start + 8..start + section_length].to_vec();

    let item_end = start + assoc_length;
    let mut data_fields = Vec::new();
    for _ in 0..mhoh_count {
        if cursor.remaining() < 8 || cursor.pos() >= item_end {
            break;
        }
        match parse_mhoh(cursor) {
            Ok(field) => data_fields.push(field),
            Err(_) => break,
        }
    }

    if cursor.pos() < item_end && item_end <= cursor.data.len() {
        cursor.set_pos(item_end);
    }

    Ok(Artist {
        raw_header,
        data_fields,
    })
}

fn parse_playlists(cursor: &mut Cursor, section_end: usize, library: &mut ParsedLibrary) -> Result<()> {
    let mut current_playlist: Option<Playlist> = None;

    while cursor.remaining() >= 8 && cursor.pos() < section_end {
        let peek = cursor.peek_bytes(4)?;
        match peek {
            b"miph" => {
                if let Some(pl) = current_playlist.take() {
                    library.playlists.push(pl);
                }
                current_playlist = Some(parse_playlist_header(cursor)?);
            }
            b"mtph" => {
                let track_id = parse_playlist_track(cursor)?;
                if let Some(ref mut pl) = current_playlist {
                    pl.track_ids.push(track_id);
                }
            }
            b"mhoh" => {
                if let Ok(field) = parse_mhoh(cursor)
                    && let Some(ref mut pl) = current_playlist
                {
                    pl.data_fields.push(field);
                }
            }
            _ => {
                // Unknown section inside playlist area — skip by length
                cursor.skip(4)?;
                if cursor.remaining() >= 4 {
                    let len = cursor.read_u32_le()? as usize;
                    let skip = len.saturating_sub(8);
                    if skip <= cursor.remaining() {
                        cursor.skip(skip)?;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    if let Some(pl) = current_playlist.take() {
        library.playlists.push(pl);
    }

    Ok(())
}

fn parse_playlist_header(cursor: &mut Cursor) -> Result<Playlist> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"miph" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected miph, got {:?}", sig),
        });
    }

    let section_length = cursor.read_u32_le()? as usize;
    let header_remaining = section_length.saturating_sub(8);
    cursor.skip(header_remaining)?;
    let raw_header = cursor.data[start + 8..start + section_length].to_vec();

    Ok(Playlist {
        raw_header,
        data_fields: Vec::new(),
        track_ids: Vec::new(),
    })
}

pub(crate) fn parse_playlist_track(cursor: &mut Cursor) -> Result<u32> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"mtph" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected mtph, got {:?}", sig),
        });
    }

    let section_length = cursor.read_u32_le()? as usize;

    // Track reference key is at offset 28 relative to section start (offset 20 in remaining data)
    let data_start = cursor.pos();
    let remaining_data = section_length.saturating_sub(8);
    if remaining_data >= 20 {
        cursor.skip(16)?;
        let key = cursor.read_u32_le()?;
        let leftover = remaining_data.saturating_sub(20);
        cursor.skip(leftover)?;
        Ok(key)
    } else {
        cursor.set_pos(data_start + remaining_data);
        Ok(0)
    }
}

pub(crate) fn parse_mhoh(cursor: &mut Cursor) -> Result<DataField> {
    let start = cursor.pos();
    let sig = cursor.read_sig()?;
    if &sig != b"mhoh" {
        return Err(ItlError::Parse {
            offset: start,
            message: format!("expected mhoh, got {:?}", sig),
        });
    }

    let _dummy = cursor.read_u32_le()?; // always 24
    let total_length = cursor.read_u32_le()? as usize;
    let subtype = cursor.read_u32_le()?;

    // Read rest of the 24-byte common header (8 bytes remain)
    let _unknown = cursor.read_bytes(8)?;

    let raw_header = cursor.data[start..start + 24].to_vec();

    let data_size = total_length.saturating_sub(24);

    if DataFieldType::is_raw_data_type(subtype) {
        let data = if data_size > 0 && data_size <= cursor.remaining() {
            cursor.read_bytes(data_size)?.to_vec()
        } else if data_size > 0 {
            let available = cursor.remaining().min(data_size);
            cursor.read_bytes(available)?.to_vec()
        } else {
            Vec::new()
        };
        return Ok(DataField {
            raw_header,
            subtype,
            content: DataContent::RawData(data),
        });
    }

    // Flex character container: string header at offsets 24-39, string at 40+
    if data_size >= 16 {
        let string_type_val = cursor.read_u32_le()?;
        let string_length = cursor.read_u32_le()? as usize;
        let _padding = cursor.read_bytes(8)?;

        let actual_string_size = data_size.saturating_sub(16);
        let read_size = actual_string_size.min(string_length).min(cursor.remaining());

        let string_bytes = cursor.read_bytes(read_size)?;

        // Skip any trailing bytes
        let trailing = actual_string_size.saturating_sub(read_size);
        if trailing > 0 && trailing <= cursor.remaining() {
            cursor.skip(trailing)?;
        }

        let encoding = StringEncoding::try_from(string_type_val).unwrap_or(StringEncoding::Utf8);
        let value = decode_string(encoding, string_bytes);

        Ok(DataField {
            raw_header,
            subtype,
            content: DataContent::String { encoding, value },
        })
    } else {
        let data = if data_size > 0 && data_size <= cursor.remaining() {
            cursor.read_bytes(data_size)?.to_vec()
        } else {
            Vec::new()
        };
        Ok(DataField {
            raw_header,
            subtype,
            content: DataContent::RawData(data),
        })
    }
}

fn decode_string(encoding: StringEncoding, bytes: &[u8]) -> String {
    match encoding {
        StringEncoding::Utf8 | StringEncoding::Uri | StringEncoding::EscapedUri => {
            String::from_utf8_lossy(bytes).into_owned()
        }
        StringEncoding::Utf16 => {
            if bytes.len() >= 2 {
                let u16s: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&u16s)
            } else {
                String::from_utf8_lossy(bytes).into_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::types::*;
    use crate::ItlError;

    fn build_mhoh_flex(subtype: u32, encoding: u32, string_bytes: &[u8]) -> Vec<u8> {
        let total_length: u32 = 24 + 16 + string_bytes.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&total_length.to_le_bytes());
        buf.extend_from_slice(&subtype.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(&encoding.to_le_bytes());
        buf.extend_from_slice(&(string_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(string_bytes);
        buf
    }

    fn build_mhoh_raw(subtype: u32, data: &[u8]) -> Vec<u8> {
        let total_length: u32 = 24 + data.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&total_length.to_le_bytes());
        buf.extend_from_slice(&subtype.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(data);
        buf
    }

    fn build_item_section(sig: &[u8; 4], extra_header: &[u8], mhohs: &[Vec<u8>]) -> Vec<u8> {
        let section_length: u32 = 16 + extra_header.len() as u32;
        let mhoh_data: Vec<u8> = mhohs.concat();
        let assoc_length: u32 = section_length + mhoh_data.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(sig);
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&(mhohs.len() as u32).to_le_bytes());
        buf.extend_from_slice(extra_header);
        buf.extend_from_slice(&mhoh_data);
        buf
    }

    fn build_master_section(sig: &[u8; 4], count: u32) -> Vec<u8> {
        let section_length: u32 = 92;
        let mut buf = Vec::new();
        buf.extend_from_slice(sig);
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 12]);
        buf
    }

    fn build_msdh(subtype: u32, content: &[u8]) -> Vec<u8> {
        let section_length: u32 = 96;
        let assoc_length: u32 = section_length + content.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"msdh");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&subtype.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 16]);
        buf.extend_from_slice(content);
        buf
    }

    fn build_mfdh() -> Vec<u8> {
        let section_length: u32 = 144;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mfdh");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 8]);
        buf
    }

    fn build_mhgh(mhohs: &[Vec<u8>]) -> Vec<u8> {
        let section_length: u32 = 280;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhgh");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&(mhohs.len() as u32).to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 12]);
        for mhoh in mhohs {
            buf.extend_from_slice(mhoh);
        }
        buf
    }

    fn build_miph() -> Vec<u8> {
        let section_length: u32 = 48;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"miph");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 8]);
        buf
    }

    fn build_mtph(track_id: u32) -> Vec<u8> {
        let section_length: u32 = 36;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mtph");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&[0u8; 16]);
        buf.extend_from_slice(&track_id.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        buf
    }

    fn build_mtph_short() -> Vec<u8> {
        let section_length: u32 = 18;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mtph");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&[0u8; 10]);
        buf
    }

    #[test]
    fn test_cursor_new() {
        let data = [1u8, 2, 3];
        let c = Cursor::new(&data);
        assert_eq!(c.pos(), 0);
        assert_eq!(c.remaining(), 3);
    }

    #[test]
    fn test_cursor_read_bytes() {
        let data = [1u8, 2, 3, 4, 5];
        let mut c = Cursor::new(&data);
        let a = c.read_bytes(2).unwrap();
        assert_eq!(a, &[1, 2]);
        assert_eq!(c.pos(), 2);
        assert_eq!(c.remaining(), 3);
    }

    #[test]
    fn test_cursor_read_bytes_eof() {
        let data = [1u8, 2];
        let mut c = Cursor::new(&data);
        let err = c.read_bytes(5).unwrap_err();
        assert!(matches!(err, ItlError::UnexpectedEof(0)));
    }

    #[test]
    fn test_cursor_peek_bytes() {
        let data = [9u8, 8, 7];
        let mut c = Cursor::new(&data);
        assert_eq!(c.peek_bytes(2).unwrap(), &[9, 8]);
        assert_eq!(c.pos(), 0);
        c.read_bytes(1).unwrap();
        assert_eq!(c.peek_bytes(1).unwrap(), &[8]);
        assert_eq!(c.pos(), 1);
    }

    #[test]
    fn test_cursor_peek_bytes_eof() {
        let data = [1u8];
        let c = Cursor::new(&data);
        let err = c.peek_bytes(4).unwrap_err();
        assert!(matches!(err, ItlError::UnexpectedEof(0)));
    }

    #[test]
    fn test_cursor_read_sig() {
        let data = *b"mithXXXX";
        let mut c = Cursor::new(&data);
        assert_eq!(c.read_sig().unwrap(), *b"mith");
        assert_eq!(c.pos(), 4);
    }

    #[test]
    fn test_cursor_read_u8() {
        let data = [0xABu8];
        let mut c = Cursor::new(&data);
        assert_eq!(c.read_u8().unwrap(), 0xAB);
        assert_eq!(c.pos(), 1);
    }

    #[test]
    fn test_cursor_read_u16_le() {
        let data = [0x34u8, 0x12];
        let mut c = Cursor::new(&data);
        assert_eq!(c.read_u16_le().unwrap(), 0x1234);
    }

    #[test]
    fn test_cursor_read_u32_le() {
        let data = [0x78u8, 0x56, 0x34, 0x12];
        let mut c = Cursor::new(&data);
        assert_eq!(c.read_u32_le().unwrap(), 0x12345678);
    }

    #[test]
    fn test_cursor_read_u64_le() {
        let data = [0xEFu8, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01];
        let mut c = Cursor::new(&data);
        assert_eq!(c.read_u64_le().unwrap(), 0x0123456789ABCDEF);
    }

    #[test]
    fn test_cursor_skip() {
        let data = [0u8; 10];
        let mut c = Cursor::new(&data);
        c.skip(3).unwrap();
        assert_eq!(c.pos(), 3);
        c.skip(7).unwrap();
        assert_eq!(c.pos(), 10);
    }

    #[test]
    fn test_cursor_skip_eof() {
        let data = [1u8, 2];
        let mut c = Cursor::new(&data);
        let err = c.skip(5).unwrap_err();
        assert!(matches!(err, ItlError::UnexpectedEof(0)));
    }

    #[test]
    fn test_cursor_set_pos() {
        let data = [0u8; 5];
        let mut c = Cursor::new(&data);
        c.set_pos(3);
        assert_eq!(c.pos(), 3);
        assert_eq!(c.remaining(), 2);
    }

    #[test]
    fn test_cursor_slice_from() {
        let data = [1u8, 2, 3, 4, 5];
        let mut c = Cursor::new(&data);
        c.read_bytes(2).unwrap();
        assert_eq!(c.slice_from(0), &[1, 2]);
        c.read_bytes(2).unwrap();
        assert_eq!(c.slice_from(1), &[2, 3, 4]);
    }

    #[test]
    fn test_parse_mhoh_flex_utf8() {
        let buf = build_mhoh_flex(
            DataFieldType::TrackTitle as u32,
            StringEncoding::Utf8 as u32,
            b"Test Song",
        );
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert_eq!(field.subtype, DataFieldType::TrackTitle as u32);
        assert!(matches!(
            field.content,
            DataContent::String {
                encoding: StringEncoding::Utf8,
                ref value
            } if value == "Test Song"
        ));
    }

    #[test]
    fn test_parse_mhoh_flex_utf16() {
        let utf16: Vec<u8> = "Hello"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let buf = build_mhoh_flex(
            DataFieldType::TrackTitle as u32,
            StringEncoding::Utf16 as u32,
            &utf16,
        );
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert!(matches!(
            field.content,
            DataContent::String {
                encoding: StringEncoding::Utf16,
                ref value
            } if value == "Hello"
        ));
    }

    #[test]
    fn test_parse_mhoh_flex_uri() {
        let buf = build_mhoh_flex(
            DataFieldType::TrackTitle as u32,
            StringEncoding::Uri as u32,
            b"https://example.com/track",
        );
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert!(matches!(
            field.content,
            DataContent::String {
                encoding: StringEncoding::Uri,
                ref value
            } if value == "https://example.com/track"
        ));
    }

    #[test]
    fn test_parse_mhoh_raw_data_type() {
        let payload = b"<art>xml</art>";
        let buf = build_mhoh_raw(DataFieldType::ArtXmlBlock as u32, payload);
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert_eq!(field.subtype, DataFieldType::ArtXmlBlock as u32);
        assert!(matches!(field.content, DataContent::RawData(ref b) if b == payload));
    }

    #[test]
    fn test_parse_mhoh_small_data() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&(DataFieldType::TrackTitle as u32).to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert!(matches!(field.content, DataContent::RawData(ref d) if d.is_empty()));
    }

    #[test]
    fn test_parse_mhoh_wrong_sig() {
        let buf = build_mhoh_flex(2, 3, b"x");
        let mut buf = buf;
        buf[0..4].copy_from_slice(b"mith");
        let mut c = Cursor::new(&buf);
        let err = super::parse_mhoh(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_mhoh_raw_zero_size() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&(DataFieldType::ArtXmlBlock as u32).to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert!(matches!(field.content, DataContent::RawData(ref d) if d.is_empty()));
    }

    #[test]
    fn test_decode_string_utf8() {
        let s = super::decode_string(StringEncoding::Utf8, b"hello \xC3\xBC");
        assert_eq!(s, "hello ü");
    }

    #[test]
    fn test_decode_string_utf16() {
        let bytes = [0x41u8, 0x00, 0x42u8, 0x00];
        let s = super::decode_string(StringEncoding::Utf16, &bytes);
        assert_eq!(s, "AB");
    }

    #[test]
    fn test_decode_string_utf16_single_byte() {
        let s = super::decode_string(StringEncoding::Utf16, &[0x41u8]);
        assert_eq!(s, "A");
    }

    #[test]
    fn test_decode_string_uri() {
        let s = super::decode_string(StringEncoding::Uri, b"file:///music/a.flac");
        assert_eq!(s, "file:///music/a.flac");
    }

    #[test]
    fn test_decode_string_escaped_uri() {
        let s = super::decode_string(StringEncoding::EscapedUri, b"path%20here");
        assert_eq!(s, "path%20here");
    }

    #[test]
    fn test_parse_track_item() {
        let tid = 42u32.to_le_bytes();
        let mhoh = build_mhoh_flex(
            DataFieldType::TrackTitle as u32,
            StringEncoding::Utf8 as u32,
            b"Test Song",
        );
        let item = build_item_section(b"mith", &tid, &[mhoh]);
        let mut c = Cursor::new(&item);
        let track = super::parse_track_item(&mut c).unwrap();
        assert_eq!(track.id(), 42);
        assert_eq!(track.title(), Some("Test Song"));
    }

    #[test]
    fn test_parse_track_item_wrong_sig() {
        let tid = 1u32.to_le_bytes();
        let item = build_item_section(b"miah", &tid, &[]);
        let mut c = Cursor::new(&item);
        let err = super::parse_track_item(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_album_item() {
        let extra = [0u8; 24];
        let mhoh = build_mhoh_flex(
            DataFieldType::AlbumItemName as u32,
            StringEncoding::Utf8 as u32,
            b"My Album",
        );
        let item = build_item_section(b"miah", &extra, &[mhoh]);
        let mut c = Cursor::new(&item);
        let album = super::parse_album_item(&mut c).unwrap();
        assert_eq!(album.name(), Some("My Album"));
    }

    #[test]
    fn test_parse_album_item_wrong_sig() {
        let extra = [0u8; 4];
        let item = build_item_section(b"mith", &extra, &[]);
        let mut c = Cursor::new(&item);
        let err = super::parse_album_item(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_artist_item() {
        let extra = [0u8; 12];
        let mhoh = build_mhoh_flex(
            DataFieldType::ArtistName as u32,
            StringEncoding::Utf8 as u32,
            b"Artist One",
        );
        let item = build_item_section(b"miih", &extra, &[mhoh]);
        let mut c = Cursor::new(&item);
        let artist = super::parse_artist_item(&mut c).unwrap();
        assert_eq!(artist.name(), Some("Artist One"));
    }

    #[test]
    fn test_parse_artist_item_wrong_sig() {
        let extra = [0u8; 4];
        let item = build_item_section(b"miah", &extra, &[]);
        let mut c = Cursor::new(&item);
        let err = super::parse_artist_item(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_playlist_header() {
        let buf = build_miph();
        let mut c = Cursor::new(&buf);
        let pl = super::parse_playlist_header(&mut c).unwrap();
        assert_eq!(pl.track_ids.len(), 0);
        assert!(pl.data_fields.is_empty());
    }

    #[test]
    fn test_parse_playlist_header_wrong_sig() {
        let mut buf = build_miph();
        buf[0..4].copy_from_slice(b"xxxx");
        let mut c = Cursor::new(&buf);
        let err = super::parse_playlist_header(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_playlist_track() {
        let buf = build_mtph(99);
        let mut c = Cursor::new(&buf);
        let id = super::parse_playlist_track(&mut c).unwrap();
        assert_eq!(id, 99);

        let short = build_mtph_short();
        let mut c2 = Cursor::new(&short);
        let id0 = super::parse_playlist_track(&mut c2).unwrap();
        assert_eq!(id0, 0);
    }

    #[test]
    fn test_parse_playlist_track_wrong_sig() {
        let mut buf = build_mtph(1);
        buf[0..4].copy_from_slice(b"miph");
        let mut c = Cursor::new(&buf);
        let err = super::parse_playlist_track(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_mfdh() {
        let buf = build_mfdh();
        let mut c = Cursor::new(&buf);
        let raw = super::parse_mfdh(&mut c).unwrap();
        assert!(raw.starts_with(b"mfdh"));
        assert_eq!(raw.len(), 144);
    }

    #[test]
    fn test_parse_mfdh_wrong_sig() {
        let mut buf = build_mfdh();
        buf[0..4].copy_from_slice(b"mfxx");
        let mut c = Cursor::new(&buf);
        let err = super::parse_mfdh(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_mhgh() {
        let share = build_mhoh_flex(
            DataFieldType::LibraryShareName as u32,
            StringEncoding::Utf8 as u32,
            b"Shared Library",
        );
        let buf = build_mhgh(&[share]);
        let mut c = Cursor::new(&buf);
        let info = super::parse_mhgh(&mut c).unwrap();
        assert_eq!(info.share_name(), Some("Shared Library"));
        assert_eq!(info.data_fields.len(), 1);
    }

    #[test]
    fn test_parse_mhgh_wrong_sig() {
        let buf = build_mhgh(&[]);
        let mut buf = buf;
        buf[0..4].copy_from_slice(b"mxxx");
        let mut c = Cursor::new(&buf);
        let err = super::parse_mhgh(&mut c).unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_master_header() {
        let buf = build_master_section(b"mlth", 7);
        let mut c = Cursor::new(&buf);
        let hdr = super::parse_master_header(&mut c, b"mlth").unwrap();
        assert_eq!(&hdr[0..4], b"mlth");
        assert_eq!(u32::from_le_bytes(hdr[8..12].try_into().unwrap()), 7);
    }

    #[test]
    fn test_parse_master_header_wrong_sig() {
        let buf = build_master_section(b"mlth", 1);
        let mut c = Cursor::new(&buf);
        let err = super::parse_master_header(&mut c, b"mlah").unwrap_err();
        assert!(matches!(err, ItlError::Parse { .. }));
    }

    #[test]
    fn test_parse_inner_empty() {
        let lib = parse_inner(&[]).unwrap();
        assert!(lib.tracks.is_empty());
        assert!(lib.albums.is_empty());
        assert!(lib.artists.is_empty());
        assert!(lib.playlists.is_empty());
        assert!(lib.raw_sections.is_empty());
        assert!(lib.inner_header.is_none());
        assert!(lib.library_info.is_none());
    }

    #[test]
    fn test_parse_inner_with_tracks() {
        let title = build_mhoh_flex(
            DataFieldType::TrackTitle as u32,
            StringEncoding::Utf8 as u32,
            b"Lonely Track",
        );
        let tid = 77u32.to_le_bytes();
        let mith = build_item_section(b"mith", &tid, &[title]);
        let mlth = build_master_section(b"mlth", 1);
        let inner = [mlth.as_slice(), mith.as_slice()].concat();
        let blob = [build_msdh(16, &build_mfdh()), build_msdh(1, &inner)].concat();
        let lib = parse_inner(&blob).unwrap();
        assert_eq!(lib.tracks.len(), 1);
        assert_eq!(lib.tracks[0].id(), 77);
        assert_eq!(lib.tracks[0].title(), Some("Lonely Track"));
    }

    #[test]
    fn test_parse_inner_with_albums() {
        let mhoh = build_mhoh_flex(
            DataFieldType::AlbumItemName as u32,
            StringEncoding::Utf8 as u32,
            b"Album X",
        );
        let extra = [0u8; 32];
        let miah = build_item_section(b"miah", &extra, &[mhoh]);
        let mlah = build_master_section(b"mlah", 1);
        let inner = [mlah.as_slice(), miah.as_slice()].concat();
        let blob = build_msdh(9, &inner);
        let lib = parse_inner(&blob).unwrap();
        assert_eq!(lib.albums.len(), 1);
        assert_eq!(lib.albums[0].name(), Some("Album X"));
    }

    #[test]
    fn test_parse_inner_with_artists() {
        let mhoh = build_mhoh_flex(
            DataFieldType::ArtistName as u32,
            StringEncoding::Utf8 as u32,
            b"Singer",
        );
        let extra = [0u8; 12];
        let miih = build_item_section(b"miih", &extra, &[mhoh]);
        let mlih = build_master_section(b"mlih", 1);
        let inner = [mlih.as_slice(), miih.as_slice()].concat();
        let blob = build_msdh(11, &inner);
        let lib = parse_inner(&blob).unwrap();
        assert_eq!(lib.artists.len(), 1);
        assert_eq!(lib.artists[0].name(), Some("Singer"));
    }

    #[test]
    fn test_parse_inner_raw_data_subtype() {
        let payload = vec![0xDEu8, 0xAD, 0xBE, 0xEF];
        let blob = build_msdh(4, &payload);
        let lib = parse_inner(&blob).unwrap();
        let found = lib.section_order.iter().any(|s| {
            matches!(
                s,
                SectionRef::Msdh {
                    content: MsdhContent::RawBlob(b),
                    ..
                } if b == &payload
            )
        });
        assert!(found, "expected RawBlob msdh subtype 4");
    }

    #[test]
    fn test_parse_inner_unknown_msdh_subtype() {
        let payload = vec![1u8, 2, 3, 4, 5];
        let blob = build_msdh(99, &payload);
        let lib = parse_inner(&blob).unwrap();
        let found = lib.section_order.iter().any(|s| {
            matches!(
                s,
                SectionRef::Msdh {
                    content: MsdhContent::Unknown(b),
                    subtype: 99,
                    ..
                } if b == &payload
            )
        });
        assert!(found);
    }

    #[test]
    fn test_parse_inner_unknown_toplevel_section() {
        let mut junk = Vec::new();
        junk.extend_from_slice(b"zzzz");
        junk.extend_from_slice(&16u32.to_le_bytes());
        junk.extend_from_slice(&[0u8; 8]);
        let lib = parse_inner(&junk).unwrap();
        assert_eq!(lib.raw_sections.len(), 1);
        assert_eq!(lib.raw_sections[0].sig, *b"zzzz");
        assert_eq!(lib.raw_sections[0].data.len(), 16);
    }

    #[test]
    fn test_msdh_is_raw_data_subtype() {
        for st in [3u32, 4, 19, 22] {
            assert!(super::msdh_is_raw_data_subtype(st), "raw {st}");
        }
        for st in [1u32, 9, 11, 12, 16] {
            assert!(!super::msdh_is_raw_data_subtype(st), "not raw {st}");
        }
    }

    #[test]
    fn test_parse_playlists() {
        let title = build_mhoh_flex(
            DataFieldType::PlaylistTitle as u32,
            StringEncoding::Utf8 as u32,
            b"My Playlist",
        );
        let playlist_blob = [
            build_miph().as_slice(),
            title.as_slice(),
            build_mtph(1001).as_slice(),
            build_mtph(1002).as_slice(),
        ]
        .concat();
        let mlth = build_master_section(b"mlph", 1);
        let inner = [mlth.as_slice(), playlist_blob.as_slice()].concat();
        let blob = build_msdh(2, &inner);
        let lib = parse_inner(&blob).unwrap();
        assert_eq!(lib.playlists.len(), 1);
        assert_eq!(lib.playlists[0].title(), Some("My Playlist"));
        assert_eq!(lib.playlists[0].track_ids(), &[1001, 1002]);

        let mut c = Cursor::new(&playlist_blob);
        let mut manual = ParsedLibrary {
            inner_header: None,
            library_info: None,
            tracks: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            playlists: Vec::new(),
            raw_sections: Vec::new(),
            section_order: Vec::new(),
        };
        let end = c.data.len();
        super::parse_playlists(&mut c, end, &mut manual).unwrap();
        assert_eq!(manual.playlists.len(), 1);
        assert_eq!(manual.playlists[0].title(), Some("My Playlist"));
        assert_eq!(manual.playlists[0].track_ids(), &[1001, 1002]);
    }

    #[test]
    fn test_parse_inner_with_library_info() {
        let share_name_mhoh =
            build_mhoh_flex(DataFieldType::LibraryShareName as u32, 3, b"My Share");
        let mhgh = build_mhgh(&[share_name_mhoh]);
        let blob = build_msdh(12, &mhgh);
        let lib = parse_inner(&blob).unwrap();
        assert!(lib.library_info.is_some());
        assert_eq!(lib.library_info.as_ref().unwrap().share_name(), Some("My Share"));
    }

    #[test]
    fn test_parse_msdh_raw_data_empty_blob() {
        // subtype 3 is a raw data type; assoc_length = section_length so blob_size = 0
        let section_length: u32 = 96;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"msdh");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&section_length.to_le_bytes()); // assoc_length == section_length
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 16]);
        let lib = parse_inner(&buf).unwrap();
        assert_eq!(lib.section_order.len(), 1);
        match &lib.section_order[0] {
            SectionRef::Msdh { content: MsdhContent::RawBlob(blob), .. } => {
                assert!(blob.is_empty());
            }
            _ => panic!("expected RawBlob"),
        }
    }

    #[test]
    fn test_parse_msdh_cursor_realignment() {
        // Construct an msdh whose content_end extends past what inner parsing consumes.
        // This tests the cursor.set_pos(content_end) at the end of parse_msdh.
        let mfdh = build_mfdh();
        let extra = vec![0u8; 20];
        // Build msdh with subtype 16 (inner header), but add extra trailing bytes
        let section_length: u32 = 96;
        let assoc_length: u32 = section_length + mfdh.len() as u32 + extra.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"msdh");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&vec![0u8; section_length as usize - 16]);
        buf.extend_from_slice(&mfdh);
        buf.extend_from_slice(&extra);
        let lib = parse_inner(&buf).unwrap();
        assert!(lib.inner_header.is_some());
    }

    #[test]
    fn test_parse_track_item_cursor_realignment() {
        // Track item where item_end > cursor.pos after mhohs
        // Build mith with extra trailing bytes after the mhoh
        let mhoh = build_mhoh_flex(0x0002, 3, b"Song");
        let extra_header = vec![0u8; 184];
        let section_length: u32 = 16 + extra_header.len() as u32;
        let mhoh_data_len = mhoh.len() as u32;
        let trailing = 10u32;
        let assoc_length: u32 = section_length + mhoh_data_len + trailing;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mith");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // mhoh_count = 1
        buf.extend_from_slice(&extra_header);
        buf.extend_from_slice(&mhoh);
        buf.extend_from_slice(&vec![0u8; trailing as usize]);

        let mut c = Cursor::new(&buf);
        let track = super::parse_track_item(&mut c).unwrap();
        assert_eq!(track.data_fields.len(), 1);
        assert_eq!(c.pos(), buf.len());
    }

    #[test]
    fn test_parse_album_item_cursor_realignment() {
        let mhoh = build_mhoh_flex(DataFieldType::AlbumItemName as u32, 3, b"Album");
        let extra_header = vec![0u8; 80];
        let section_length: u32 = 16 + extra_header.len() as u32;
        let trailing = 8u32;
        let assoc_length: u32 = section_length + mhoh.len() as u32 + trailing;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"miah");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&extra_header);
        buf.extend_from_slice(&mhoh);
        buf.extend_from_slice(&vec![0u8; trailing as usize]);

        let mut c = Cursor::new(&buf);
        let album = super::parse_album_item(&mut c).unwrap();
        assert_eq!(album.data_fields.len(), 1);
        assert_eq!(c.pos(), buf.len());
    }

    #[test]
    fn test_parse_artist_item_cursor_realignment() {
        let mhoh = build_mhoh_flex(DataFieldType::ArtistName as u32, 3, b"Artist");
        let extra_header = vec![0u8; 80];
        let section_length: u32 = 16 + extra_header.len() as u32;
        let trailing = 6u32;
        let assoc_length: u32 = section_length + mhoh.len() as u32 + trailing;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"miih");
        buf.extend_from_slice(&section_length.to_le_bytes());
        buf.extend_from_slice(&assoc_length.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&extra_header);
        buf.extend_from_slice(&mhoh);
        buf.extend_from_slice(&vec![0u8; trailing as usize]);

        let mut c = Cursor::new(&buf);
        let artist = super::parse_artist_item(&mut c).unwrap();
        assert_eq!(artist.data_fields.len(), 1);
        assert_eq!(c.pos(), buf.len());
    }

    #[test]
    fn test_parse_playlists_multiple_with_unknown_section() {
        // Two playlists and an unknown section between them
        let miph1 = build_miph();
        let mtph1 = build_mtph(100);
        let miph2 = build_miph();
        let mtph2 = build_mtph(200);

        // Unknown section: sig + length + data
        let mut unknown = Vec::new();
        unknown.extend_from_slice(b"xyzh");
        unknown.extend_from_slice(&20u32.to_le_bytes());
        unknown.extend_from_slice(&[0u8; 12]); // 20 - 8 = 12 bytes

        let blob: Vec<u8> = [
            miph1.as_slice(),
            mtph1.as_slice(),
            unknown.as_slice(),
            miph2.as_slice(),
            mtph2.as_slice(),
        ]
        .concat();

        let mut c = Cursor::new(&blob);
        let mut lib = ParsedLibrary {
            inner_header: None,
            library_info: None,
            tracks: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            playlists: Vec::new(),
            raw_sections: Vec::new(),
            section_order: Vec::new(),
        };
        super::parse_playlists(&mut c, blob.len(), &mut lib).unwrap();
        assert_eq!(lib.playlists.len(), 2);
        assert_eq!(lib.playlists[0].track_ids(), &[100]);
        assert_eq!(lib.playlists[1].track_ids(), &[200]);
    }

    #[test]
    fn test_parse_mhoh_raw_data_truncated() {
        // Raw data type where data_size > cursor.remaining()
        let subtype = 0x0036u32; // ArtXmlBlock, a raw data type
        let total_length: u32 = 24 + 100; // claims 100 bytes of data
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&total_length.to_le_bytes());
        buf.extend_from_slice(&subtype.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        // Only provide 50 bytes instead of 100
        buf.extend_from_slice(&[0xABu8; 50]);

        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        match &field.content {
            DataContent::RawData(d) => assert_eq!(d.len(), 50),
            _ => panic!("expected RawData"),
        }
    }

    #[test]
    fn test_parse_mhoh_flex_with_trailing_bytes() {
        // Flex string mhoh where actual_string_size > string_length (trailing bytes to skip)
        let string_bytes = b"Hello";
        let trailing = 10usize;
        let data_size = 16 + string_bytes.len() + trailing;
        let total_length = 24 + data_size;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&(total_length as u32).to_le_bytes());
        buf.extend_from_slice(&0x0002u32.to_le_bytes()); // TrackTitle
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(&3u32.to_le_bytes()); // UTF-8
        buf.extend_from_slice(&(string_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(string_bytes);
        buf.extend_from_slice(&vec![0u8; trailing]);

        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert_eq!(field.as_str(), Some("Hello"));
        assert_eq!(c.pos(), buf.len());
    }

    #[test]
    fn test_parse_mhoh_nonraw_small_data() {
        // Non-raw subtype with data_size between 1 and 15 (hits the else branch at line 702-712)
        let total_length = 24 + 8;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"mhoh");
        buf.extend_from_slice(&24u32.to_le_bytes());
        buf.extend_from_slice(&(total_length as u32).to_le_bytes());
        buf.extend_from_slice(&0x0002u32.to_le_bytes()); // TrackTitle - not a raw data type
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend_from_slice(&[0xAAu8; 8]);

        let mut c = Cursor::new(&buf);
        let field = super::parse_mhoh(&mut c).unwrap();
        assert_eq!(field.subtype, 0x0002);
        match &field.content {
            DataContent::RawData(d) => assert_eq!(d.len(), 8),
            _ => panic!("expected RawData for small data"),
        }
    }

    #[test]
    fn test_parse_playlist_track_short_section() {
        let data = build_mtph_short();
        let mut c = Cursor::new(&data);
        let id = super::parse_playlist_track(&mut c).unwrap();
        assert_eq!(id, 0);
    }
}
