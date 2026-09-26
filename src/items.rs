use chrono::NaiveDate;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumMode {
    Single,
    Ep,
}

#[derive(Debug)]
pub struct Album {
    pub(crate) mode: AlbumMode,
    pub(crate) title: String,
    pub(crate) album_artist: String,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) genre: String,
    items: Vec<AlbumItem>,
    pub(crate) art_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum AlbumItem {
    Disk { title: Option<String> },
    Track(Track),
    Void { size: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumItemType {
    Disk,
    Track,
    Void,
}

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub(crate) file_path: PathBuf,
    pub(crate) title: String,
    pub(crate) genre: Option<String>,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) performer: String,
    pub(crate) composer: String,
    pub(crate) lyricist: String,
    pub(crate) remixer: String,
    pub(crate) sort_album_title: String,
    pub(crate) sort_album_artist: String,
    pub(crate) sort_composer: String,
    pub(crate) sort_performer: String,
    pub(crate) sort_track_title: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackEdit {
    pub title: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlbumItemPosition {
    pub disk: DiscLabel,
    pub track: Option<u32>,
}

impl std::fmt::Display for AlbumItemPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.disk {
            DiscLabel::Ungrouped => {}
            DiscLabel::Unassigned => write!(f, "_.")?,
            DiscLabel::Numbered(n) => write!(f, "{n}.")?,
        }
        match self.track {
            Some(n) => write!(f, "{n}"),
            None => write!(f, "_"),
        }
    }
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

impl Album {
    pub fn new(
        mode: AlbumMode,
        title: &str,
        artist: &str,
        date: &str,
        genre: &str,
        art: Option<PathBuf>,
    ) -> Result<Self, chrono::ParseError> {
        let release_date = match date.trim() {
            "" => None,
            s => Some(NaiveDate::parse_from_str(s, "%Y-%m-%d")?),
        };
        Ok(Self {
            mode,
            title: title.to_string(),
            album_artist: artist.to_string(),
            release_date,
            genre: genre.to_string(),
            items: Vec::new(),
            art_path: art,
        })
    }

    /// Item kinds this album's mode currently permits
    pub fn allowed_item_types(&self) -> Vec<AlbumItemType> {
        match self.mode {
            AlbumMode::Single => vec![AlbumItemType::Track],
            AlbumMode::Ep => vec![
                AlbumItemType::Disk,
                AlbumItemType::Track,
                AlbumItemType::Void,
            ],
        }
    }

    /// Returns `false` if `disk` isn't allowed by the current mode.
    pub fn add_disk(&mut self, title: Option<String>) -> bool {
        if !self.can_add(AlbumItemType::Disk) {
            return false;
        }
        self.items.push(AlbumItem::Disk { title });
        true
    }

    /// Returns `false` if `track` isn't allowed by the current mode.
    pub fn add_track(&mut self, track: Track) -> bool {
        if !self.can_add(AlbumItemType::Track) {
            return false;
        }
        self.items.push(AlbumItem::Track(track));
        true
    }

    /// Returns `false` if `void` isn't allowed by the current mode.
    pub fn add_void(&mut self, size: u32) -> bool {
        if !self.can_add(AlbumItemType::Void) {
            return false;
        }
        self.items.push(AlbumItem::Void { size });
        true
    }

    /// Total number of track-number slots consumed by Tracks and Voids.
    /// Disks don't count.
    pub fn get_length(&self) -> u32 {
        let mut track_cnt: u32 = 0;
        for item in self.items.iter() {
            match item {
                AlbumItem::Disk { title: _ } => continue,
                AlbumItem::Track(_track) => track_cnt += 1,
                AlbumItem::Void { size } => track_cnt += size,
            }
        }

        track_cnt
    }

    pub fn get_items(&self) -> &[AlbumItem] {
        &self.items
    }

    /// Single source of truth for whether `kind` may be added right now,
    /// enforced by every `add_*` method so the model can never end up in
    /// an invalid state (e.g. a Single with a Disk, or two Tracks).
    fn can_add(&self, kind: AlbumItemType) -> bool {
        if !self.allowed_item_types().contains(&kind) {
            return false;
        }

        if kind == AlbumItemType::Track && self.mode == AlbumMode::Single {
            return !self.items.iter().any(|i| matches!(i, AlbumItem::Track(_)));
        }
        true
    }

    /// Per-item display position, parallel to `get_items()`. `None` for Disk
    /// headers (they render as a section label, not a numbered row).
    pub fn item_positions(&self) -> Vec<Option<AlbumItemPosition>> {
        let has_disks = self
            .items
            .iter()
            .any(|i| matches!(i, AlbumItem::Disk { .. }));
        let disc_label = |disk_no: u32| {
            if !has_disks {
                DiscLabel::Ungrouped
            } else if disk_no == 0 {
                DiscLabel::Unassigned
            } else {
                DiscLabel::Numbered(disk_no)
            }
        };

        let mut disk_no: u32 = 0;
        let mut track_no: u32 = 0;
        self.items
            .iter()
            .map(|item| match item {
                AlbumItem::Disk { .. } => {
                    disk_no += 1;
                    track_no = 0;
                    None
                }
                AlbumItem::Track(_) => {
                    track_no += 1;
                    Some(AlbumItemPosition {
                        disk: disc_label(disk_no),
                        track: Some(track_no),
                    })
                }
                AlbumItem::Void { size } => {
                    let position = AlbumItemPosition {
                        disk: disc_label(disk_no),
                        track: None,
                    };
                    track_no += size;
                    Some(position)
                }
            })
            .collect()
    }

    /// Removes and returns the item at `index`,
    /// or `None` if `index` is out of bounds.
    pub fn remove_item(&mut self, index: usize) -> Option<AlbumItem> {
        if index >= self.items.len() {
            return None;
        }
        Some(self.items.remove(index))
    }

    /// Moves the item at `from` so that it ends up at index `to` in the resulting
    /// list.
    /// Returns `false` and does nothing if either index is out of bounds;
    /// a `from == to` move is a no-op that still returns `true`.
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.items.len() || to >= self.items.len() {
            return false;
        }
        if from == to {
            return true;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        true
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

    pub fn edit_snapshot(&self) -> TrackEdit {
        TrackEdit {
            title: self.title.clone(),
            genre: self.genre.clone().unwrap_or_default(),
            release_date: self
                .release_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            performer: self.performer.clone(),
            composer: self.composer.clone(),
            lyricist: self.lyricist.clone(),
            remixer: self.remixer.clone(),
            sort_album_title: self.sort_album_title.clone(),
            sort_album_artist: self.sort_album_artist.clone(),
            sort_composer: self.sort_composer.clone(),
            sort_performer: self.sort_performer.clone(),
            sort_track_title: self.sort_track_title.clone(),
        }
    }

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

        self.title = title;
        self.genre = Self::blank_to_none(&edit.genre);
        self.release_date = release_date;
        self.performer = edit.performer.trim().to_string();
        self.composer = edit.composer.trim().to_string();
        self.lyricist = edit.lyricist.trim().to_string();
        self.remixer = edit.remixer.trim().to_string();
        self.sort_album_title = edit.sort_album_title.trim().to_string();
        self.sort_album_artist = edit.sort_album_artist.trim().to_string();
        self.sort_composer = edit.sort_composer.trim().to_string();
        self.sort_performer = edit.sort_performer.trim().to_string();
        self.sort_track_title = edit.sort_track_title.trim().to_string();

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
}
