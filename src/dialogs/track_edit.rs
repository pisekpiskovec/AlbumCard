use adw::prelude::*;
use chrono::{Datelike, NaiveDate};
use relm4::prelude::*;

use crate::models::track::{Track, TrackEdit, TrackFieldError};

#[derive(Debug)]
pub enum TrackEditMsg {
    TitleChanged(String),
    GenreChanged(String),
    ReleaseDateChanged(String),
    ArtistChanged(String),
    PerformerChanged(String),
    ComposerChanged(String),
    LyricistChanged(String),
    RemixerChanged(String),
    Save,
    Cancel,
}

#[derive(Debug)]
pub enum TrackEditOutput {
    Saved { index: usize, edit: TrackEdit },
    Cancelled,
}

pub struct TrackEditDialog {
    index: usize,
    edit: TrackEdit,
}

pub struct TrackEditWidgets {
    release_date_button: gtk::MenuButton,
    toast_overlay: adw::ToastOverlay,
}

fn release_date_label(value: &str) -> &str {
    match value.is_empty() {
        true => "Not set",
        false => value,
    }
}

impl Component for TrackEditDialog {
    type Input = TrackEditMsg;
    type Output = TrackEditOutput;
    type Init = (usize, TrackEdit);
    type Root = adw::Dialog;
    type Widgets = TrackEditWidgets;
    type CommandOutput = ();

    fn init_root() -> Self::Root {
        adw::Dialog::builder()
            .title("Edit Track")
            .content_height(560)
            .content_width(480)
            .build()
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let (index, edit) = init;

        {
            let sender = sender.clone();
            root.connect_closed(move |_| sender.input(TrackEditMsg::Cancel));
        }

        let header = adw::HeaderBar::new();
        let save_button = gtk::Button::with_label("Save");
        save_button.add_css_class("suggested-action");
        {
            let sender = sender.clone();
            save_button.connect_clicked(move |_| sender.input(TrackEditMsg::Save));
        }
        header.pack_end(&save_button);

        let group = adw::PreferencesGroup::new();

        macro_rules! entry_row {
            ($title:literal, $field:ident, $variant:ident) => {{
                let row = adw::EntryRow::builder()
                    .title($title)
                    .text(edit.$field.as_str())
                    .build();
                let sender = sender.clone();
                row.connect_changed(move |row| {
                    sender.input(TrackEditMsg::$variant(row.text().to_string()));
                });
                group.add(&row);
            }};
        }

        entry_row!("Title", title, TitleChanged);
        entry_row!("Genre (blank to use album genre)", genre, GenreChanged);

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
            let button = release_date_button.clone();
            calendar.connect_day_selected(move |cal| {
                let date = cal.date();
                let formatted = format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day_of_month());
                sender.input(TrackEditMsg::ReleaseDateChanged(formatted.clone()));
                button.set_label(&formatted);
                popover.popdown();
            });
        }
        {
            let sender = sender.clone();
            let popover = calendar_popover.clone();
            clear_date_button.connect_clicked(move |_| {
                sender.input(TrackEditMsg::ReleaseDateChanged(String::new()));
                popover.popdown();
            });
        }

        let release_date_row = adw::ActionRow::builder()
            .title("Release date")
            .activatable_widget(&release_date_button)
            .build();
        release_date_row.add_suffix(&release_date_button);
        group.add(&release_date_row);

        entry_row!("Artist (blank to use album artist)", artist, ArtistChanged);
        entry_row!("Performer", performer, PerformerChanged);
        entry_row!("Composer", composer, ComposerChanged);
        entry_row!("Lyricist", lyricist, LyricistChanged);
        entry_row!("Remixer", remixer, RemixerChanged);

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

        let model = TrackEditDialog { index, edit };
        let widgets = TrackEditWidgets {
            release_date_button,
            toast_overlay,
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
            TrackEditMsg::TitleChanged(v) => self.edit.title = v,
            TrackEditMsg::GenreChanged(v) => self.edit.genre = v,
            TrackEditMsg::ReleaseDateChanged(v) => self.edit.release_date = v,
            TrackEditMsg::ArtistChanged(v) => self.edit.artist = v,
            TrackEditMsg::PerformerChanged(v) => self.edit.performer = v,
            TrackEditMsg::ComposerChanged(v) => self.edit.composer = v,
            TrackEditMsg::LyricistChanged(v) => self.edit.lyricist = v,
            TrackEditMsg::RemixerChanged(v) => self.edit.remixer = v,
            TrackEditMsg::Save => {
                let mut scratch = Track::default();
                match scratch.apply_edit(self.edit.clone()) {
                    Ok(()) => {
                        sender
                            .output(TrackEditOutput::Saved {
                                index: self.index,
                                edit: self.edit.clone(),
                            })
                            .ok();
                        return;
                    }
                    Err(errors) => {
                        let text: String = errors
                            .iter()
                            .map(|e: &TrackFieldError| e.to_string())
                            .collect::<Vec<_>>()
                            .join("\n");
                        widgets
                            .toast_overlay
                            .add_toast(adw::Toast::builder().title(text).build());
                    }
                }
            }
            TrackEditMsg::Cancel => {
                sender.output(TrackEditOutput::Cancelled).ok();
            }
        }
    }
}
