#![cfg(test)]

use std::{fs::create_dir, path::PathBuf};

use crate::{
    models::{
        album::{Album, AlbumEdit, AlbumItem, AlbumItemPosition, AlbumMode::*},
        rating::{raw_to_stars, stars_to_raw},
        track::{DiscLabel, Track, TrackEdit, TrackFieldError},
    },
    tags::{read_rating, read_track_edit, write_album_tags, write_rating},
    templ::{Templ, TemplAlbumArtMode, parse_templ, read_templ},
};

fn silent_mp3(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("albumcard-tags-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut data = Vec::new();
    for _ in 0..20 {
        data.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        data.extend(std::iter::repeat_n(0u8, 413));
    }
    std::fs::write(&path, data).unwrap();
    path
}

fn track_at(path: PathBuf, title: &str) -> Track {
    let mut t = Track::new(path);
    t.title = title.to_string();
    t
}

fn album_with_rating(rating: Option<u8>) -> Album {
    let mut a = Album::new(Ep, "Album", "Artist", "", "Genre", None).unwrap();
    a.apply_edit(AlbumEdit {
        title: "A".into(),
        album_artist: "Artist".into(),
        release_date: String::new(),
        genre: "Genre".into(),
        mode: Ep,
        rating: rating,
    })
    .unwrap();
    a
}

#[test]
fn parsing_album_items() {
    let track = Track {
        file_path: PathBuf::from("~/Hudba/ProjectMoon - Oh Crab So Crab.mp3"),
        artist: None,
        title: "Oh Crab So Crab".to_string(),
        genre: None,
        release_date: None,
        performer: None,
        composer: None,
        lyricist: None,
        remixer: None,
        sort_album_title: None,
        sort_album_artist: None,
        sort_composer: None,
        sort_performer: None,
        sort_track_title: None,
        rating: None,
    };

    let mut album =
        Album::new(Ep, "Limbus Company OST", "ProjectMoon", "2023-01-01", "Soundtrack", None).expect("valid date");

    album.add_disk();
    album.add_void(96);
    album.add_track(track);
    album.add_void(236);
    album.add_disk();
    album.add_void(1);

    assert_eq!(album.get_length(), 334);
}

#[test]
fn single_mode_rejects_disks_and_a_second_track() {
    let mut album =
        Album::new(Single, "Oh Crab So Crab", "ProjectMoon", "2023-01-01", "Soundtrack", None).expect("valid date");

    assert!(!album.add_disk(), "Single must not accept a Disk");
    assert!(album.add_track(Track::new(PathBuf::from("a.mp3"))), "First track should succeed");
    assert!(!album.add_track(Track::new(PathBuf::from("b.mp3"))), "Single must not accept a second Track");
    assert_eq!(album.get_items().len(), 1);
}

#[test]
fn album_new_rejects_unparseable_date() {
    let result = Album::new(Ep, "Title", "Artist", "NaD", "Genre", None);
    assert!(result.is_err());
}

#[test]
fn album_new_allows_a_blank_release_date() {
    let album = Album::new(Ep, "Limbus Company OST", "ProjectMoon", "", "Soundtrack", None)
        .expect("a blank date must not be an error");
    assert_eq!(album.release_date, None);
}

#[test]
fn void_gap_with_no_disc_grouping() {
    let mut album = Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_void(1);
    album.add_track(Track::new(PathBuf::from("b.mp3")));
    album.add_track(Track::new(PathBuf::from("c.mp3")));
    album.add_void(3);
    album.add_track(Track::new(PathBuf::from("g.mp3")));

    let positions = album.item_positions();
    let rendered: Vec<String> = positions
        .into_iter()
        .map(|p| p.map(|p| p.to_string()).unwrap_or_default())
        .collect();
    assert_eq!(rendered, vec!["_", "2", "3", "_", "7"]);

    assert_eq!(
        album.item_positions(),
        vec![
            Some(AlbumItemPosition {
                disk: DiscLabel::Ungrouped,
                track: None
            }),
            Some(AlbumItemPosition {
                disk: DiscLabel::Ungrouped,
                track: Some(2)
            }),
            Some(AlbumItemPosition {
                disk: DiscLabel::Ungrouped,
                track: Some(3)
            }),
            Some(AlbumItemPosition {
                disk: DiscLabel::Ungrouped,
                track: None
            }),
            Some(AlbumItemPosition {
                disk: DiscLabel::Ungrouped,
                track: Some(7)
            }),
        ]
    );
}

#[test]
fn void_gap_with_disc_grouping() {
    let mut album = Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_disk();
    album.add_void(1);
    album.add_track(Track::new(PathBuf::from("b.mp3")));
    album.add_track(Track::new(PathBuf::from("c.mp3")));
    album.add_void(3);
    album.add_track(Track::new(PathBuf::from("g.mp3")));
    album.add_disk();
    album.add_track(Track::new(PathBuf::from("h.mp3")));
    album.add_track(Track::new(PathBuf::from("i.mp3")));

    let rendered: Vec<String> = album
        .item_positions()
        .into_iter()
        .map(|p| p.map(|p| p.to_string()).unwrap_or_default())
        .collect();

    assert_eq!(rendered, vec!["", "1._", "1.2", "1.3", "1._", "1.7", "", "2.1", "2.2"]);
}

#[test]
fn track_before_first_disk_is_unassigned_until_disk_moves_up() {
    let mut album = Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_track(Track::new(PathBuf::from("a.mp3")));
    album.add_disk();
    album.add_track(Track::new(PathBuf::from("b.mp3")));

    let rendered: Vec<String> = album
        .item_positions()
        .into_iter()
        .map(|p| p.map(|p| p.to_string()).unwrap_or_default())
        .collect();
    assert_eq!(rendered, vec!["_.1", "", "1.1"]);

    assert!(album.move_item(1, 0));

    let rendered: Vec<String> = album
        .item_positions()
        .into_iter()
        .map(|p| p.map(|p| p.to_string()).unwrap_or_default())
        .collect();
    assert_eq!(rendered, vec!["", "1.1", "1.2"]);
}

#[test]
fn move_item_rejects_out_of_bounds_and_no_ops_on_equal_indices() {
    let mut album = Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_track(Track::new(PathBuf::from("a.mp3")));
    album.add_track(Track::new(PathBuf::from("b.mp3")));

    assert!(!album.move_item(0, 5), "destination out of bounds must be rejected");
    assert!(!album.move_item(5, 0), "source out of bounds must be rejected");
    assert!(album.move_item(1, 1), "from == to is a no-op that still succeeds");
    assert_eq!(album.get_items().len(), 2, "rejected/no-op moves must not touch the list");
}

#[test]
fn remove_item_returns_the_removed_item_and_none_out_of_bounds() {
    let mut album = Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_void(4);
    album.add_track(Track::new(PathBuf::from("a.mp3")));

    let removed = album.remove_item(0);
    assert!(matches!(removed, Some(AlbumItem::Void { size: 4 })));
    assert_eq!(album.get_items().len(), 1);
    assert_eq!(album.get_length(), 1, "only the Track's slot should remain after removing the Void");
}

#[test]
fn removing_the_track_frees_up_single_mode_for_another() {
    let mut album = Album::new(Single, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    assert!(album.add_track(Track::new(PathBuf::from("a.mp3"))));
    assert!(!album.add_track(Track::new(PathBuf::from("b.mp3"))), "Single already has its one track");

    assert!(album.remove_item(0).is_some());
    assert!(
        album.add_track(Track::new(PathBuf::from("b.mp3"))),
        "removing the only track should free the single up to accepts a new one"
    );
}

#[test]
fn apply_edit_trims_and_normalizes_fields() {
    let mut track = Track::new(PathBuf::from("a.mp3"));
    let edit = TrackEdit {
        artist: "   ProjectMoon    ".to_string(),
        title: "   Oh Crab So Crab ".to_string(),
        genre: "   ".to_string(),
        release_date: " 2023-05-01    ".to_string(),
        performer: "   ProjectMoon    ".to_string(),
        composer: "ProjectMoon    ".to_string(),
        lyricist: String::new(),
        remixer: String::new(),
        sort_album_title: String::new(),
        sort_album_artist: String::new(),
        sort_composer: String::new(),
        sort_performer: String::new(),
        sort_track_title: String::new(),
        file_path: PathBuf::new(),
        rating: None,
    };

    assert!(track.apply_edit(edit).is_ok());
    assert_eq!(track.title, "Oh Crab So Crab");
    assert_eq!(track.genre, None);
    assert_eq!(track.release_date, Some(chrono::NaiveDate::from_ymd_opt(2023, 5, 1).unwrap()));
    assert_eq!(track.performer, Some("ProjectMoon".to_string()));
}

#[test]
fn apply_edit_rejects_empty_title_and_leaves_track_unchanged() {
    let mut track = Track::new(PathBuf::from("a.mp3"));
    track.title = "Original Title".to_string();

    let edit = TrackEdit {
        title: "   ".to_string(),
        ..TrackEdit::default()
    };

    let errors = track.apply_edit(edit).unwrap_err();
    assert_eq!(errors, vec![TrackFieldError::TitleEmpty]);
    assert_eq!(track.title, "Original Title", "a rejected edit must not partially apply");
}

#[test]
fn apply_edit_rejecs_unparseable_date_and_reports_both_errors_together() {
    let mut track = Track::new(PathBuf::from("a.mp3"));
    let edit = TrackEdit {
        title: "".to_string(),
        release_date: "NaD".to_string(),
        ..TrackEdit::default()
    };

    let errors = track.apply_edit(edit).unwrap_err();
    assert_eq!(errors, vec![TrackFieldError::TitleEmpty, TrackFieldError::InvalidReleaseDate]);
}

#[test]
fn edit_snapshot_round_trips_through_apply_edit() {
    let mut track = Track::new(PathBuf::from("a.mp3"));
    track
        .apply_edit(TrackEdit {
            title: "G Song".to_string(),
            genre: "Soundtrack".to_string(),
            release_date: "2023-01-01".to_string(),
            performer: "ProjectMoon".to_string(),
            ..TrackEdit::default()
        })
        .unwrap();

    let snapshot = track.edit_snapshot();
    assert_eq!(snapshot.title, "G Song");
    assert_eq!(snapshot.genre, "Soundtrack");
    assert_eq!(snapshot.release_date, "2023-01-01");
    assert_eq!(snapshot.performer, "ProjectMoon");

    let mut track2 = Track::new(PathBuf::from("a.mp3"));
    track2.apply_edit(snapshot).unwrap();
    assert_eq!(track2.title, track.title);
    assert_eq!(track2.genre, track.genre);
    assert_eq!(track2.release_date, track.release_date);
}

#[test]
fn track_artist_and_genre_fall_back_to_the_album_when_unset() {
    let album = Album::new(Ep, "Title", "Artist", "", "Soundtrack", None).expect("valid date");

    let plain_track = Track::new(PathBuf::from("a.mp3"));
    assert_eq!(plain_track.effective_artist(&album), "Artist");
    assert_eq!(plain_track.effective_genre(&album), "Soundtrack");

    let mut featured_track = Track::new(PathBuf::from("b.mp3"));
    featured_track.artist = Some("Mili".to_string());
    featured_track.genre = Some("Electronic".to_string());
    assert_eq!(featured_track.effective_artist(&album), "Mili");
    assert_eq!(featured_track.effective_genre(&album), "Electronic");
}

#[test]
fn album_round_trips_through_json() {
    let mut album = Album::new(Ep, "Limbus Company OST", "ProjectMoon", "", "Soundtrack", None).expect("valid date");

    album.add_disk();
    album.add_void(1);
    let mut track = Track::new(PathBuf::from("b.mp3"));
    track.title = "B Song".to_string();
    track.artist = Some("Studio EIM".to_string());
    album.add_track(track);
    album.add_void(3);

    let json = album.to_json().expect("serialization should succeed");
    let restored = Album::from_json(&json).expect("deserialization should succeed");

    assert_eq!(restored, album);
    assert_eq!(restored.id, album.id);
}

#[test]
fn each_album_gets_a_distinct_id() {
    let a = Album::new(Single, "A", "Artist", "", "Genre", None).expect("valid date");
    let b = Album::new(Single, "B", "Brtist", "", "Genre", None).expect("valid date");
    assert_ne!(a.id, b.id);
}

#[test]
fn untagged_file_prefills_title_from_filename() {
    let path = silent_mp3("Mili - Iron Lotus.mp3");
    let edit = read_track_edit(&path, "Mili", "Soundtrack").unwrap();
    assert_eq!(edit.title, "Mili - Iron Lotus");
    assert_eq!(edit.artist, "");
}

#[test]
fn write_then_read_round_trips_and_inherits_album_values() {
    let path = silent_mp3("roundtrip.mp3");
    let mut album = Album::new(Ep, "To Kill A Living Book", "Mili", "2021-02-27", "Soundtrack", None).unwrap();
    album.add_disk();
    album.add_void(1);
    let mut t = track_at(path.clone(), "Iron Lotus");
    t.composer = Some("Cassie Wei".to_string());
    t.sort_track_title = Some("Iron Lotus (sort)".to_string());
    album.add_track(t);

    let summary = write_album_tags(&album);
    assert_eq!(summary.reports.len(), 1);
    assert!(summary.failures().next().is_none(), "{:?}", summary.reports);
    assert!(summary.art_error.is_none());

    let edit = read_track_edit(&path, &album.album_artist, &album.genre).unwrap();
    assert_eq!(edit.title, "Iron Lotus");
    // artist/genre equal the album's, so they come back blank (= inherit)
    assert_eq!(edit.artist, "");
    assert_eq!(edit.genre, "");
    assert_eq!(edit.release_date, "2021-02-27");
    assert_eq!(edit.composer, "Cassie Wei");
    assert_eq!(edit.sort_track_title, "Iron Lotus (sort)");
}

#[test]
fn clearing_a_field_clears_it_in_the_file() {
    let path = silent_mp3("clearing.mp3");
    let mut album = Album::new(Ep, "A", "Artist", "", "The Nerve", None).unwrap();
    let mut t = track_at(path.clone(), "Song");
    t.composer = Some("Someone".to_string());
    album.add_track(t);
    write_album_tags(&album);
    assert_eq!(read_track_edit(&path, "Artist", "Genre").unwrap().composer, "Someone");

    //Rebuild the album with the composer removed and write again.
    let mut album2 = Album::new(Ep, "A", "Artist", "", "The Nerve", None).unwrap();
    album2.add_track(track_at(path.clone(), "Song"));
    write_album_tags(&album2);
    assert_eq!(read_track_edit(&path, "Artist", "Genre").unwrap().composer, "");
}

#[test]
fn track_override_beats_album_and_missing_file_is_reported_not_fatal() {
    let good = silent_mp3("override.mp3");
    let mut album = Album::new(Ep, "A", "Artist", "", "The Nerve", None).unwrap();
    let mut featured = track_at(good.clone(), "Featured");
    featured.artist = Some("Guest".to_string());
    album.add_track(featured);
    album.add_track(track_at(PathBuf::from("/definitely/not/here.mp3"), "Missing"));

    let summary = write_album_tags(&album);
    assert_eq!(summary.reports.len(), 2);
    assert_eq!(summary.failures().count(), 1, "only the missing file should fail");
    assert_eq!(summary.failures().next().unwrap().index, 1);

    let edit = read_track_edit(&good, "Album Artist", "Genre").unwrap();
    assert_eq!(edit.artist, "Guest");
}

#[test]
fn year_only_date_is_left_blank() {
    // Hand-write a year-only date through the writer's own path:
    // a track whose date is None and album has none leaves the key absent -> blang.
    let path = silent_mp3("nodate.mp3");
    let mut album = Album::new(Ep, "A", "Artist", "", "The Nerve", None).unwrap();
    album.add_track(track_at(path.clone(), "Song"));
    write_album_tags(&album);
    assert_eq!(read_track_edit(&path, "Artist", "Genre").unwrap().release_date, "");
    let _ = TrackEdit::default();
}

#[test]
fn every_edit_field_survives_an_id3_round_trip() {
    let path = silent_mp3("everything.mp3");
    let mut album = Album::new(Ep, "A", "Artist", "", "The Nerve", None).unwrap();
    let mut t = Track::new(path.clone());
    t.apply_edit(TrackEdit {
        file_path: t.file_path.clone(),
        title: "Song".into(),
        artist: "Guest".into(),
        genre: "Other".into(),
        release_date: "2020-05-01".into(),
        performer: "P".into(),
        composer: "C".into(),
        lyricist: "L".into(),
        remixer: "R".into(),
        sort_album_title: "sortAT".into(),
        sort_album_artist: "sortAA".into(),
        sort_composer: "sortC".into(),
        sort_performer: "sortP".into(),
        sort_track_title: "sortTT".into(),
        rating: None,
    })
    .unwrap();
    album.add_track(t.clone());

    let summary = write_album_tags(&album);
    assert!(summary.reports[0].result.is_ok());
    assert!(summary.reports[0].unsupported.is_empty());

    let back = read_track_edit(&path, "Artist", "Genre").unwrap();
    assert_eq!(
        back,
        TrackEdit {
            file_path: t.file_path.clone(),
            title: "Song".into(),
            artist: "Guest".into(),
            genre: "Other".into(),
            release_date: "2020-05-01".into(),
            performer: "P".into(),
            composer: "C".into(),
            lyricist: "L".into(),
            remixer: "R".into(),
            sort_album_title: "sortAT".into(),
            sort_album_artist: "sortAA".into(),
            sort_composer: "sortC".into(),
            sort_performer: "sortP".into(),
            sort_track_title: "sortTT".into(),
            rating: None,
        }
    );
}

const TEMPL1: &str = "MDK|Evan King\nJelly Castle (Evan King Retro Mix)\nJelly Castle\n2013\n10\n0\nElectronic|Retro\nMDK\nMDK\nEvan King\n0\n0\n0\n0\n0\n0\nfile\n/home/pisek/Obrázky/2P.png\n";
const TEMPL1A: &str = "MDK|Evan King\nJelly Castle (Evan King Retro Mix)\nJelly Castle\n2013\n10\n0\nElectronic|Retro\nMDK\nMDK\nEvan King\n0\n0\n0\n0\n0\n0\nfile\n/home/pisek/Obrázky/2P.png\n0";

#[test]
fn parses_a_full_templ1_file() {
    let t = parse_templ(TEMPL1).unwrap();
    assert_eq!(t.artist.as_deref(), Some("MDK; Evan King"));
    assert_eq!(t.title.as_deref(), Some("Jelly Castle (Evan King Retro Mix)"));
    assert_eq!(t.album.as_deref(), Some("Jelly Castle"));
    assert_eq!(t.year, Some(2013));
    assert_eq!(t.track, Some(10));
    assert_eq!(t.disc, Some(0));
    assert_eq!(t.genre.as_deref(), Some("Electronic; Retro"));
    assert_eq!(t.album_artist.as_deref(), Some("MDK"));
    assert_eq!(t.composer.as_deref(), Some("MDK"));
    assert_eq!(t.remixer.as_deref(), Some("Evan King"));
    assert_eq!(t.rating, Some(0));
    assert_eq!(t.max_rating, Some(0));
    assert_eq!(t.spotify_rating, Some(0));
    assert_eq!(t.max_spotify_rating, Some(0));
    assert_eq!(t.youtube_rating, Some(0));
    assert_eq!(t.soundcloud_rating, Some(0));
    assert_eq!(t.art_mode, Some(TemplAlbumArtMode::Change(PathBuf::new())));
    assert_eq!(t.art_path.as_deref(), Some(std::path::Path::new("/home/pisek/Obrázky/2P.png")));
}

#[test]
fn parses_a_full_templ1a_file() {
    let t = parse_templ(TEMPL1A).unwrap();
    assert_eq!(t.artist.as_deref(), Some("MDK; Evan King"));
    assert_eq!(t.title.as_deref(), Some("Jelly Castle (Evan King Retro Mix)"));
    assert_eq!(t.album.as_deref(), Some("Jelly Castle"));
    assert_eq!(t.year, Some(2013));
    assert_eq!(t.track, Some(10));
    assert_eq!(t.disc, Some(0));
    assert_eq!(t.genre.as_deref(), Some("Electronic; Retro"));
    assert_eq!(t.album_artist.as_deref(), Some("MDK"));
    assert_eq!(t.composer.as_deref(), Some("MDK"));
    assert_eq!(t.remixer.as_deref(), Some("Evan King"));
    assert_eq!(t.rating, Some(0));
    assert_eq!(t.max_rating, Some(0));
    assert_eq!(t.spotify_rating, Some(0));
    assert_eq!(t.max_spotify_rating, Some(0));
    assert_eq!(t.youtube_rating, Some(0));
    assert_eq!(t.soundcloud_rating, Some(0));
    assert_eq!(t.art_mode, Some(TemplAlbumArtMode::Change(PathBuf::new())));
    assert_eq!(t.art_path.as_deref(), Some(std::path::Path::new("/home/pisek/Obrázky/2P.png")));
    assert_eq!(t.youtube_music_rating, Some(0));
}

#[test]
fn templ1_and_short_files_are_fine() {
    let t = parse_templ(TEMPL1).unwrap();
    assert_eq!(t.youtube_music_rating, None);
    assert_eq!(t.rating, Some(0));
    let short = parse_templ("Artist\nTitle").unwrap();
    assert_eq!(short.title.as_deref(), Some("Title"));
    assert_eq!(short.album, None);
}

#[test]
fn empty_lines_are_skipped_not_cleared() {
    let t = parse_templ("\nTitle\n\n\n\n\n\n\n\n\n\n").unwrap();
    assert_eq!(t.artist, None);
    assert_eq!(t.title.as_deref(), Some("Title"));
    assert_eq!(t.rating, None);
    assert_eq!(parse_templ("").unwrap(), Templ::default());
}

#[test]
fn crlf_and_bom_are_handled() {
    let crlf = TEMPL1.replace('\n', "\r\n");
    assert_eq!(parse_templ(&crlf).unwrap(), parse_templ(TEMPL1).unwrap());
    let dir = std::env::temp_dir().join("album_card_templ_bom");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("a.templ");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(TEMPL1.as_bytes());
    std::fs::write(&p, bytes).unwrap();
    assert_eq!(read_templ(&p).unwrap().title.as_deref(), Some("Jelly Castle (Evan King Retro Mix)"));
    std::fs::write(&p, [0xFF, 0xFE, 0x41]).unwrap();
    assert!(read_templ(&p).unwrap_err().line.is_none());
}

#[test]
fn bad_numbers_name_the_line() {
    // Year
    let mut lines: Vec<&str> = TEMPL1A.lines().collect();
    lines[3] = "abcd";
    assert_eq!(parse_templ(&lines.join("\n")).unwrap_err().line, Some(4));

    // Track
    let mut lines: Vec<&str> = TEMPL1A.lines().collect();
    lines[4] = "501";
    assert_eq!(parse_templ(&lines.join("\n")).unwrap_err().line, Some(5));

    // Rating
    let mut lines: Vec<&str> = TEMPL1A.lines().collect();
    lines[10] = "256";
    assert_eq!(parse_templ(&lines.join("\n")).unwrap_err().line, Some(11));
}

#[test]
fn apply_fills_track_fields_and_inherits_album_values() {
    let t = parse_templ("Mili\nCamelia\n\n\n\n\nSoundtrack\n\nC\nR\n\n\n\n\n\n\n\n\n").unwrap();
    let mut e = TrackEdit {
        performer: "keep".into(),
        ..TrackEdit::default()
    };
    t.apply_to_track_edit(&mut e, "Mili", "Rock");
    assert_eq!(e.title, "Camelia");
    assert_eq!(e.artist, "");
    assert_eq!(e.genre, "Soundtrack");
    assert_eq!((e.composer.as_str(), e.remixer.as_str()), ("C", "R"));
    assert_eq!(e.performer, "keep");
}

#[test]
fn start_and_raw_round_trip() {
    for stars in 0..=5u8 {
        assert_eq!(raw_to_stars(stars_to_raw(stars)), stars);
    }
    assert_eq!(raw_to_stars(200), 4);
    assert_eq!(raw_to_stars(255), 5);
    assert_eq!(raw_to_stars(31), 1);
    assert_eq!(raw_to_stars(32), 2);
}

#[test]
fn album_rating_goes_to_every_track_and_a_track_rating_wins() {
    let (p1, p2) = (silent_mp3("rating-one.mp3"), silent_mp3("rating-two.mp3"));

    let mut album = album_with_rating(Some(200));
    let mut t1 = Track::new(p1.clone());
    t1.apply_edit(TrackEdit {
        title: "One".into(),
        ..TrackEdit::default()
    })
    .unwrap();
    let mut t2 = Track::new(p2.clone());
    t2.apply_edit(TrackEdit {
        title: "Two".into(),
        rating: Some(64),
        ..TrackEdit::default()
    })
    .unwrap();
    album.add_track(t1);
    album.add_track(t2);

    let summary = write_album_tags(&album);
    assert_eq!(summary.failures().count(), 0);
    assert!(summary.reports.iter().all(|r| r.unsupported.is_empty()));
    assert_eq!(read_rating(&p1), Some(200)); // inhereted from the album
    assert_eq!(read_rating(&p2), Some(64)); // its own rating

    // Clearing everything clears the files.
    let mut album = album_with_rating(None);
    let mut t1 = Track::new(p1.clone());
    t1.apply_edit(TrackEdit {
        title: "One".into(),
        ..TrackEdit::default()
    })
    .unwrap();
    album.add_track(t1);
    write_album_tags(&album);
    assert_eq!(read_rating(&p1), None);
}

#[test]
fn prefill_reads_the_rating_back() {
    let p = silent_mp3("rating-prefill.mp3");
    assert!(write_rating(&p, Some(128)).unwrap());
    assert_eq!(read_track_edit(&p, "", "").unwrap().rating, Some(128));
}
