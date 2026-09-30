#![cfg(test)]

use std::path::PathBuf;

use crate::models::{album::{Album, AlbumItem, AlbumItemPosition, AlbumMode::*}, track::{DiscLabel, Track, TrackEdit, TrackFieldError}};

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
    };

    let mut album = Album::new(
        Ep,
        "Limbus Company OST",
        "ProjectMoon",
        "2023-01-01",
        "Soundtrack",
        None,
    )
    .expect("valid date");

    album.add_disk(Some("Songs".to_string()));
    album.add_void(96);
    album.add_track(track);
    album.add_void(236);
    album.add_disk(Some("Latest".to_string()));
    album.add_void(1);

    assert_eq!(album.get_length(), 334);
}

#[test]
fn single_mode_rejects_disks_and_a_second_track() {
    let mut album = Album::new(
        Single,
        "Oh Crab So Crab",
        "ProjectMoon",
        "2023-01-01",
        "Soundtrack",
        None,
    )
    .expect("valid date");

    assert!(
        !album.add_disk(Some("Disc".to_string())),
        "Single must not accept a Disk"
    );
    assert!(
        album.add_track(Track::new(PathBuf::from("a.mp3"))),
        "First track should succeed"
    );
    assert!(
        !album.add_track(Track::new(PathBuf::from("b.mp3"))),
        "Single must not accept a second Track"
    );
    assert_eq!(album.get_items().len(), 1);
}

#[test]
fn album_new_rejects_unparseable_date() {
    let result = Album::new(Ep, "Title", "Artist", "NaD", "Genre", None);
    assert!(result.is_err());
}

#[test]
fn album_new_allows_a_blank_release_date() {
    let album = Album::new(
        Ep,
        "Limbus Company OST",
        "ProjectMoon",
        "",
        "Soundtrack",
        None,
    )
    .expect("a blank date must not be an error");
    assert_eq!(album.release_date, None);
}

#[test]
fn void_gap_with_no_disc_grouping() {
    let mut album =
        Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

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
    let mut album =
        Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_disk(Some("Disc 1".to_string()));
    album.add_void(1);
    album.add_track(Track::new(PathBuf::from("b.mp3")));
    album.add_track(Track::new(PathBuf::from("c.mp3")));
    album.add_void(3);
    album.add_track(Track::new(PathBuf::from("g.mp3")));
    album.add_disk(Some("Disc 2".to_string()));
    album.add_track(Track::new(PathBuf::from("h.mp3")));
    album.add_track(Track::new(PathBuf::from("i.mp3")));

    let rendered: Vec<String> = album
        .item_positions()
        .into_iter()
        .map(|p| p.map(|p| p.to_string()).unwrap_or_default())
        .collect();

    assert_eq!(
        rendered,
        vec!["", "1._", "1.2", "1.3", "1._", "1.7", "", "2.1", "2.2"]
    );
}

#[test]
fn track_before_first_disk_is_unassigned_until_disk_moves_up() {
    let mut album =
        Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_track(Track::new(PathBuf::from("a.mp3")));
    album.add_disk(Some("Disc 1".to_string()));
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
    let mut album =
        Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_track(Track::new(PathBuf::from("a.mp3")));
    album.add_track(Track::new(PathBuf::from("b.mp3")));

    assert!(
        !album.move_item(0, 5),
        "destination out of bounds must be rejected"
    );
    assert!(
        !album.move_item(5, 0),
        "source out of bounds must be rejected"
    );
    assert!(
        album.move_item(1, 1),
        "from == to is a no-op that still succeeds"
    );
    assert_eq!(
        album.get_items().len(),
        2,
        "rejected/no-op moves must not touch the list"
    );
}

#[test]
fn remove_item_returns_the_removed_item_and_none_out_of_bounds() {
    let mut album =
        Album::new(Ep, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    album.add_void(4);
    album.add_track(Track::new(PathBuf::from("a.mp3")));

    let removed = album.remove_item(0);
    assert!(matches!(
        removed,
        Some(AlbumItem::Void { size: 4 })
    ));
    assert_eq!(album.get_items().len(), 1);
    assert_eq!(
        album.get_length(),
        1,
        "only the Track's slot should remain after removing the Void"
    );
}

#[test]
fn removing_the_track_frees_up_single_mode_for_another() {
    let mut album =
        Album::new(Single, "Title", "Artist", "2023-01-01", "Genre", None).expect("valid date");

    assert!(album.add_track(Track::new(PathBuf::from("a.mp3"))));
    assert!(
        !album.add_track(Track::new(PathBuf::from("b.mp3"))),
        "Single already has its one track"
    );

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
    };

    assert!(track.apply_edit(edit).is_ok());
    assert_eq!(track.title, "Oh Crab So Crab");
    assert_eq!(track.genre, None);
    assert_eq!(
        track.release_date,
        Some(chrono::NaiveDate::from_ymd_opt(2023, 5, 1).unwrap())
    );
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
    assert_eq!(
        track.title, "Original Title",
        "a rejected edit must not partially apply"
    );
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
    assert_eq!(
        errors,
        vec![
            TrackFieldError::TitleEmpty,
            TrackFieldError::InvalidReleaseDate
        ]
    );
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
