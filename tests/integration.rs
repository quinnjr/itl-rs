use itl_rs::ItlFile;

const DEFAULT_ITL_PATH: &str =
    "/run/media/joseph/Local Disk/Users/Joseph/Music/iTunes/iTunes Library.itl";

/// Open the full-size reference library, honoring `ITL_TEST_PATH`. Returns
/// `None` (so the test skips instead of panicking) when the file is not
/// available on this machine.
fn open_real_library() -> Option<ItlFile> {
    let path =
        std::env::var("ITL_TEST_PATH").unwrap_or_else(|_| DEFAULT_ITL_PATH.to_string());
    if !std::path::Path::new(&path).exists() {
        eprintln!("skipping: real library not found at {path} (set ITL_TEST_PATH to override)");
        return None;
    }
    Some(ItlFile::open(&path).expect("failed to open ITL file"))
}

#[test]
fn open_and_read() {
    let Some(lib) = open_real_library() else {
        return;
    };

    println!("{lib:?}");

    assert!(!lib.version().is_empty());
    assert!(lib.library_persistent_id() != 0);
    assert!(lib.tz_offset_seconds() != 0);
    assert!(lib.library_date_unix() > 0);

    assert!(
        lib.tracks().len() > 1000,
        "expected many tracks, got {}",
        lib.tracks().len()
    );
    assert!(
        lib.albums().len() > 100,
        "expected many albums, got {}",
        lib.albums().len()
    );
    assert!(
        lib.artists().len() > 100,
        "expected many artists, got {}",
        lib.artists().len()
    );
    assert!(
        lib.playlists().len() > 10,
        "expected many playlists, got {}",
        lib.playlists().len()
    );

    // Verify track metadata is present
    let tracks_with_title = lib.tracks().iter().filter(|t| t.title().is_some()).count();
    let tracks_with_artist = lib.tracks().iter().filter(|t| t.artist().is_some()).count();
    assert!(
        tracks_with_title > 1000,
        "expected most tracks to have titles, got {}",
        tracks_with_title
    );
    assert!(
        tracks_with_artist > 1000,
        "expected most tracks to have artists, got {}",
        tracks_with_artist
    );

    // Verify albums have names
    let albums_with_name = lib.albums().iter().filter(|a| a.name().is_some()).count();
    assert!(
        albums_with_name > 100,
        "expected most albums to have names, got {}",
        albums_with_name
    );

    // Verify artists have names
    let artists_with_name = lib.artists().iter().filter(|a| a.name().is_some()).count();
    assert!(
        artists_with_name > 100,
        "expected most artists to have names, got {}",
        artists_with_name
    );

    // Verify playlists
    let playlists_with_title = lib
        .playlists()
        .iter()
        .filter(|p| p.title().is_some())
        .count();
    assert!(
        playlists_with_title > 5,
        "expected playlists with titles, got {}",
        playlists_with_title
    );

    // Verify playlist track resolution
    let first_playlist_with_tracks = lib
        .playlists()
        .iter()
        .find(|p| !p.track_ids().is_empty())
        .expect("expected at least one playlist with tracks");
    let resolved = lib.playlist_tracks(first_playlist_with_tracks);
    assert!(
        !resolved.is_empty(),
        "expected to resolve at least some playlist tracks"
    );

    // Verify track_by_id lookup
    let first_track_id = lib.tracks()[0].id();
    let found = lib.track_by_id(first_track_id);
    assert!(found.is_some(), "track_by_id should find a track");
    assert_eq!(found.unwrap().id(), first_track_id);

    println!(
        "READ OK: {} tracks, {} albums, {} artists, {} playlists",
        lib.tracks().len(),
        lib.albums().len(),
        lib.artists().len(),
        lib.playlists().len()
    );
    println!(
        "  tracks with title: {}, with artist: {}",
        tracks_with_title, tracks_with_artist
    );
}

#[test]
fn round_trip_write() {
    let Some(mut lib) = open_real_library() else {
        return;
    };

    let original_track_count = lib.tracks().len();
    let original_album_count = lib.albums().len();
    let original_artist_count = lib.artists().len();
    let original_playlist_count = lib.playlists().len();
    let original_version = lib.version().to_string();

    // Serialize to bytes
    let bytes = lib.to_bytes().expect("failed to serialize");
    assert!(
        bytes.len() > 1000,
        "serialized output too small: {}",
        bytes.len()
    );

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

    println!(
        "ROUND-TRIP OK: serialized {} bytes, all counts and metadata match",
        bytes.len()
    );
}

#[test]
fn mutation() {
    let Some(mut lib) = open_real_library() else {
        return;
    };

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

    println!(
        "MUTATION OK: changed {:?} -> \"Test Title 12345\", {:?} -> \"Test Artist 67890\", survived round-trip",
        original_title, original_artist
    );
}

#[test]
fn track_persistent_id_is_nonzero_and_stable_across_reopens() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib1 = itl_rs::ItlFile::open(&path).unwrap();
    let lib2 = itl_rs::ItlFile::open(&path).unwrap();

    assert!(!lib1.tracks().is_empty(), "fixture has no tracks");
    for (t1, t2) in lib1.tracks().iter().zip(lib2.tracks().iter()) {
        assert_ne!(t1.persistent_id(), 0, "persistent_id should be nonzero");
        assert_eq!(
            t1.persistent_id(),
            t2.persistent_id(),
            "persistent_id must be stable across reopens",
        );
    }
}

#[test]
fn track_persistent_ids_are_mostly_unique_within_fixture() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    let mut pid_counts = std::collections::HashMap::new();
    let mut zero_count = 0;

    for t in lib.tracks() {
        let pid = t.persistent_id();
        if pid == 0 {
            zero_count += 1;
        } else {
            *pid_counts.entry(pid).or_insert(0) += 1;
        }
    }

    // Verify no tracks have zero persistent_id
    assert_eq!(
        zero_count, 0,
        "found {} tracks with persistent_id=0",
        zero_count
    );

    // Verify the vast majority of persistent_ids are unique
    // (allow for some duplicates due to iTunes import behavior)
    let total_tracks = lib.tracks().len();
    let unique_pids = pid_counts.len();
    let uniqueness_ratio = unique_pids as f64 / total_tracks as f64;

    // Real iTunes libraries commonly contain duplicate entries for the
    // same underlying file (re-imports, file moves, etc.), so pids are
    // not strictly unique. The observed floor on full-size libraries is
    // around 65% unique; 50% is a comfortable lower bound that still
    // catches a broken offset (which would land at much lower ratios).
    assert!(
        uniqueness_ratio > 0.5,
        "persistent_id uniqueness suspiciously low: {}/{} = {:.2}% \
         (bad offset would land well under 50%)",
        unique_pids,
        total_tracks,
        uniqueness_ratio * 100.0
    );
}

#[test]
fn playlist_persistent_id_is_nonzero_and_mostly_unique() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    assert!(!lib.playlists().is_empty(), "fixture has no playlists");

    let mut seen = std::collections::HashSet::new();
    let mut zeros = 0usize;
    for p in lib.playlists() {
        let pid = p.persistent_id();
        if pid == 0 {
            zeros += 1;
        } else {
            seen.insert(pid);
        }
    }

    let total = lib.playlists().len();
    assert!(
        zeros < total / 20,
        "too many zero persistent_ids: {zeros} of {total}",
    );
    let ratio = seen.len() as f64 / (total - zeros) as f64;
    assert!(
        ratio > 0.95,
        "playlist persistent_id uniqueness too low: {:.2}%",
        ratio * 100.0,
    );
}

#[test]
fn is_smart_agrees_with_datafield_scan() {
    // is_smart() is known-unreliable on modern iTunes libraries (see
    // method docs — iTunes 12+ doesn't store SmartPlaylistXml at
    // subtype 0x02BC). Its contract is "true iff a 0x02BC field is
    // present". Verify that contract.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    assert!(!lib.playlists().is_empty());
    for p in lib.playlists() {
        let has_xml = p.data_fields().iter().any(|f| f.subtype == 0x02BC);
        assert_eq!(
            p.is_smart(),
            has_xml,
            "is_smart() disagrees with data_fields check for {:?}",
            p.title(),
        );
    }
}

#[test]
fn folders_have_no_tracks() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    let mut folder_count = 0usize;
    for p in lib.playlists() {
        if p.is_folder() {
            folder_count += 1;
            assert!(
                p.track_ids().is_empty(),
                "is_folder()==true for {:?} but it has {} tracks",
                p.title(),
                p.track_ids().len(),
            );
        }
    }
    assert!(
        folder_count >= 1,
        "fixture should contain at least one folder"
    );
}

#[test]
fn track_audio_accessors_parse_reasonably() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    let total = lib.tracks().len();
    assert!(total > 0);

    let mut with_size = 0;
    let mut with_duration = 0;
    let mut with_bit_rate = 0;
    let mut with_sample_rate = 0;
    let mut sample_rate_in_range = 0;
    let mut duration_in_range = 0;

    for t in lib.tracks() {
        if t.size_bytes() > 0 {
            with_size += 1;
        }
        let d = t.duration_ms();
        if d > 0 {
            with_duration += 1;
            // Typical music tracks: 5 seconds to 4 hours.
            if (5_000..=4 * 60 * 60 * 1000).contains(&d) {
                duration_in_range += 1;
            }
        }
        if t.bit_rate() > 0 {
            with_bit_rate += 1;
        }
        let sr = t.sample_rate();
        if sr > 0 {
            with_sample_rate += 1;
            // Accept the common CD/DVD/HD rates.
            if matches!(
                sr,
                22050 | 32000 | 44100 | 48000 | 88200 | 96000 | 176400 | 192000
            ) {
                sample_rate_in_range += 1;
            }
        }
    }

    // The vast majority of tracks in a real library have these fields.
    assert!(
        with_size * 100 / total >= 95,
        "size: only {with_size}/{total} tracks populated",
    );
    assert!(
        with_duration * 100 / total >= 95,
        "duration: only {with_duration}/{total} tracks populated",
    );
    assert!(
        duration_in_range * 100 / with_duration.max(1) >= 95,
        "duration sanity: only {duration_in_range}/{with_duration} in 5s..4h range",
    );
    assert!(
        with_bit_rate * 100 / total >= 95,
        "bit_rate: only {with_bit_rate}/{total} tracks populated",
    );
    assert!(
        with_sample_rate * 100 / total >= 95,
        "sample_rate: only {with_sample_rate}/{total} tracks populated",
    );
    assert!(
        sample_rate_in_range * 100 / with_sample_rate.max(1) >= 95,
        "sample_rate sanity: only {sample_rate_in_range}/{with_sample_rate} at known standard rates",
    );
}

#[test]
fn track_small_numeric_accessors_degrade_gracefully() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();

    // track_number, disc_number, year, bpm are Option<u16> and correctly
    // unset across many tracks. We assert three things: Some(0) never
    // occurs; at least one track populates each field; and the large
    // majority of populated values are in plausible ranges. (iTunes
    // preserves whatever the user or importer wrote, so a handful of
    // tracks with nonsense years or BPMs is expected.)
    let mut any_track_number = false;
    let mut any_disc_number = false;
    let mut year_total = 0usize;
    let mut year_sane = 0usize;
    let mut bpm_total = 0usize;
    let mut bpm_sane = 0usize;
    for t in lib.tracks() {
        if let Some(n) = t.track_number() {
            assert!(n > 0, "track_number returned Some(0)");
            any_track_number = true;
        }
        if let Some(n) = t.disc_number() {
            assert!(n > 0, "disc_number returned Some(0)");
            any_disc_number = true;
        }
        if let Some(y) = t.year() {
            year_total += 1;
            if (1900..=2100).contains(&y) {
                year_sane += 1;
            }
        }
        if let Some(b) = t.bpm() {
            bpm_total += 1;
            if (1..=400).contains(&b) {
                bpm_sane += 1;
            }
        }
    }
    assert!(any_track_number, "expected some track with track_number");
    assert!(any_disc_number, "expected some track with disc_number");
    assert!(year_total > 0, "expected some track with year");
    assert!(bpm_total > 0, "expected some track with bpm");
    assert!(
        year_sane * 100 / year_total >= 99,
        "year sanity: {year_sane}/{year_total} in 1900..=2100",
    );
    assert!(
        bpm_sane * 100 / bpm_total >= 99,
        "bpm sanity: {bpm_sane}/{bpm_total} in 1..=400",
    );
}

#[test]
fn track_metadata_accessors_parse_reasonably() {
    // Fixture-backed sanity for the accessors that previously had no
    // real-data assertions: play_count, rating, date_added, and
    // album_persistent_id.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    let total = lib.tracks().len();
    assert!(total > 0);

    let album_pids: std::collections::HashSet<u64> =
        lib.albums().iter().map(|a| a.persistent_id()).collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut rating_ok = 0usize;
    let mut with_date = 0usize;
    let mut date_sane = 0usize;
    let mut with_album_pid = 0usize;
    let mut album_pid_resolves = 0usize;
    for t in lib.tracks() {
        // iTunes ratings are 0-100 in steps of 10 (half stars included).
        if t.rating() <= 100 && t.rating() % 10 == 0 {
            rating_ok += 1;
        }
        let d = t.date_added_unix();
        if t.date_added_raw() != 0 {
            with_date += 1;
            // iTunes launched in 2001; allow a generous 1995..now window.
            if (789_000_000..=now).contains(&d) {
                date_sane += 1;
            }
        }
        let pid = t.album_persistent_id();
        if pid != 0 {
            with_album_pid += 1;
            if album_pids.contains(&pid) {
                album_pid_resolves += 1;
            }
        }
    }

    assert!(
        rating_ok * 100 / total >= 99,
        "rating sanity: only {rating_ok}/{total} in 0..=100 step 10",
    );
    assert!(
        with_date * 100 / total >= 95,
        "date_added: only {with_date}/{total} tracks populated",
    );
    assert!(
        date_sane * 100 / with_date.max(1) >= 95,
        "date_added sanity: only {date_sane}/{with_date} in 1995..now",
    );
    assert!(
        with_album_pid * 100 / total >= 90,
        "album_persistent_id: only {with_album_pid}/{total} nonzero",
    );
    // Empirically, Track::album_persistent_id and Album::persistent_id are
    // (mostly) distinct ID namespaces in real libraries — only ~5% of
    // track values appear in the miah list — so we report the overlap
    // rather than asserting on it.
    eprintln!(
        "note: {album_pid_resolves}/{with_album_pid} track album pids match a miah persistent_id"
    );
}

#[test]
fn playlist_parent_persistent_ids_resolve_to_known_playlists() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini.itl");
    if !path.exists() {
        eprintln!("skipping: tests/fixtures/mini.itl not present");
        return;
    }

    let lib = itl_rs::ItlFile::open(&path).unwrap();
    let known: std::collections::HashSet<u64> =
        lib.playlists().iter().map(|p| p.persistent_id()).collect();

    let mut any_parent = false;
    let mut resolved = 0usize;
    let mut total = 0usize;
    for p in lib.playlists() {
        if let Some(parent) = p.parent_persistent_id() {
            any_parent = true;
            total += 1;
            if known.contains(&parent) {
                resolved += 1;
            }
        }
    }
    assert!(any_parent, "fixture should contain folder-nested playlists",);
    let ratio = resolved as f64 / total as f64;
    assert!(
        ratio > 0.95,
        "only {resolved}/{total} parent pids resolve ({:.1}%)",
        ratio * 100.0,
    );
}
