use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process;

use itl_rs::ItlFile;

#[derive(Hash, PartialEq, Eq)]
struct TrackKey {
    title: String,
    artist: String,
    album: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <iTunes Library.itl> [--write]", args[0]);
        eprintln!();
        eprintln!("  Scans for duplicate tracks (same title + artist + album).");
        eprintln!("  Pass --write to save a deduplicated copy next to the original.");
        process::exit(1);
    }

    let itl_path = PathBuf::from(&args[1]);
    let do_write = args.iter().any(|a| a == "--write");

    let mut library = match ItlFile::open(&itl_path) {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("error: failed to open {}: {e}", itl_path.display());
            process::exit(1);
        }
    };

    println!("iTunes {}", library.version());
    println!(
        "Loaded {} tracks, {} playlists",
        library.tracks().len(),
        library.playlists().len()
    );
    println!();

    // Group tracks by (title, artist, album). The first occurrence in each
    // group is the "keeper"; the rest are duplicates.
    let mut seen: HashMap<TrackKey, Vec<usize>> = HashMap::new();
    for (idx, track) in library.tracks().iter().enumerate() {
        let key = TrackKey {
            title: track.title().unwrap_or("").to_lowercase(),
            artist: track.artist().unwrap_or("").to_lowercase(),
            album: track.album().unwrap_or("").to_lowercase(),
        };
        seen.entry(key).or_default().push(idx);
    }

    let mut dup_indices: Vec<usize> = Vec::new();
    let mut dup_groups = 0u64;
    for indices in seen.values() {
        if indices.len() > 1 {
            dup_groups += 1;
            let keeper = &library.tracks()[indices[0]];
            println!(
                "duplicate group ({} copies): \"{}\" - {} [{}]",
                indices.len(),
                keeper.title().unwrap_or("<no title>"),
                keeper.artist().unwrap_or("<no artist>"),
                keeper.album().unwrap_or("<no album>"),
            );

            // Keep the copy with the highest play count (prefer the one the
            // user has interacted with the most). Break ties by lowest index
            // (earliest added).
            let best = *indices
                .iter()
                .max_by_key(|&&i| library.tracks()[i].play_count())
                .unwrap();

            for &i in indices {
                if i != best {
                    dup_indices.push(i);
                }
            }
        }
    }

    if dup_indices.is_empty() {
        println!("No duplicates found.");
        return;
    }

    println!();
    println!(
        "Found {} duplicate groups, {} tracks to remove.",
        dup_groups,
        dup_indices.len()
    );

    // Collect the track IDs that will be removed so we can clean playlists.
    let removed_ids: HashSet<u32> = dup_indices
        .iter()
        .map(|&i| library.tracks()[i].id())
        .collect();

    // Remove duplicates from the track list (iterate in reverse so indices
    // stay valid).
    dup_indices.sort_unstable();
    dup_indices.dedup();
    for &i in dup_indices.iter().rev() {
        library.tracks_mut().remove(i);
    }

    // Scrub the removed IDs from every playlist.
    let mut playlist_fixes = 0u64;
    for playlist in library.playlists_mut() {
        let before = playlist.track_ids().len();
        for &id in &removed_ids {
            playlist.remove_track(id);
        }
        let after = playlist.track_ids().len();
        if before != after {
            playlist_fixes += 1;
        }
    }

    println!(
        "After dedup: {} tracks, {} playlists updated.",
        library.tracks().len(),
        playlist_fixes
    );

    if do_write {
        let out_path = itl_path.with_extension("deduped.itl");
        match library.save(&out_path) {
            Ok(()) => println!("Saved to {}", out_path.display()),
            Err(e) => {
                eprintln!("error: failed to save: {e}");
                process::exit(1);
            }
        }
    } else {
        println!();
        println!("Dry run — pass --write to save changes.");
    }
}
