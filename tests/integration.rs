use itl_rs::ItlFile;

const ITL_PATH: &str = "/run/media/joseph/Local Disk/Users/Joseph/Music/iTunes/iTunes Library.itl";

#[test]
fn open_and_read() {
    let lib = ItlFile::open(ITL_PATH).expect("failed to open ITL file");

    println!("{lib:?}");

    assert!(!lib.version().is_empty());
    assert!(lib.library_persistent_id() != 0);
    assert!(lib.tz_offset_seconds() != 0);
    assert!(lib.library_date_unix() > 0);

    assert!(lib.tracks().len() > 1000, "expected many tracks, got {}", lib.tracks().len());
    assert!(lib.albums().len() > 100, "expected many albums, got {}", lib.albums().len());
    assert!(lib.artists().len() > 100, "expected many artists, got {}", lib.artists().len());
    assert!(lib.playlists().len() > 10, "expected many playlists, got {}", lib.playlists().len());

    // Verify track metadata is present
    let tracks_with_title = lib.tracks().iter().filter(|t| t.title().is_some()).count();
    let tracks_with_artist = lib.tracks().iter().filter(|t| t.artist().is_some()).count();
    assert!(tracks_with_title > 1000, "expected most tracks to have titles, got {}", tracks_with_title);
    assert!(tracks_with_artist > 1000, "expected most tracks to have artists, got {}", tracks_with_artist);

    // Verify albums have names
    let albums_with_name = lib.albums().iter().filter(|a| a.name().is_some()).count();
    assert!(albums_with_name > 100, "expected most albums to have names, got {}", albums_with_name);

    // Verify artists have names
    let artists_with_name = lib.artists().iter().filter(|a| a.name().is_some()).count();
    assert!(artists_with_name > 100, "expected most artists to have names, got {}", artists_with_name);

    // Verify playlists
    let playlists_with_title = lib.playlists().iter().filter(|p| p.title().is_some()).count();
    assert!(playlists_with_title > 5, "expected playlists with titles, got {}", playlists_with_title);

    // Verify playlist track resolution
    let first_playlist_with_tracks = lib.playlists().iter()
        .find(|p| !p.track_ids().is_empty())
        .expect("expected at least one playlist with tracks");
    let resolved = lib.playlist_tracks(first_playlist_with_tracks);
    assert!(!resolved.is_empty(), "expected to resolve at least some playlist tracks");

    // Verify track_by_id lookup
    let first_track_id = lib.tracks()[0].id();
    let found = lib.track_by_id(first_track_id);
    assert!(found.is_some(), "track_by_id should find a track");
    assert_eq!(found.unwrap().id(), first_track_id);

    println!("READ OK: {} tracks, {} albums, {} artists, {} playlists",
        lib.tracks().len(), lib.albums().len(), lib.artists().len(), lib.playlists().len());
    println!("  tracks with title: {}, with artist: {}", tracks_with_title, tracks_with_artist);
}

#[test]
fn round_trip_write() {
    let mut lib = ItlFile::open(ITL_PATH).expect("failed to open ITL file");

    let original_track_count = lib.tracks().len();
    let original_album_count = lib.albums().len();
    let original_artist_count = lib.artists().len();
    let original_playlist_count = lib.playlists().len();
    let original_version = lib.version().to_string();

    // Serialize to bytes
    let bytes = lib.to_bytes().expect("failed to serialize");
    assert!(bytes.len() > 1000, "serialized output too small: {}", bytes.len());

    // Re-parse from serialized bytes
    let lib2 = ItlFile::from_bytes(&bytes).expect("failed to re-parse serialized ITL");

    assert_eq!(lib2.version(), original_version);
    assert_eq!(lib2.tracks().len(), original_track_count);
    assert_eq!(lib2.albums().len(), original_album_count);
    assert_eq!(lib2.artists().len(), original_artist_count);
    assert_eq!(lib2.playlists().len(), original_playlist_count);

    // Spot-check first few tracks survived round-trip
    for i in 0..20.min(original_track_count) {
        let orig = &lib.tracks()[i];
        let rt = &lib2.tracks()[i];
        assert_eq!(orig.id(), rt.id(), "track ID mismatch at index {i}");
        assert_eq!(orig.title(), rt.title(), "title mismatch at index {i}");
        assert_eq!(orig.artist(), rt.artist(), "artist mismatch at index {i}");
        assert_eq!(orig.album(), rt.album(), "album mismatch at index {i}");
        assert_eq!(orig.genre(), rt.genre(), "genre mismatch at index {i}");
    }

    println!("ROUND-TRIP OK: serialized {} bytes, all counts and metadata match", bytes.len());
}

#[test]
fn mutation() {
    let mut lib = ItlFile::open(ITL_PATH).expect("failed to open ITL file");

    let original_title = lib.tracks()[0].title().map(String::from);
    let original_artist = lib.tracks()[0].artist().map(String::from);

    // Mutate
    lib.tracks_mut()[0].set_title("Test Title 12345");
    lib.tracks_mut()[0].set_artist("Test Artist 67890");

    assert_eq!(lib.tracks()[0].title(), Some("Test Title 12345"));
    assert_eq!(lib.tracks()[0].artist(), Some("Test Artist 67890"));

    // Serialize and re-parse to verify mutation survives round-trip
    let bytes = lib.to_bytes().expect("failed to serialize after mutation");
    let lib2 = ItlFile::from_bytes(&bytes).expect("failed to re-parse mutated ITL");

    assert_eq!(lib2.tracks()[0].title(), Some("Test Title 12345"));
    assert_eq!(lib2.tracks()[0].artist(), Some("Test Artist 67890"));

    // Second track should be unaffected
    let orig_second_title = lib.tracks()[1].title().map(String::from);
    assert_eq!(lib2.tracks()[1].title(), orig_second_title.as_deref());

    println!("MUTATION OK: changed {:?} -> \"Test Title 12345\", {:?} -> \"Test Artist 67890\", survived round-trip",
        original_title, original_artist);
}
