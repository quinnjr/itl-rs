# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
