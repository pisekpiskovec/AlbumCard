use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub type TagError = Box<dyn std::error::Error + Send + Sync>;

// Lets get proper error message out of lofty
fn err<E: std::error::Error>(e: E) -> TagError {
    let mut msg = e.to_string();
    let mut source = e.source();
    while let Some(s) = source {
        msg.push_str(": ");
        msg.push_str(&s.to_string());
        source = s.source();
    }
    msg.into()
}

use chrono::NaiveDate;
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::{AudioFile, FileType, TaggedFileExt},
    flac::FlacFile,
    mp4::{Atom, AtomData, AtomIdent, Mp4File},
    mpeg::MpegFile,
    ogg::{OpusFile, VorbisFile},
    picture::Picture,
    probe::Probe,
    tag::{Accessor, ItemKey, Tag},
};

use crate::models::{
    album::{Album, AlbumItem, AlbumItemPosition, AlbumMode},
    track::TrackEdit,
};

fn parse_opts() -> ParseOptions {
    ParseOptions::new().max_junk_bytes(1 << 20)
}

fn write_opts() -> WriteOptions {
    WriteOptions::new().parse_options(parse_opts())
}

/// A few fields have no standard key in every tag format.
fn custom_fallback(key: &ItemKey) -> Option<&'static str> {
    match key {
        ItemKey::Performer => Some("PERFORMER"),
        ItemKey::ComposerSortOrder => Some("COMPOSERSORT"),
        _ => None,
    }
}

fn custom_name(key: ItemKey) -> Option<&'static str> {
    match key {
        ItemKey::Performer => Some("PERFORMER"),
        ItemKey::ComposerSortOrder => Some("COMPOSERSORT"),
        _ => None,
    }
}

const MP4_MEAN: &str = "com.apple.iTunes";

fn mp4_ident(name: &str) -> AtomIdent<'static> {
    AtomIdent::Freeform {
        mean: MP4_MEAN.into(),
        name: name.to_string().into(),
    }
}

/// Reads a custom field. `None` if absent or the format has no such storage.
fn read_custom(path: &Path, name: &str) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let opts = parse_opts();
    let ft = FileType::from_path(path)?;
    match ft {
        FileType::Flac => {
            let f = FlacFile::read_from(&mut file, opts).ok()?;
            f.vorbis_comments()?.get(name).map(str::to_string)
        }
        FileType::Mpeg => {
            let f = MpegFile::read_from(&mut file, opts).ok()?;
            f.id3v2()?.get_user_text(name).map(str::to_string)
        }
        FileType::Mp4 => {
            let f = Mp4File::read_from(&mut file, opts).ok()?;
            let atom = f.ilst()?.get(&mp4_ident(name))?;
            atom.data().find_map(|d| match d {
                lofty::mp4::AtomData::UTF8(s) | lofty::mp4::AtomData::UTF16(s) => Some(s.clone()),
                _ => None,
            })
        }
        FileType::Opus => {
            let f = OpusFile::read_from(&mut file, opts).ok()?;
            f.vorbis_comments().get(name).map(str::to_string)
        }
        FileType::Vorbis => {
            let f = VorbisFile::read_from(&mut file, opts).ok()?;
            f.vorbis_comments().get(name).map(str::to_string)
        }
        _ => None,
    }
}

/// Reads the tags of `path` into a `TrackEdit`, ready to prefill the edit dialog.
/// Never invents data: a missing title falls back to the filename
/// (so the track passes validation), everythin else stays blank.
pub fn read_track_edit(path: &Path, album_artist: &str, album_genre: &str) -> Result<TrackEdit, TagError> {
    let tagged = Probe::open(path)
        .map_err(err)?
        .options(parse_opts())
        .read()
        .map_err(err)?;
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(TrackEdit {
            title: fallback_title,
            ..TrackEdit::default()
        });
    };

    let text = |key: ItemKey| {
        tag.get_string(key)
            .map(str::to_string)
            .or_else(|| custom_name(key).and_then(|n| read_custom(path, n)))
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    let inherit = |value: String, album_value: &str| {
        if value == album_value.trim() {
            String::new()
        } else {
            value
        }
    };

    let title = tag
        .title()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or(fallback_title);

    let release_date = text(ItemKey::RecordingDate);
    let release_date = release_date
        .get(..10)
        .filter(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
        .unwrap_or_default()
        .to_string();

    Ok(TrackEdit {
        file_path: path.to_path_buf(),
        title,
        artist: inherit(tag.artist().map(|a| a.trim().to_string()).unwrap_or_default(), album_artist),
        genre: inherit(tag.genre().map(|g| g.trim().to_string()).unwrap_or_default(), album_genre),
        release_date,
        performer: text(ItemKey::Performer),
        composer: text(ItemKey::Composer),
        lyricist: text(ItemKey::Lyricist),
        remixer: text(ItemKey::Remixer),
        sort_album_title: text(ItemKey::AlbumTitleSortOrder),
        sort_album_artist: text(ItemKey::AlbumArtistSortOrder),
        sort_composer: text(ItemKey::ComposerSortOrder),
        sort_performer: text(ItemKey::TrackArtistSortOrder),
        sort_track_title: text(ItemKey::TrackTitleSortOrder),
    })
}

/// Outcome of writing one track's file.
#[derive(Debug)]
pub struct WriteReport {
    /// Index into `album.get_items()`.
    pub index: usize,
    pub path: PathBuf,
    pub result: Result<(), TagError>,
    /// Fields this file's tag format has no way to store at all
    /// (not even as a custom field), so they were left out.
    ///
    /// Empty for the common formats.
    pub unsupported: Vec<&'static str>,
}

#[derive(Debug, Default)]
pub struct WriteSummary {
    pub reports: Vec<WriteReport>,
    /// Set when the album has cover art configured but the image could not be read;
    /// the tags were still written, just without the picture.
    pub art_error: Option<String>,
}

impl WriteSummary {
    pub fn failures(&self) -> impl Iterator<Item = &WriteReport> {
        self.reports.iter().filter(|r| r.result.is_err())
    }
}

/// Sets (`Some`) or removes (`None`) a custom field in the file.
/// Returns `Ok(false)` if this format has no place to store it.
fn write_custom(path: &Path, name: &str, value: Option<&str>) -> Result<bool, TagError> {
    let mut file = File::options().read(true).write(true).open(path)?;
    let parse_options = parse_opts();
    let ft = FileType::from_path(path);
    match ft {
        Some(FileType::Mpeg) => {
            let mut f = MpegFile::read_from(&mut file, parse_options).map_err(err)?;
            let Some(tag) = f.id3v2_mut() else {
                return Ok(value.is_none());
            };
            tag.remove_user_text(name);
            if let Some(v) = value {
                tag.insert_user_text(name.to_string(), v.to_string());
            }
            f.save_to_path(path, write_opts()).map_err(err)?;
        }
        Some(FileType::Mp4) => {
            let mut f = Mp4File::read_from(&mut file, parse_options).map_err(err)?;
            let Some(ilst) = f.ilst_mut() else {
                return Ok(value.is_none());
            };
            let ident = mp4_ident(name);
            drop(ilst.remove(&ident));
            if let Some(v) = value {
                ilst.replace_atom(Atom::new(ident, AtomData::UTF8(v.to_string())));
            }
            f.save_to_path(path, write_opts()).map_err(err)?;
        }
        Some(FileType::Flac) => {
            let mut f = FlacFile::read_from(&mut file, parse_options).map_err(err)?;
            let Some(vc) = f.vorbis_comments_mut() else {
                return Ok(value.is_none());
            };
            drop(vc.remove(name));
            if let Some(v) = value {
                vc.push(name.to_string(), v.to_string());
            }
            f.save_to_path(path, write_opts()).map_err(err)?;
        }
        Some(FileType::Vorbis) => {
            let mut f = VorbisFile::read_from(&mut file, parse_options).map_err(err)?;
            let vc = f.vorbis_comments_mut();
            drop(vc.remove(name));
            if let Some(v) = value {
                vc.push(name.to_string(), v.to_string());
            }
            f.save_to_path(path, write_opts()).map_err(err)?;
        }
        Some(FileType::Opus) => {
            let mut f = OpusFile::read_from(&mut file, parse_options).map_err(err)?;
            let vc = f.vorbis_comments_mut();
            drop(vc.remove(name));
            if let Some(v) = value {
                vc.push(name.to_string(), v.to_string());
            }
            f.save_to_path(path, write_opts()).map_err(err)?;
        }
        _ => return Ok(value.is_none()),
    }
    Ok(true)
}

/// Sets `key` to `value`, or removes it when `value` is `None` - so clearing a field
/// also clears it in the file instead of leaving stale data.
/// Returns `false` only if a value could not be stored in this format at all.
fn put(tag: &mut Tag, key: ItemKey, value: Option<&str>) -> bool {
    tag.remove_key(key);
    match value {
        None => true,
        Some(v) => tag.insert_text(key, v.to_string()),
    }
}

fn load_cover(path: &Path) -> Result<Picture, TagError> {
    let mut file = File::open(path)?;
    let mut picture = Picture::from_reader(&mut file)?;
    picture.set_pic_type(lofty::picture::PictureType::CoverFront);
    Ok(picture)
}

fn write_track_inner(
    album: &Album,
    track: &crate::models::track::Track,
    position: Option<AlbumItemPosition>,
    picture: Option<&Picture>,
    unsupported: &mut Vec<&'static str>,
) -> Result<(), TagError> {
    let path = &track.file_path;
    let mut tagged = Probe::open(path)
        .map_err(err)?
        .options(parse_opts())
        .read()
        .map_err(err)?;

    if tagged.primary_tag().is_none() {
        let tag_type = tagged.primary_tag_type();
        tagged.insert_tag(Tag::new(tag_type));
    }

    let tag = tagged
        .primary_tag_mut()
        .expect("a tag wa inserted just above if none existed");

    tag.set_title(track.title.clone());
    tag.set_artist(track.effective_artist(album).to_string());
    tag.set_album(album.title.clone());
    tag.set_genre(track.effective_genre(album).to_string());
    tag.insert_text(ItemKey::AlbumArtist, album.album_artist.clone());

    if let Some(position) = position {
        if let Some(n) = position.track
            && album.mode != AlbumMode::Single
        {
            tag.set_track(n);
        }
        match position.disk {
            // No disc grouping (or before the first Disk): don't claim a disc.
            crate::models::track::DiscLabel::Ungrouped | crate::models::track::DiscLabel::Unassigned => {
                tag.remove_disk()
            }
            crate::models::track::DiscLabel::Numbered(n) => tag.set_disk(n),
        }
    }

    // A track's own date wins; otherwise the album's, if it has one.
    let date = track
        .release_date
        .or(album.release_date)
        .map(|d| d.format("%Y-%m-%d").to_string());
    let fields: [(&'static str, ItemKey, Option<&str>); 10] = [
        ("release date", ItemKey::RecordingDate, date.as_deref()),
        ("performer", ItemKey::Performer, track.performer.as_deref()),
        ("composer", ItemKey::Composer, track.composer.as_deref()),
        ("lyricist", ItemKey::Lyricist, track.lyricist.as_deref()),
        ("remixer", ItemKey::Remixer, track.remixer.as_deref()),
        ("sort album title", ItemKey::AlbumTitleSortOrder, track.sort_album_title.as_deref()),
        ("sort album artist", ItemKey::AlbumArtistSortOrder, track.sort_album_artist.as_deref()),
        ("sort composer", ItemKey::ComposerSortOrder, track.sort_composer.as_deref()),
        ("sort performer", ItemKey::TrackArtistSortOrder, track.sort_performer.as_deref()),
        ("sort track title", ItemKey::TrackTitleSortOrder, track.sort_track_title.as_deref()),
    ];
    let mut custom: Vec<(&'static str, &'static str, Option<String>)> = Vec::new();
    for (name, key, value) in fields {
        let stored = put(tag, key, value);
        match custom_name(key) {
            Some(cn) if !stored || value.is_none() => {
                custom.push((name, cn, value.map(str::to_string)));
            }
            None if !stored => unsupported.push(name),
            None | Some(_) => {}
        }
    }

    if let Some(picture) = picture {
        tag.remove_picture_type(lofty::picture::PictureType::CoverFront);
        tag.push_picture(picture.clone());
    }

    tagged.save_to_path(path, write_opts()).map_err(err)?;

    for (label, cn, value) in custom {
        if !write_custom(path, cn, value.as_deref())? {
            unsupported.push(label);
        }
    }
    Ok(())
}

fn write_track(
    album: &Album,
    track: &crate::models::track::Track,
    position: Option<AlbumItemPosition>,
    picture: Option<&Picture>,
) -> (Result<(), TagError>, Vec<&'static str>) {
    let mut unsupported = Vec::new();
    let result = write_track_inner(album, track, position, picture, &mut unsupported);
    (result, unsupported)
}

/// Writes the album's metadata to every Track's file. Disk and Voids have no
/// file and are skipped; one failing file never stops the others.
///
/// Track numbers come from `item_positions()`, so Voids leave real gaps in the numbering.
/// No totals are written (track/disc totals are ambiguous with gaps).
/// Cover art is only embedded when the album has one; an album without art leaves any picture already in the files alone.
pub fn write_album_tags(album: &Album) -> WriteSummary {
    let mut summary = WriteSummary::default();

    let picture = match &album.art_path {
        Some(art_path) => match load_cover(art_path) {
            Ok(pic) => Some(pic),
            Err(e) => {
                summary.art_error = Some(format!("{}: {e}", art_path.display()));
                None
            }
        },
        None => None,
    };

    let positions = album.item_positions();
    for (index, (item, position)) in album.get_items().iter().zip(positions).enumerate() {
        let AlbumItem::Track(track) = item else {
            continue;
        };
        let (result, unsupported) = write_track(album, track, position, picture.as_ref());
        summary.reports.push(WriteReport {
            index,
            path: track.file_path.clone(),
            result,
            unsupported,
        });
    }

    summary
}
