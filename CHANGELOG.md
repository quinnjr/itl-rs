# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-08-26

### Fixed

- Tag-3 ("UTF-8") strings whose bytes are not valid UTF-8 now decode as
  ISO-8859-1 instead of becoming `UnknownString` (so `title()` etc. no
  longer return `None`). Windows iTunes writes 8-bit strings in the
  system code page under tag 3; iTunes and its XML export read them as
  Latin-1, and so does this crate now — 229/229 such titles in the
  reference library match the XML. New `StringEncoding::Latin1` variant
  serialises back under tag 3 byte-for-byte (`StringEncoding::tag()`).
- `Track::persistent_id()` now reads bytes 120..128 of the mhit header,
  which is the id iTunes' XML reports as `Persistent ID` (2,953 of 2,992
  tracks matched by file location). Bytes 24..32, read before, are a
  different identifier that never appears in the XML. Consumers that
  keyed rows on the old value must re-key.

## [1.1.0] - 2026-08-26

### Fixed

- `Playlist::is_folder()` now reads the folder flag byte in the `miph`
  header (offset 457 of the post-length header; located by matching all
  434 playlists of an iTunes 12.13 library against the XML `Folder` key).
  The previous "no track entries" heuristic was wrong in both directions:
  iTunes folders hold the union of their children's tracks, and empty
  regular playlists exist. Falls back to the heuristic only when the
  header is too short to carry the flag.
- `Playlist::is_smart()` now recognises iTunes 12's `SmartCriteria`
  (`0x65`) data field. Previously only the legacy `SmartPlaylistXml`
  (`0x02BC`) counted, so no smart playlist in a modern library was
  detected (0 of 406 in the reference library). Folders carry criteria
  too — check `is_folder()` first.

### Added

- `DataFieldType::SmartCriteria` (`0x65`) and `DataFieldType::SmartInfo`
  (`0x66`), plus `Playlist::smart_criteria()` / `smart_info()` returning
  the raw bytes — byte-identical to the XML's `Smart Criteria` /
  `Smart Info` values (415/415 in the reference library).

## [1.0.0] - 2026-08-14

First stable release, following a full conformance/efficiency audit and
an adversarially-verified code review of the entire crate.

### Added

- `DataContent::UnknownString` — flex-string segments with an unknown
  encoding tag (or bytes that don't decode under the claimed encoding)
  are preserved byte-for-byte and re-emitted verbatim on save.
  `as_str()` returns `None` for them instead of mangled text.
- `Playlist::remove_tracks(&HashSet<u32>)` — single-pass bulk removal.
- `ItlFile::raw_sections()` — view of unparsed non-msdh top-level
  sections, which now survive an open→save round trip verbatim.
- Original `mtph` playlist-entry bytes and the unknown tail of each
  mhoh common header are preserved through save.
- Integration tests honor `ITL_TEST_PATH` and skip (instead of
  panicking) when the reference library is absent; new fixture-backed
  sanity tests for `rating`, `date_added`, and msdh subtype handling.

### Changed

- **Breaking**: `Playlist::track_ids()` returns `Vec<u32>` (entries are
  now stored as internally-paired `PlaylistEntry` records).
- **Breaking**: `ItlError::UnknownSection` and
  `ItlError::InvalidStringEncoding` removed (never constructed).
- `Playlist::item_count()` derives from the entry list and can no
  longer go stale; `add_track`/`remove_track(s)` keep the on-disk
  header field in step.
- A pure open→save round trip no longer consolidates multi-section
  track/playlist lists: reindexing preserves section membership unless
  items were actually added or removed.
- `unix_to_apple` saturates at the representable range
  (1904-01-01..2040-02-06) instead of wrapping.
- `mlph` playlist-list master count is patched on write like the other
  list masters.
- Decryption copies only the encrypted prefix of the payload instead of
  the whole file; `playlist_tracks` resolves via an id index instead of
  a linear scan per entry.

### Fixed

- All file-supplied section lengths are validated before slicing:
  corrupt or truncated libraries now return `Err` instead of panicking
  (msdh, mith/miah/miih, miph, envelope version string).
- Truncated mhoh fields and blobs error instead of silently desyncing
  the cursor and misparsing everything after them.
- Off-by-one bounds guards in nine legacy accessors (`play_count`,
  `rating`, `is_checked`, `date_added_raw`, `album_persistent_id`,
  `Album::persistent_id`, `Album::rating`, `Artist::persistent_id`,
  playlist header count) returned defaults for headers of exactly the
  required length.
- `dedup` example: tie-break now actually keeps the earliest copy on
  equal play counts, the reported "keeper" is the track actually kept,
  and removals are single-pass.

## [0.2.0] - 2026-04-29

### Added

- `Track::persistent_id()` — 64-bit iTunes Persistent ID at mhit
  offset 24..32. Stable across library rebuilds; used by downstream
  consumers as the cross-sync identity key.
- `Playlist::persistent_id()` — 64-bit Persistent ID at miph offset
  432..440, confirmed against `iTunes Music Library.xml` ground truth.
- `Playlist::parent_persistent_id()` — optional parent folder
  reference at miph offset 520..528. Returns `None` when zero.
- `Playlist::is_smart()` — detects presence of a `SmartPlaylistXml`
  data field (subtype `0x02BC`). Documented as unreliable on iTunes
  12+ libraries; full smart-rule parity is deferred.
- `Playlist::is_folder()` — heuristic based on empty track list.
  Correctly identifies all folders in the iTunes 12 reference
  library; may misclassify empty regular playlists but never the
  reverse.
- `Track` audio accessors: `size_bytes`, `duration_ms`,
  `track_number`, `track_count`, `year`, `bit_rate`, `disc_number`,
  `disc_count`, `sample_rate`, `bpm`. All read from the mhit raw
  header at empirically-confirmed offsets.

## [0.1.0] - 2026-03-19

### Added

- Initial release.
- Parse iTunes `Library.itl` binary format (AES-128-ECB encrypted, zlib-compressed).
- Read/write support with round-trip fidelity for unknown fields.
- Track metadata: title, artist, album, album artist, genre, composer, kind,
  local path, sort fields, play count, rating, date added, album persistent ID.
- Album metadata: name, artist, persistent ID, rating.
- Artist metadata: name, sort name, persistent ID.
- Playlist support: title, track ID list, add/remove tracks.
- Library-level metadata: iTunes version, library persistent ID, library date,
  timezone offset, share name, msdh section count.
- Mutable accessors for tracks, albums, artists, and playlists.
- Automatic section reindexing before serialization to handle mutations safely.
- `dedup` example binary for removing duplicate tracks.
