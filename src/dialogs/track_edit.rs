use adw::prelude::*;
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
    errors: Vec<TrackFieldError>,
}

pub struct TrackEditWidgets {
    error_label: gtk::Label,
}

impl SimpleComponent for TrackEditDialog {
    type Input = TrackEditMsg;
    type Output = TrackEditOutput;
    type Init = (usize, TrackEdit);
    type Root = adw::Dialog;
    type Widgets = TrackEditWidgets;

    fn init_root() -> Self::Root {
        adw::Dialog::builder()
            .title("Edit Track")
            .content_height(560)
            .content_width(480)
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
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
        entry_row!(
            "Release Date (YYYY-MM-DD, blank to use album date)",
            release_date,
            ReleaseDateChanged
        );
        entry_row!("Artist (blank to use album artist)", artist, ArtistChanged);
        entry_row!("Performer", performer, PerformerChanged);
        entry_row!("Composer", composer, ComposerChanged);
        entry_row!("Lyricist", lyricist, LyricistChanged);
        entry_row!("Remixer", remixer, RemixerChanged);

        let error_label = gtk::Label::builder()
            .css_classes(["error"])
            .wrap(true)
            .halign(gtk::Align::Start)
            .visible(false)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        content_box.append(&group);
        content_box.append(&error_label);

        let scroller = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&content_box)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&scroller));

        root.set_child(Some(&toolbar_view));

        let model = TrackEditDialog {
            index,
            edit,
            errors: Vec::new(),
        };
        let widgets = TrackEditWidgets { error_label };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
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
                    }
                    Err(errors) => self.errors = errors,
                }
            }
            TrackEditMsg::Cancel => {
                sender.output(TrackEditOutput::Cancelled).ok();
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.errors.is_empty() {
            widgets.error_label.set_visible(false);
        } else {
            let text = self
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            widgets.error_label.set_label(&text);
            widgets.error_label.set_visible(true);
        }
    }
}
