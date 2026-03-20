# itl-rs

A Rust library for reading and writing Apple iTunes `Library.itl` files.

The ITL format is a proprietary binary format that stores library metadata
including tracks, playlists, albums, and artists. The file consists of a
big-endian envelope header followed by an AES-128-ECB encrypted and
zlib-compressed payload containing little-endian data sections.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
itl-rs = { path = "." }
```

### Reading a library

```rust
use itl_rs::ItlFile;

let library = ItlFile::open("/path/to/iTunes Library.itl").unwrap();

println!("iTunes version: {}", library.version());
println!("Tracks: {}", library.tracks().len());
println!("Playlists: {}", library.playlists().len());

for track in library.tracks() {
    if let Some(title) = track.title() {
        println!("  {} - {}", title, track.artist().unwrap_or("Unknown"));
    }
}
```

### Modifying and saving

```rust
use itl_rs::ItlFile;

let mut library = ItlFile::open("/path/to/iTunes Library.itl").unwrap();

// Edit track metadata
if let Some(track) = library.track_by_id_mut(42) {
    track.set_title("New Title");
    track.set_artist("New Artist");
}

// Remove tracks, update playlists, etc.
library.tracks_mut().retain(|t| t.play_count() > 0);

// Save to a new file
library.save("/path/to/output.itl").unwrap();
```

### Available metadata

**Track fields:** title, artist, album, album artist, genre, composer, kind,
local path, sort title, sort artist, sort album, play count, rating, date
added, and more via raw data fields.

**Album fields:** name, artist, persistent ID, rating.

**Artist fields:** name, sort name, persistent ID.

**Playlist fields:** title, track IDs, add/remove tracks.

**Library metadata:** iTunes version, library persistent ID, library date,
timezone offset, share name, msdh section count.

## Examples

### Dedup

Scans for duplicate tracks (same title + artist + album) and removes them,
keeping the copy with the highest play count:

```sh
# Dry run — report duplicates without writing
cargo run --example dedup -- "/path/to/iTunes Library.itl"

# Save a deduplicated copy
cargo run --example dedup -- "/path/to/iTunes Library.itl" --write
```

## Format overview

```
┌──────────────────────────────────────┐
│  Envelope Header (144 bytes, BE)     │
│  magic: "hdfm"                       │
│  version, library ID, crypt size,    │
│  timezone offset, library date       │
├──────────────────────────────────────┤
│  Encrypted + Compressed Payload      │
│  AES-128-ECB → zlib                  │
│  ┌────────────────────────────────┐  │
│  │  msdh sections (LE)            │  │
│  │  ├─ mfdh  (inner header)      │  │
│  │  ├─ mhgh  (library info)      │  │
│  │  ├─ mlth → mith (tracks)      │  │
│  │  ├─ mlah → miah (albums)      │  │
│  │  ├─ mlih → miih (artists)     │  │
│  │  └─ mlph → miph (playlists)   │  │
│  │           → mtph (track refs)  │  │
│  │           → mhoh (data fields) │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

## License

MIT
