/// String encoding type in mhoh flex character containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum StringEncoding {
    Uri = 0,
    Utf16 = 1,
    EscapedUri = 2,
    Utf8 = 3,
}

impl TryFrom<u32> for StringEncoding {
    type Error = u32;
    fn try_from(v: u32) -> Result<Self, u32> {
        match v {
            0 => Ok(Self::Uri),
            1 => Ok(Self::Utf16),
            2 => Ok(Self::EscapedUri),
            3 => Ok(Self::Utf8),
            other => Err(other),
        }
    }
}

/// Known mhoh subtype identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DataFieldType {
    TrackTitle = 0x0002,
    Album = 0x0003,
    Artist = 0x0004,
    Genre = 0x0005,
    Kind = 0x0006,
    Comment = 0x0008,
    Category = 0x0009,
    LocalPath = 0x000B,
    Composer = 0x000C,
    NativeFilepath = 0x000D,
    Grouping = 0x000E,
    ShortDescription = 0x0012,
    PodcastEpisodeUrl = 0x0013,
    FullDescription = 0x0016,
    TvShowTitle = 0x0018,
    EpisodeId = 0x0019,
    AlbumArtist = 0x001B,
    TvRating = 0x001C,
    XmlBlock = 0x001D,
    SortTrackTitle = 0x001E,
    SortAlbum = 0x001F,
    SortArtist = 0x0020,
    SortAlbumArtist = 0x0021,
    SortComposer = 0x0022,
    PodcastRssUrl = 0x0025,
    CopyrightStatement = 0x002E,
    AltDescription = 0x0033,
    ArtXmlBlock = 0x0036,
    DownloadXmlBlock = 0x0038,
    PodcastFeedUrl = 0x003A,
    PurchaserAccount = 0x003B,
    PurchaserName = 0x003C,
    WorkName = 0x003F,
    MovementName = 0x0040,
    PlaylistTitle = 0x0064,
    DisplayArtXml = 0x006D,
    PodcastTitle = 0x00C8,
    AlbumItemName = 0x012C,
    AlbumItemArtist = 0x012D,
    AlbumItemArtist2 = 0x012E,
    SeriesTitle = 0x0130,
    FeedUrl = 0x0131,
    ArtistName = 0x0190,
    SortArtistName = 0x0191,
    StoreArtUrlXml = 0x0192,
    LibraryOwner = 0x01FA,
    LibraryShareName = 0x01FC,
    LongXmlBlock = 0x0202,
    SmartPlaylistXml = 0x02BC,
    TrackTitleAlt = 0x02BE,
    ArtistAlbum = 0x02BF,
    TvDisplayXml = 0x0320,
}

impl DataFieldType {
    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            0x0002 => Self::TrackTitle,
            0x0003 => Self::Album,
            0x0004 => Self::Artist,
            0x0005 => Self::Genre,
            0x0006 => Self::Kind,
            0x0008 => Self::Comment,
            0x0009 => Self::Category,
            0x000B => Self::LocalPath,
            0x000C => Self::Composer,
            0x000D => Self::NativeFilepath,
            0x000E => Self::Grouping,
            0x0012 => Self::ShortDescription,
            0x0013 => Self::PodcastEpisodeUrl,
            0x0016 => Self::FullDescription,
            0x0018 => Self::TvShowTitle,
            0x0019 => Self::EpisodeId,
            0x001B => Self::AlbumArtist,
            0x001C => Self::TvRating,
            0x001D => Self::XmlBlock,
            0x001E => Self::SortTrackTitle,
            0x001F => Self::SortAlbum,
            0x0020 => Self::SortArtist,
            0x0021 => Self::SortAlbumArtist,
            0x0022 => Self::SortComposer,
            0x0025 => Self::PodcastRssUrl,
            0x002E => Self::CopyrightStatement,
            0x0033 => Self::AltDescription,
            0x0036 => Self::ArtXmlBlock,
            0x0038 => Self::DownloadXmlBlock,
            0x003A => Self::PodcastFeedUrl,
            0x003B => Self::PurchaserAccount,
            0x003C => Self::PurchaserName,
            0x003F => Self::WorkName,
            0x0040 => Self::MovementName,
            0x0064 => Self::PlaylistTitle,
            0x006D => Self::DisplayArtXml,
            0x00C8 => Self::PodcastTitle,
            0x012C => Self::AlbumItemName,
            0x012D => Self::AlbumItemArtist,
            0x012E => Self::AlbumItemArtist2,
            0x0130 => Self::SeriesTitle,
            0x0131 => Self::FeedUrl,
            0x0190 => Self::ArtistName,
            0x0191 => Self::SortArtistName,
            0x0192 => Self::StoreArtUrlXml,
            0x01FA => Self::LibraryOwner,
            0x01FC => Self::LibraryShareName,
            0x0202 => Self::LongXmlBlock,
            0x02BC => Self::SmartPlaylistXml,
            0x02BE => Self::TrackTitleAlt,
            0x02BF => Self::ArtistAlbum,
            0x0320 => Self::TvDisplayXml,
            _ => return None,
        })
    }

    /// Returns true for mhoh subtypes where data starts at offset 24
    /// with no string header (narrow/binary types).
    pub fn is_raw_data_type(subtype: u32) -> bool {
        matches!(
            subtype,
            0x0013
                | 0x0036
                | 0x0038
                | 0x0042
                | 0x0068
                | 0x0069
                | 0x006A
                | 0x006B
                | 0x006C
                | 0x006D
                | 0x0192
                | 0x01F4
                | 0x01F7
                | 0x0202
                | 0x02BC
                | 0x0320
        )
    }
}

/// Content of a data field (mhoh section).
#[derive(Debug, Clone)]
pub enum DataContent {
    String {
        encoding: StringEncoding,
        value: String,
    },
    RawData(Vec<u8>),
}

/// A parsed mhoh data field.
#[derive(Debug, Clone)]
pub struct DataField {
    #[allow(dead_code)]
    pub(crate) raw_header: Vec<u8>,
    pub subtype: u32,
    pub content: DataContent,
}

impl DataField {
    pub fn known_type(&self) -> Option<DataFieldType> {
        DataFieldType::from_u32(self.subtype)
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.content {
            DataContent::String { value, .. } => Some(value.as_str()),
            DataContent::RawData(bytes) => std::str::from_utf8(bytes).ok(),
        }
    }

    pub fn set_string(&mut self, new_value: &str) {
        match &mut self.content {
            DataContent::String { value, .. } => {
                *value = new_value.to_string();
            }
            DataContent::RawData(_) => {
                self.content = DataContent::String {
                    encoding: StringEncoding::Utf8,
                    value: new_value.to_string(),
                };
            }
        }
    }
}

/// Seconds between 1904-01-01 and 1970-01-01.
pub const APPLE_EPOCH_OFFSET: u64 = 2_082_844_800;

/// Convert an Apple timestamp (seconds since 1904-01-01) to Unix timestamp.
pub fn apple_to_unix(apple_ts: u32) -> i64 {
    apple_ts as i64 - APPLE_EPOCH_OFFSET as i64
}

/// Convert a Unix timestamp to an Apple timestamp.
pub fn unix_to_apple(unix_ts: i64) -> u32 {
    (unix_ts + APPLE_EPOCH_OFFSET as i64) as u32
}

/// A track in the iTunes library.
#[derive(Debug, Clone)]
pub struct Track {
    pub(crate) raw_header: Vec<u8>,
    pub(crate) data_fields: Vec<DataField>,
}

impl Track {
    pub fn id(&self) -> u32 {
        u32::from_le_bytes(self.raw_header[8..12].try_into().unwrap())
    }

    pub fn mhoh_count(&self) -> u32 {
        u32::from_le_bytes(self.raw_header[4..8].try_into().unwrap())
    }

    pub fn play_count(&self) -> u32 {
        if self.raw_header.len() > 72 {
            u32::from_le_bytes(self.raw_header[68..72].try_into().unwrap())
        } else {
            0
        }
    }

    pub fn rating(&self) -> u8 {
        if self.raw_header.len() > 101 {
            self.raw_header[100]
        } else {
            0
        }
    }

    pub fn is_checked(&self) -> bool {
        if self.raw_header.len() > 103 {
            self.raw_header[102] == 0
        } else {
            true
        }
    }

    pub fn date_added_raw(&self) -> u32 {
        if self.raw_header.len() > 116 {
            u32::from_le_bytes(self.raw_header[112..116].try_into().unwrap())
        } else {
            0
        }
    }

    pub fn date_added_unix(&self) -> i64 {
        apple_to_unix(self.date_added_raw())
    }

    pub fn album_persistent_id(&self) -> u64 {
        if self.raw_header.len() > 128 {
            u64::from_le_bytes(self.raw_header[120..128].try_into().unwrap())
        } else {
            0
        }
    }

    fn field_str(&self, subtype: u32) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == subtype)
            .and_then(|f| f.as_str())
    }

    fn set_field_str(&mut self, subtype: u32, value: &str) {
        if let Some(field) = self.data_fields.iter_mut().find(|f| f.subtype == subtype) {
            field.set_string(value);
        } else {
            self.data_fields.push(DataField {
                raw_header: Vec::new(),
                subtype,
                content: DataContent::String {
                    encoding: StringEncoding::Utf8,
                    value: value.to_string(),
                },
            });
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.field_str(DataFieldType::TrackTitle as u32)
    }

    pub fn set_title(&mut self, title: &str) {
        self.set_field_str(DataFieldType::TrackTitle as u32, title);
    }

    pub fn artist(&self) -> Option<&str> {
        self.field_str(DataFieldType::Artist as u32)
    }

    pub fn set_artist(&mut self, artist: &str) {
        self.set_field_str(DataFieldType::Artist as u32, artist);
    }

    pub fn album(&self) -> Option<&str> {
        self.field_str(DataFieldType::Album as u32)
    }

    pub fn set_album(&mut self, album: &str) {
        self.set_field_str(DataFieldType::Album as u32, album);
    }

    pub fn album_artist(&self) -> Option<&str> {
        self.field_str(DataFieldType::AlbumArtist as u32)
    }

    pub fn set_album_artist(&mut self, artist: &str) {
        self.set_field_str(DataFieldType::AlbumArtist as u32, artist);
    }

    pub fn genre(&self) -> Option<&str> {
        self.field_str(DataFieldType::Genre as u32)
    }

    pub fn set_genre(&mut self, genre: &str) {
        self.set_field_str(DataFieldType::Genre as u32, genre);
    }

    pub fn composer(&self) -> Option<&str> {
        self.field_str(DataFieldType::Composer as u32)
    }

    pub fn kind(&self) -> Option<&str> {
        self.field_str(DataFieldType::Kind as u32)
    }

    pub fn local_path(&self) -> Option<&str> {
        self.field_str(DataFieldType::LocalPath as u32)
    }

    pub fn sort_title(&self) -> Option<&str> {
        self.field_str(DataFieldType::SortTrackTitle as u32)
    }

    pub fn sort_artist(&self) -> Option<&str> {
        self.field_str(DataFieldType::SortArtist as u32)
    }

    pub fn sort_album(&self) -> Option<&str> {
        self.field_str(DataFieldType::SortAlbum as u32)
    }

    pub fn data_fields(&self) -> &[DataField] {
        &self.data_fields
    }

    pub fn data_fields_mut(&mut self) -> &mut Vec<DataField> {
        &mut self.data_fields
    }
}

/// An album in the iTunes library.
#[derive(Debug, Clone)]
pub struct Album {
    pub(crate) raw_header: Vec<u8>,
    pub(crate) data_fields: Vec<DataField>,
}

impl Album {
    pub fn persistent_id(&self) -> u64 {
        if self.raw_header.len() > 32 {
            u64::from_le_bytes(self.raw_header[24..32].try_into().unwrap())
        } else {
            0
        }
    }

    pub fn rating(&self) -> u8 {
        if self.raw_header.len() > 33 {
            self.raw_header[32]
        } else {
            0
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::AlbumItemName as u32)
            .and_then(|f| f.as_str())
    }

    pub fn artist(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::AlbumItemArtist as u32)
            .and_then(|f| f.as_str())
    }

    pub fn data_fields(&self) -> &[DataField] {
        &self.data_fields
    }
}

/// An artist in the iTunes library.
#[derive(Debug, Clone)]
pub struct Artist {
    pub(crate) raw_header: Vec<u8>,
    pub(crate) data_fields: Vec<DataField>,
}

impl Artist {
    pub fn persistent_id(&self) -> u64 {
        if self.raw_header.len() > 20 {
            u64::from_le_bytes(self.raw_header[12..20].try_into().unwrap())
        } else {
            0
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::ArtistName as u32)
            .and_then(|f| f.as_str())
    }

    pub fn sort_name(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::SortArtistName as u32)
            .and_then(|f| f.as_str())
    }

    pub fn data_fields(&self) -> &[DataField] {
        &self.data_fields
    }
}

/// A playlist in the iTunes library.
#[derive(Debug, Clone)]
pub struct Playlist {
    pub(crate) raw_header: Vec<u8>,
    pub(crate) data_fields: Vec<DataField>,
    pub(crate) track_ids: Vec<u32>,
}

impl Playlist {
    pub fn item_count(&self) -> u32 {
        if self.raw_header.len() > 16 {
            u32::from_le_bytes(self.raw_header[12..16].try_into().unwrap())
        } else {
            0
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.data_fields
            .iter()
            .find(|f| f.subtype == DataFieldType::PlaylistTitle as u32)
            .and_then(|f| f.as_str())
    }

    pub fn track_ids(&self) -> &[u32] {
        &self.track_ids
    }

    pub fn add_track(&mut self, track_id: u32) {
        self.track_ids.push(track_id);
    }

    pub fn remove_track(&mut self, track_id: u32) {
        self.track_ids.retain(|&id| id != track_id);
    }

    pub fn data_fields(&self) -> &[DataField] {
        &self.data_fields
    }
}

/// An unparsed section preserved for round-trip fidelity.
#[derive(Debug, Clone)]
pub struct RawSection {
    pub sig: [u8; 4],
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_string_field(subtype: u32, value: &str) -> DataField {
        DataField {
            raw_header: Vec::new(),
            subtype,
            content: DataContent::String {
                encoding: StringEncoding::Utf8,
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

    fn track_header_200() -> Vec<u8> {
        let mut h = vec![0u8; 200];
        h[4..8].copy_from_slice(&7u32.to_le_bytes());
        h[8..12].copy_from_slice(&0xABCDEF01u32.to_le_bytes());
        h[68..72].copy_from_slice(&100u32.to_le_bytes());
        h[100] = 80;
        h[102] = 0;
        h[112..116].copy_from_slice(&3_659_329_801u32.to_le_bytes());
        h[120..128].copy_from_slice(&0x1122334455667788u64.to_le_bytes());
        h
    }

    #[test]
    fn string_encoding_try_from_valid_and_invalid() {
        assert_eq!(StringEncoding::try_from(0u32), Ok(StringEncoding::Uri));
        assert_eq!(StringEncoding::try_from(1u32), Ok(StringEncoding::Utf16));
        assert_eq!(StringEncoding::try_from(2u32), Ok(StringEncoding::EscapedUri));
        assert_eq!(StringEncoding::try_from(3u32), Ok(StringEncoding::Utf8));
        assert_eq!(StringEncoding::try_from(99u32), Err(99u32));
    }

    #[test]
    fn data_field_type_from_u32_known_and_unknown() {
        assert_eq!(
            DataFieldType::from_u32(0x0002),
            Some(DataFieldType::TrackTitle)
        );
        assert_eq!(DataFieldType::from_u32(0x0004), Some(DataFieldType::Artist));
        assert_eq!(DataFieldType::from_u32(0x0003), Some(DataFieldType::Album));
        assert_eq!(DataFieldType::from_u32(0x0005), Some(DataFieldType::Genre));
        assert_eq!(DataFieldType::from_u32(0x0064), Some(DataFieldType::PlaylistTitle));
        assert_eq!(DataFieldType::from_u32(0x02BC), Some(DataFieldType::SmartPlaylistXml));
        assert_eq!(DataFieldType::from_u32(0x9999), None);
    }

    #[test]
    fn data_field_type_is_raw_data_type() {
        for &st in &[
            0x0013u32, 0x0036, 0x0038, 0x0042, 0x0068, 0x006D, 0x0192, 0x0202, 0x02BC, 0x0320,
        ] {
            assert!(
                DataFieldType::is_raw_data_type(st),
                "expected raw: {st:#x}"
            );
        }
        for &st in &[0x0002u32, 0x0004, 0x0064] {
            assert!(
                !DataFieldType::is_raw_data_type(st),
                "expected not raw: {st:#x}"
            );
        }
    }

    #[test]
    fn data_field_known_type() {
        let known = make_string_field(DataFieldType::Artist as u32, "x");
        assert_eq!(known.known_type(), Some(DataFieldType::Artist));

        let unknown = make_string_field(0xDEAD_BEEF, "y");
        assert_eq!(unknown.known_type(), None);
    }

    #[test]
    fn data_field_as_str_string_and_raw() {
        let s = make_string_field(1, "hello");
        assert_eq!(s.as_str(), Some("hello"));

        let raw_ok = make_raw_field(2, b"utf8");
        assert_eq!(raw_ok.as_str(), Some("utf8"));

        let raw_bad = make_raw_field(3, &[0xFF, 0xFE, 0xFD]);
        assert_eq!(raw_bad.as_str(), None);
    }

    #[test]
    fn data_field_set_string_on_string_and_raw() {
        let mut f = make_string_field(0x10, "old");
        f.set_string("new");
        match &f.content {
            DataContent::String { value, .. } => assert_eq!(value, "new"),
            _ => panic!("expected String"),
        }

        let mut r = make_raw_field(0x11, b"bytes");
        r.set_string("text");
        match &r.content {
            DataContent::String {
                encoding,
                value,
            } => {
                assert_eq!(*encoding, StringEncoding::Utf8);
                assert_eq!(value, "text");
            }
            _ => panic!("expected String after set_string on RawData"),
        }
    }

    #[test]
    fn apple_unix_round_trip_known_zero_and_boundaries() {
        let apple = 3_659_329_801u32;
        let unix = apple_to_unix(apple);
        assert_eq!(unix, 1_576_485_001);
        assert_eq!(unix_to_apple(unix), apple);

        assert_eq!(apple_to_unix(0), -(APPLE_EPOCH_OFFSET as i64));
        assert_eq!(unix_to_apple(-(APPLE_EPOCH_OFFSET as i64)), 0);

        let max_apple = u32::MAX;
        let u = apple_to_unix(max_apple);
        assert_eq!(unix_to_apple(u), max_apple);
    }

    #[test]
    fn track_full_header_and_data_fields() {
        let header = track_header_200();
        let fields = vec![
            make_string_field(DataFieldType::TrackTitle as u32, "Title"),
            make_string_field(DataFieldType::Artist as u32, "Artist"),
            make_string_field(DataFieldType::Album as u32, "Album"),
            make_string_field(DataFieldType::AlbumArtist as u32, "AlbumArtist"),
            make_string_field(DataFieldType::Genre as u32, "Genre"),
            make_string_field(DataFieldType::Composer as u32, "Composer"),
            make_string_field(DataFieldType::Kind as u32, "Kind"),
            make_string_field(DataFieldType::LocalPath as u32, "/path"),
            make_string_field(DataFieldType::SortTrackTitle as u32, "STitle"),
            make_string_field(DataFieldType::SortArtist as u32, "SArtist"),
            make_string_field(DataFieldType::SortAlbum as u32, "SAlbum"),
        ];
        let mut t = Track {
            raw_header: header,
            data_fields: fields,
        };

        assert_eq!(t.id(), 0xABCDEF01);
        assert_eq!(t.mhoh_count(), 7);
        assert_eq!(t.play_count(), 100);
        assert_eq!(t.rating(), 80);
        assert!(t.is_checked());
        assert_eq!(t.date_added_raw(), 3_659_329_801);
        assert_eq!(t.date_added_unix(), apple_to_unix(3_659_329_801));
        assert_eq!(t.album_persistent_id(), 0x1122334455667788);

        assert_eq!(t.title(), Some("Title"));
        assert_eq!(t.artist(), Some("Artist"));
        assert_eq!(t.album(), Some("Album"));
        assert_eq!(t.album_artist(), Some("AlbumArtist"));
        assert_eq!(t.genre(), Some("Genre"));
        assert_eq!(t.composer(), Some("Composer"));
        assert_eq!(t.kind(), Some("Kind"));
        assert_eq!(t.local_path(), Some("/path"));
        assert_eq!(t.sort_title(), Some("STitle"));
        assert_eq!(t.sort_artist(), Some("SArtist"));
        assert_eq!(t.sort_album(), Some("SAlbum"));
        assert_eq!(t.data_fields().len(), 11);

        t.set_title("T2");
        assert_eq!(t.title(), Some("T2"));
        t.set_artist("A2");
        t.set_album("Al2");
        t.set_album_artist("AA2");
        t.set_genre("G2");
        assert_eq!(t.artist(), Some("A2"));
        assert_eq!(t.album(), Some("Al2"));
        assert_eq!(t.album_artist(), Some("AA2"));
        assert_eq!(t.genre(), Some("G2"));

        t.data_fields_mut().push(make_string_field(0x9999, "extra"));
        assert_eq!(t.data_fields().last().unwrap().subtype, 0x9999);
    }

    #[test]
    fn track_short_header_defaults() {
        let short = vec![0u8; 10];
        let t = Track {
            raw_header: short,
            data_fields: Vec::new(),
        };
        assert_eq!(t.play_count(), 0);
        assert_eq!(t.rating(), 0);
        assert!(t.is_checked());
        assert_eq!(t.date_added_raw(), 0);
        assert_eq!(t.date_added_unix(), apple_to_unix(0));
        assert_eq!(t.album_persistent_id(), 0);
    }

    #[test]
    fn track_unchecked_flag() {
        let mut h = track_header_200();
        h[102] = 1;
        let t = Track {
            raw_header: h,
            data_fields: Vec::new(),
        };
        assert!(!t.is_checked());
    }

    #[test]
    fn album_full_and_short_header() {
        let mut rh = vec![0u8; 40];
        rh[24..32].copy_from_slice(&0xAABBCCDDEEFF0011u64.to_le_bytes());
        rh[32] = 60;
        let a = Album {
            raw_header: rh,
            data_fields: vec![
                make_string_field(DataFieldType::AlbumItemName as u32, "AlbumName"),
                make_string_field(DataFieldType::AlbumItemArtist as u32, "AlbumArtist"),
            ],
        };
        assert_eq!(a.persistent_id(), 0xAABBCCDDEEFF0011);
        assert_eq!(a.rating(), 60);
        assert_eq!(a.name(), Some("AlbumName"));
        assert_eq!(a.artist(), Some("AlbumArtist"));
        assert_eq!(a.data_fields().len(), 2);

        let short = Album {
            raw_header: vec![0u8; 20],
            data_fields: Vec::new(),
        };
        assert_eq!(short.persistent_id(), 0);
        assert_eq!(short.rating(), 0);
    }

    #[test]
    fn artist_full_and_short_header() {
        let mut rh = vec![0u8; 24];
        rh[12..20].copy_from_slice(&0x0102030405060708u64.to_le_bytes());
        let ar = Artist {
            raw_header: rh,
            data_fields: vec![
                make_string_field(DataFieldType::ArtistName as u32, "Name"),
                make_string_field(DataFieldType::SortArtistName as u32, "Sort"),
            ],
        };
        assert_eq!(ar.persistent_id(), 0x0102030405060708);
        assert_eq!(ar.name(), Some("Name"));
        assert_eq!(ar.sort_name(), Some("Sort"));
        assert_eq!(ar.data_fields().len(), 2);

        let short = Artist {
            raw_header: vec![0u8; 10],
            data_fields: Vec::new(),
        };
        assert_eq!(short.persistent_id(), 0);
    }

    #[test]
    fn playlist_full_and_short_header() {
        let mut rh = vec![0u8; 20];
        rh[12..16].copy_from_slice(&99u32.to_le_bytes());
        let mut p = Playlist {
            raw_header: rh,
            data_fields: vec![make_string_field(DataFieldType::PlaylistTitle as u32, "My List")],
            track_ids: vec![1, 2, 3],
        };
        assert_eq!(p.item_count(), 99);
        assert_eq!(p.title(), Some("My List"));
        assert_eq!(p.track_ids(), &[1, 2, 3]);

        p.add_track(4);
        assert_eq!(p.track_ids(), &[1, 2, 3, 4]);
        p.remove_track(2);
        assert_eq!(p.track_ids(), &[1, 3, 4]);
        assert_eq!(p.data_fields().len(), 1);

        let short = Playlist {
            raw_header: vec![0u8; 10],
            data_fields: Vec::new(),
            track_ids: Vec::new(),
        };
        assert_eq!(short.item_count(), 0);
    }

    #[test]
    fn raw_section_constructible() {
        let r = RawSection {
            sig: *b"abcd",
            data: vec![1, 2, 3],
        };
        assert_eq!(r.sig, *b"abcd");
        assert_eq!(r.data, vec![1, 2, 3]);
    }

    #[test]
    fn data_field_type_from_u32_exhaustive() {
        let all_known: &[(u32, DataFieldType)] = &[
            (0x0002, DataFieldType::TrackTitle),
            (0x0003, DataFieldType::Album),
            (0x0004, DataFieldType::Artist),
            (0x0005, DataFieldType::Genre),
            (0x0006, DataFieldType::Kind),
            (0x0008, DataFieldType::Comment),
            (0x0009, DataFieldType::Category),
            (0x000B, DataFieldType::LocalPath),
            (0x000C, DataFieldType::Composer),
            (0x000D, DataFieldType::NativeFilepath),
            (0x000E, DataFieldType::Grouping),
            (0x0012, DataFieldType::ShortDescription),
            (0x0013, DataFieldType::PodcastEpisodeUrl),
            (0x0016, DataFieldType::FullDescription),
            (0x0018, DataFieldType::TvShowTitle),
            (0x0019, DataFieldType::EpisodeId),
            (0x001B, DataFieldType::AlbumArtist),
            (0x001C, DataFieldType::TvRating),
            (0x001D, DataFieldType::XmlBlock),
            (0x001E, DataFieldType::SortTrackTitle),
            (0x001F, DataFieldType::SortAlbum),
            (0x0020, DataFieldType::SortArtist),
            (0x0021, DataFieldType::SortAlbumArtist),
            (0x0022, DataFieldType::SortComposer),
            (0x0025, DataFieldType::PodcastRssUrl),
            (0x002E, DataFieldType::CopyrightStatement),
            (0x0033, DataFieldType::AltDescription),
            (0x0036, DataFieldType::ArtXmlBlock),
            (0x0038, DataFieldType::DownloadXmlBlock),
            (0x003A, DataFieldType::PodcastFeedUrl),
            (0x003B, DataFieldType::PurchaserAccount),
            (0x003C, DataFieldType::PurchaserName),
            (0x003F, DataFieldType::WorkName),
            (0x0040, DataFieldType::MovementName),
            (0x0064, DataFieldType::PlaylistTitle),
            (0x006D, DataFieldType::DisplayArtXml),
            (0x00C8, DataFieldType::PodcastTitle),
            (0x012C, DataFieldType::AlbumItemName),
            (0x012D, DataFieldType::AlbumItemArtist),
            (0x012E, DataFieldType::AlbumItemArtist2),
            (0x0130, DataFieldType::SeriesTitle),
            (0x0131, DataFieldType::FeedUrl),
            (0x0190, DataFieldType::ArtistName),
            (0x0191, DataFieldType::SortArtistName),
            (0x0192, DataFieldType::StoreArtUrlXml),
            (0x01FA, DataFieldType::LibraryOwner),
            (0x01FC, DataFieldType::LibraryShareName),
            (0x0202, DataFieldType::LongXmlBlock),
            (0x02BC, DataFieldType::SmartPlaylistXml),
            (0x02BE, DataFieldType::TrackTitleAlt),
            (0x02BF, DataFieldType::ArtistAlbum),
            (0x0320, DataFieldType::TvDisplayXml),
        ];
        for &(val, expected) in all_known {
            assert_eq!(
                DataFieldType::from_u32(val),
                Some(expected),
                "from_u32({:#06X}) failed",
                val
            );
        }
    }

    #[test]
    fn track_set_field_creates_new_field() {
        let raw_header = vec![0u8; 200];
        let mut track = Track {
            raw_header,
            data_fields: vec![],
        };
        assert!(track.title().is_none());
        track.set_title("New Title");
        assert_eq!(track.title(), Some("New Title"));
        assert_eq!(track.data_fields.len(), 1);

        track.set_artist("New Artist");
        assert_eq!(track.artist(), Some("New Artist"));
        assert_eq!(track.data_fields.len(), 2);
    }
}
