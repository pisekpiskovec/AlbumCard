use std::{fs, io, path::PathBuf, time::SystemTime};

use gtk::glib;
use uuid::Uuid;

use crate::{models::album::Album, sha};

fn albums_dir() -> PathBuf {
    glib::user_data_dir().join("albumcard").join("albums")
}

fn arts_dir() -> PathBuf {
    glib::user_data_dir().join("albumcard").join("arts")
}

fn ensure_album_dir() -> io::Result<PathBuf> {
    let dir = albums_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn ensure_art_dir() -> io::Result<PathBuf> {
    let dir = arts_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Writes `album` to `{id}.json` in the albums directory, creating the
/// directory on first use. Overwrites any existing file for the same id.
pub fn save_album(album: &Album) -> io::Result<()> {
    let dir = ensure_album_dir()?;
    let json = album.to_json().map_err(io::Error::other)?;
    fs::write(dir.join(format!("{}.json", album.id)), json)
}

/// Reads and parses a single album by id.
pub fn load_album(id: Uuid) -> io::Result<Album> {
    let json = fs::read_to_string(albums_dir().join(format!("{id}.json")))?;
    let mut album = Album::from_json(&json).map_err(io::Error::other)?;
    album.migrate_legacy_art();
    Ok(album)
}

pub fn list_albums() -> Vec<(Uuid, String)> {
    let Ok(dir) = ensure_album_dir() else {
        return Vec::new();
    };
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut entries: Vec<(SystemTime, Uuid, String)> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        };
        let Ok(json) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut album) = Album::from_json(&json) else {
            continue;
        };
        album.migrate_legacy_art();
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push((modified, album.id, album.title));
    }

    entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    entries.into_iter().map(|(_, id, title)| (id, title)).collect()
}

pub fn delete_album(id: Uuid) -> io::Result<()> {
    fs::remove_file(albums_dir().join(format!("{id}.json")))?;

    if let Some(dir) = dirs::cache_dir().map(|d| d.join("albumcard").join("covers"))
        && let Ok(entries) = fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            if entry.path().file_stem().and_then(|s| s.to_str()) == Some(&id.to_string()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    Ok(())
}

/// Copies the image to a safe place, calculates SHA and renames it.
pub fn store_image(path: PathBuf) -> Result<String, String> {
    // Get the safe place
    let dir = match ensure_art_dir() {
        Ok(v) => v,
        Err(e) => return Err(e.to_string()),
    };

    // Calculate SHA
    let hash = sha::Sha256::sha256(path.clone())?;

    // Copy the image
    if let Err(e) = fs::copy(path.clone(), dir.join(hash.clone())) {
        return Err(e.to_string());
    }

    // Rename the file
    // if let Err(e) = fs::rename(dir.join(path), dir.join(hash.clone())) {
    //     return Err(e.to_string());
    // }
    
    Ok(hash)
}

/// Restore art path from hash.
pub fn art_path(hash: String) -> PathBuf {
    ensure_art_dir().unwrap().join(hash)
}

pub fn image_ext(path: &std::path::Path) -> &'static str {
    use std::io::Read;
    let mut buf = [0u8; 12];
    let _ = std::fs::File::open(path).and_then(|mut f| f.read(&mut buf));
    match &buf {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [0xFF, 0xD8, ..] => "jpg",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P'] => "webp",
        _ => "png",
    }
}
