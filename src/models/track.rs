use chrono::NaiveDate;
use std::path::PathBuf;

use crate::models::album::Album;

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub(crate) file_path: PathBuf,
    pub(crate) title: String,
    pub(crate) genre: Option<String>,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) artist: Option<String>,
    pub(crate) performer: Option<String>,
    pub(crate) composer: Option<String>,
    pub(crate) lyricist: Option<String>,
    pub(crate) remixer: Option<String>,
    pub(crate) sort_album_title: Option<String>,
    pub(crate) sort_album_artist: Option<String>,
    pub(crate) sort_composer: Option<String>,
    pub(crate) sort_performer: Option<String>,
    pub(crate) sort_track_title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackEdit {
    pub title: String,
    pub artist: String,
    pub genre: String,
    pub release_date: String,
    pub performer: String,
    pub composer: String,
    pub lyricist: String,
    pub remixer: String,
    pub sort_album_title: String,
    pub sort_album_artist: String,
    pub sort_composer: String,
    pub sort_performer: String,
    pub sort_track_title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackFieldError {
    TitleEmpty,
    InvalidReleaseDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscLabel {
    Ungrouped,
    Unassigned,
    Numbered(u32),
}

impl std::fmt::Display for TrackFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackFieldError::TitleEmpty => write!(f, "Title cannot be empty"),
            TrackFieldError::InvalidReleaseDate => {
                write!(f, "Release date must be in YYYY-MM-DD format")
            }
        }
    }
}

impl Track {
    pub fn new(file_path: PathBuf) -> Self {
        let mut track = Self::default();
        track.file_path = file_path;
        track
    }

    pub fn get_file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Snapshot of this track's editable fields, formatted as the raw string.
    pub fn edit_snapshot(&self) -> TrackEdit {
        TrackEdit {
            artist: self.artist.clone().unwrap_or_default(),
            title: self.title.clone(),
            genre: self.genre.clone().unwrap_or_default(),
            release_date: self
                .release_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            performer: self.performer.clone().unwrap_or_default(),
            composer: self.composer.clone().unwrap_or_default(),
            lyricist: self.lyricist.clone().unwrap_or_default(),
            remixer: self.remixer.clone().unwrap_or_default(),
            sort_album_title: self.sort_album_title.clone().unwrap_or_default(),
            sort_album_artist: self.sort_album_artist.clone().unwrap_or_default(),
            sort_composer: self.sort_composer.clone().unwrap_or_default(),
            sort_performer: self.sort_performer.clone().unwrap_or_default(),
            sort_track_title: self.sort_track_title.clone().unwrap_or_default(),
        }
    }

    /// Validates and normalizes `edit` (trims whitespace, blank optional fields become `None`,
    /// the date is parsed), then applies it to `self` - but only if every field is valid.
    pub fn apply_edit(&mut self, edit: TrackEdit) -> Result<(), Vec<TrackFieldError>> {
        let mut errors = Vec::new();

        let title = edit.title.trim().to_string();
        if title.is_empty() {
            errors.push(TrackFieldError::TitleEmpty);
        }

        let release_date = match edit.release_date.trim() {
            "" => None,
            s => match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    errors.push(TrackFieldError::InvalidReleaseDate);
                    None
                },
            },
        };

        if !errors.is_empty() {
            return Err(errors);
        }

        self.artist = Self::blank_to_none(&edit.artist);
        self.title = title;
        self.genre = Self::blank_to_none(&edit.genre);
        self.release_date = release_date;
        self.performer = Self::blank_to_none(&edit.performer);
        self.composer = Self::blank_to_none(&edit.composer);
        self.lyricist = Self::blank_to_none(&edit.lyricist);
        self.remixer = Self::blank_to_none(&edit.remixer);
        self.sort_album_title = Self::blank_to_none(&edit.sort_album_title);
        self.sort_album_artist = Self::blank_to_none(&edit.sort_album_artist);
        self.sort_composer = Self::blank_to_none(&edit.sort_composer);
        self.sort_performer = Self::blank_to_none(&edit.sort_performer);
        self.sort_track_title = Self::blank_to_none(&edit.sort_track_title);

        Ok(())
    }

    fn blank_to_none(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// This track's artist, falling back to the album's `album_artist`
    pub fn effective_artist<'a>(&'a self, album: &'a Album) -> &'a str {
        self.artist.as_deref().unwrap_or(&album.album_artist)
    }

    /// This track's genre, falling back to the album's `genre`
    pub fn effective_genre<'a>(&'a self, album: &'a Album) -> &'a str {
        self.genre.as_deref().unwrap_or(&album.genre)
    }
}
