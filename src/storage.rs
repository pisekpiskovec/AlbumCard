use std::{fs, io, path::PathBuf, time::SystemTime};

use gtk::glib;
use uuid::Uuid;

use crate::models::album::Album;

fn albums_dir() -> PathBuf {
    glib::user_data_dir().join("albumcard").join("albums")
}

fn ensure_album_dir() -> io::Result<PathBuf> {
    let dir = albums_dir();
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
    let path = albums_dir().join(format!("{id}.json"));
    let json = fs::read_to_string(path)?;
    Album::from_json(&json).map_err(io::Error::other)
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
        let Ok(album) = Album::from_json(&json) else {
            continue;
        };
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push((modified, album.id, album.title));
    }

    entries.sort_by_key(|a| std::cmp::Reverse(a.0));
    entries.into_iter().map(|(_, id, title)| (id, title)).collect()
}
