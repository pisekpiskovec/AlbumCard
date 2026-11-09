use std::path::PathBuf;

use adw::prelude::*;
use chrono::{Datelike, NaiveDate};
use relm4::prelude::*;

use crate::{models::{
    album::{Album, AlbumEdit, AlbumMode},
    track::Track,
}, tags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumEditPurpose {
    New,
    Edit,
}

#[derive(Debug)]
pub enum AlbumEditMsg {
    TitleChanged(String),
    ArtistChanged(String),
    ReleaseDateChanged(String),
    GenreChanged(String),
    ModeChanged(AlbumMode),
    AddGenre(String),
    Save,
    Cancel,
}

#[derive(Debug)]
pub enum AlbumEditOutput {
    Saved { purpose: AlbumEditPurpose, edit: AlbumEdit },
    Cancelled,
}

pub struct AlbumEditDialog {
    purpose: AlbumEditPurpose,
    edit: AlbumEdit,
    could_be_single: bool,
}

pub struct AlbumEditWidgets {
    release_date_button: gtk::MenuButton,
    toast_overlay: adw::ToastOverlay,
    genre_row: adw::EntryRow,
}

fn release_date_label(value: &str) -> &str {
    match value.is_empty() {
        true => "Not set",
        false => value,
    }
}

impl Component for AlbumEditDialog {
    type Input = AlbumEditMsg;
    type Output = AlbumEditOutput;
    type Init = (AlbumEditPurpose, AlbumEdit, bool);
    type Root = adw::Dialog;
    type Widgets = AlbumEditWidgets;
    type CommandOutput = ();

    fn init_root() -> Self::Root {
        adw::Dialog::builder()
            .title("Edit Track")
            .content_height(560)
            .content_width(480)
            .build()
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (purpose, edit, could_be_single) = init;

        root.set_title(match purpose {
            AlbumEditPurpose::New => "New Album",
            AlbumEditPurpose::Edit => "Edit Album",
        });

        {
            let sender = sender.clone();
            root.connect_closed(move |_| sender.input(AlbumEditMsg::Cancel));
        }

        let header = adw::HeaderBar::new();
        let save_button = gtk::Button::with_label(match purpose {
            AlbumEditPurpose::New => "Create",
            AlbumEditPurpose::Edit => "Save",
        });
        save_button.add_css_class("suggested-action");
        {
            let sender = sender.clone();
            save_button.connect_clicked(move |_| sender.input(AlbumEditMsg::Save));
        }
        header.pack_end(&save_button);

        let group = adw::PreferencesGroup::new();

        // Mode
        let single_toggle = gtk::ToggleButton::builder().label("Single").build();
        let ep_toggle = gtk::ToggleButton::builder().label("EP").build();
        ep_toggle.set_group(Some(&single_toggle));
        match edit.mode {
            AlbumMode::Single => single_toggle.set_active(true),
            AlbumMode::Ep => ep_toggle.set_active(true),
        }
        let mode_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        mode_box.add_css_class("linked");
        mode_box.set_valign(gtk::Align::Center);
        mode_box.append(&single_toggle);
        mode_box.append(&ep_toggle);
        {
            let sender = sender.clone();
            single_toggle.connect_toggled(move |btn| {
                if btn.is_active() {
                    sender.input(AlbumEditMsg::ModeChanged(AlbumMode::Single));
                }
            });
        }
        {
            let sender = sender.clone();
            ep_toggle.connect_toggled(move |btn| {
                if btn.is_active() {
                    sender.input(AlbumEditMsg::ModeChanged(AlbumMode::Ep));
                }
            });
        }
        let mode_row = adw::ActionRow::builder()
            .title("Album type")
            .activatable_widget(&mode_box)
            .build();
        mode_row.add_suffix(&mode_box);
        group.add(&mode_row);

        macro_rules! entry_row {
            ($title:literal, $field:ident, $variant:ident) => {{
                let row = adw::EntryRow::builder()
                    .title($title)
                    .text(edit.$field.as_str())
                    .build();
                let sender = sender.clone();
                row.connect_changed(move |row| {
                    sender.input(AlbumEditMsg::$variant(row.text().to_string()));
                });
                group.add(&row);
            }};
        }

        entry_row!("Title", title, TitleChanged);
        entry_row!("Artist", album_artist, ArtistChanged);

        // Release date
        let calendar = gtk::Calendar::new();
        if let Ok(parsed) = NaiveDate::parse_from_str(&edit.release_date, "%Y-%m-%d") {
            calendar.set_year(parsed.year());
            calendar.set_month(parsed.month0() as i32);
            calendar.set_day(parsed.day() as i32);
        }

        let clear_date_button = gtk::Button::with_label("Clear date");

        let calendar_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        calendar_popover_box.append(&calendar);
        calendar_popover_box.append(&clear_date_button);
        let calendar_popover = gtk::Popover::builder().child(&calendar_popover_box).build();

        let release_date_button = gtk::MenuButton::builder()
            .label(release_date_label(&edit.release_date))
            .valign(gtk::Align::Center)
            .popover(&calendar_popover)
            .build();
        {
            let sender = sender.clone();
            let popover = calendar_popover.clone();
            calendar.connect_day_selected(move |cal| {
                let date = cal.date();
                let formatted = format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day_of_month());
                sender.input(AlbumEditMsg::ReleaseDateChanged(formatted.clone()));
                popover.popdown();
            });
        }
        {
            let sender = sender.clone();
            let popover = calendar_popover.clone();
            clear_date_button.connect_clicked(move |_| {
                sender.input(AlbumEditMsg::ReleaseDateChanged(String::new()));
                popover.popdown();
            });
        }

        let release_date_row = adw::ActionRow::builder()
            .title("Release date")
            .activatable_widget(&release_date_button)
            .build();
        release_date_row.add_suffix(&release_date_button);
        group.add(&release_date_row);

        let genre_row = adw::EntryRow::builder()
            .title("Genre")
            .text(edit.genre.as_str())
            .build();
        {
            let sender = sender.clone();
            genre_row.connect_changed(move |row| {
                sender.input(AlbumEditMsg::GenreChanged(row.text().to_string()));
            });
        }
        group.add(&genre_row);

        let genre_list_row = adw::ExpanderRow::builder()
            .title("Suggested Genres")
            .subtitle("Pick one or more from these widely recognised genres.")
            .build();
        for genre in get_genres_vector() {
            let add_genre_button = adw::ButtonRow::builder().title(genre).build();
            {
                let sender = sender.clone();
                add_genre_button.connect_activated(move |row| {
                    sender.input(AlbumEditMsg::AddGenre(row.title().to_string()));
                });
            }
            genre_list_row.add_row(&add_genre_button);
        }
        group.add(&genre_list_row);

        let toast_overlay = adw::ToastOverlay::new();
        let scroller = gtk::ScrolledWindow::builder().vexpand(true).child(&group).build();
        toast_overlay.set_child(Some(&scroller));

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        content_box.append(&toast_overlay);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&content_box));

        root.set_child(Some(&toolbar_view));

        let model = AlbumEditDialog {
            purpose,
            edit,
            could_be_single,
        };
        let widgets = AlbumEditWidgets {
            release_date_button,
            toast_overlay,
            genre_row,
        };

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AlbumEditMsg::TitleChanged(v) => self.edit.title = v,
            AlbumEditMsg::GenreChanged(v) => self.edit.genre = v,
            AlbumEditMsg::ReleaseDateChanged(v) => self.edit.release_date = v,
            AlbumEditMsg::ArtistChanged(v) => self.edit.album_artist = v,
            AlbumEditMsg::ModeChanged(v) => self.edit.mode = v,
            AlbumEditMsg::AddGenre(v) => {
                let already_added = tags::split_values(&self.edit.genre);
                if already_added.contains(&v) {
                    return;
                }

                if !already_added.is_empty() {
                    self.edit.genre.push(';');
                }
                self.edit.genre.push_str(&v);
                widgets.genre_row.set_text(&self.edit.genre);
            }
            AlbumEditMsg::Save => {
                // Fake scratch album to apply changes to
                // The limit for singles here is 1 but the real limit is already calculated before so nothing here is breaking
                // Also the values must be the same! If changing, the tree in `self.could_be_single` must be the value + 1 !!!
                let mut scratch =
                    Album::new(AlbumMode::Ep, "x", "x", "", "x", None).expect("static placeholder values always parse");
                if !self.could_be_single {
                    scratch.add_track(Track::new(PathBuf::new()), 1);
                    scratch.add_track(Track::new(PathBuf::new()), 1);
                }

                match scratch.apply_edit(self.edit.clone(), 1) {
                    Ok(()) => {
                        sender
                            .output(AlbumEditOutput::Saved {
                                purpose: self.purpose,
                                edit: self.edit.clone(),
                            })
                            .ok();
                        return;
                    }
                    Err(errors) => {
                        for error in errors {
                            widgets
                                .toast_overlay
                                .add_toast(adw::Toast::builder().title(error).build());
                        }
                    }
                }
            }
            AlbumEditMsg::Cancel => {
                sender.output(AlbumEditOutput::Cancelled).ok();
                return;
            }
        }

        widgets
            .release_date_button
            .set_label(release_date_label(&self.edit.release_date));
    }
}

fn get_genres_vector() -> Vec<String> {
    vec![
        "Blues".to_owned(),
        "Classic rock".to_owned(),
        "Country".to_owned(),
        "Dance".to_owned(),
        "Disco".to_owned(),
        "Funk".to_owned(),
        "Grunge".to_owned(),
        "Hip-hop".to_owned(),
        "Jazz".to_owned(),
        "Metal".to_owned(),
        "New age".to_owned(),
        "Oldies".to_owned(),
        "Pop".to_owned(),
        "Rhythm and blues".to_owned(),
        "Rap".to_owned(),
        "Reggae".to_owned(),
        "Rock".to_owned(),
        "Techno".to_owned(),
        "Industiral".to_owned(),
    ]
}
