use core::fmt;
use std::path::{Path, PathBuf};

use crate::models::{album::ArtChange, track::TrackEdit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplAlbumArtMode {
    Change(PathBuf),
    Clear,
    Keep,
}

impl TemplAlbumArtMode {
    fn transform(value: String) -> TemplAlbumArtMode {
        match value.as_str() {
            "null" | "empty" => TemplAlbumArtMode::Clear,
            "custom" | "my" | "file" | "own" => TemplAlbumArtMode::Change(PathBuf::from("")),
            "skip" | "" | _ => TemplAlbumArtMode::Keep,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Templ {
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub year: Option<u16>,
    pub track: Option<u16>,
    pub disc: Option<u16>,
    pub genre: Option<String>,
    pub album_artist: Option<String>,
    pub composer: Option<String>,
    pub remixer: Option<String>,
    pub rating: Option<u8>,
    pub max_rating: Option<u8>,
    pub spotify_rating: Option<u8>,
    pub max_spotify_rating: Option<u32>,
    pub youtube_rating: Option<u8>,
    pub youtube_music_rating: Option<u8>,
    pub soundcloud_rating: Option<u8>,
    pub art_mode: Option<TemplAlbumArtMode>,
    pub art_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplError {
    pub line: Option<usize>,
    pub message: String,
}

impl fmt::Display for TemplError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(n) => write!(f, "line {n}: {}", self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for TemplError {}

fn whole_file(message: impl Into<String>) -> TemplError {
    TemplError {
        line: None,
        message: message.into(),
    }
}

/// Reads and parses a `.templ` file.
pub fn read_templ(path: &Path) -> Result<Templ, TemplError> {
    let bytes = std::fs::read(path).map_err(|e| whole_file(format!("{}: {e}", path.display())))?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| whole_file("template is not valid UTF-8 text"))?;
    parse_templ(text)
}

pub fn parse_templ(text: &str) -> Result<Templ, TemplError> {
    let lines: Vec<&str> = text.lines().collect();
    let line = |n: usize| -> Option<&str> { lines.get(n - 1).map(|l| l.trim()).filter(|l| !l.is_empty()) };

    let text_field = |n: usize| line(n).map(str::to_string);
    let multi = |n: usize| {
        line(n)
            .map(|l| {
                l.split('|')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|l| !l.is_empty())
    };

    fn number<T: TryFrom<u64>>(value: Option<&str>, n: usize, max: u64, what: &str) -> Result<Option<T>, TemplError> {
        let Some(raw) = value else {
            return Ok(None);
        };
        let bad = |why: &str| TemplError {
            line: Some(n),
            message: format!("{what}: {why} (got \"{raw}\")"),
        };
        let parsed: u64 = raw.parse().map_err(|_| bad("expected a whole number"))?;
        if parsed > max {
            return Err(bad(&format!("must be 0-{max}")));
        }
        T::try_from(parsed).map(Some).map_err(|_| bad("out of range"))
    }

    Ok(Templ {
        artist: multi(1),
        title: text_field(2),
        album: text_field(3),
        year: number(line(4), 4, 9999, "year")?,
        track: number(line(5), 5, 500, "track number")?,
        disc: number(line(6), 6, 500, "disc number")?,
        genre: multi(7),
        album_artist: multi(8),
        composer: multi(9),
        remixer: text_field(10),
        rating: number(line(11), 11, 255, "rating")?,
        max_rating: number(line(12), 12, 100, "maximal rating")?,
        spotify_rating: number(line(13), 13, 255, "Spotify rating")?,
        max_spotify_rating: number(line(14), 14, 4_000_000_000, "Spotify most played")?,
        youtube_rating: number(line(15), 15, 255, "YouTube rating")?,
        youtube_music_rating: number(line(19), 19, 255, "YouTube Music rating")?,
        soundcloud_rating: number(line(16), 16, 255, "SoundCloud rating")?,
        art_mode: text_field(17).map(TemplAlbumArtMode::transform),
        art_path: text_field(18).map(PathBuf::from),
    })
}

impl Templ {
    /// Fills the track-level fields of `edit` from this template.
    pub fn apply_to_track_edit(&self, edit: &mut TrackEdit, album_artist: &str, album_genre: &str) {
        let inherit = |value: &str, album_value: &str| {
            if value == album_value.trim() {
                String::new()
            } else {
                value.to_string()
            }
        };

        if let Some(v) = &self.title {
            edit.title = v.clone();
        }
        if let Some(v) = &self.artist {
            edit.artist = inherit(v, album_artist);
        }
        if let Some(v) = &self.genre {
            edit.genre = inherit(v, album_genre);
        }
        if let Some(v) = &self.composer {
            edit.composer = v.clone();
        }
        if let Some(v) = &self.remixer {
            edit.remixer = v.clone();
        }
    }

    pub fn rating_raw(&self) -> Option<u8> {
        let rating = u32::from(self.rating?);
        let raw = match self.max_rating {
            Some(max) if max > 0 => {
                let max = u32::from(max);
                (rating.min(max) * 255 + max / 2) / max
            }
            _ => rating,
        };
        u8::try_from(raw).ok().filter(|&r| r > 0)
    }

    /// What the template wants done with the album art.
    /// A relative path in the template is resolver against the template's own folder.
    pub fn art_change(&self, template_dir: &Path) -> ArtChange {
        match (&self.art_mode, &self.art_path) {
            (Some(TemplAlbumArtMode::Clear), _) => ArtChange::Clear,
            (Some(TemplAlbumArtMode::Change(_)), Some(p)) if p.is_relative() => {
                ArtChange::Set(template_dir.join(p))
            }
            (Some(TemplAlbumArtMode::Change(_)), Some(p)) => ArtChange::Set(p.clone()),
            _ => ArtChange::Keep,
        }
    }
}
