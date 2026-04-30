# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
