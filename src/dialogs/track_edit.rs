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
}

pub struct TrackEditWidgets {
    toast_overlay: adw::ToastOverlay,
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

            .build();
        let toast_overlay = adw::ToastOverlay::new();
        let scroller = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&group)
            .build();
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
