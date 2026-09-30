mod dialogs;
mod models;
mod tests;

use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

use crate::dialogs::track_edit::{TrackEditDialog, TrackEditOutput};
use crate::models::{
    album::{Album, AlbumItem, AlbumItemType, AlbumMode},
    track::Track,
};

const DEFAULT_VOID_SIZE: u32 = 1;

struct AppModel {
    album: Album,
    window: adw::Window,
    track_edit_dialog: Option<Controller<TrackEditDialog>>,
}

#[derive(Debug)]
enum AppMsg {
    AddItem(AlbumItemType),
    NewAlbum,
    EditTrackRequest(usize),
    TrackEditResult(TrackEditOutput),
}

struct AppWidgets {
    split_view: adw::NavigationSplitView,
    album_title_label: gtk::Label,
    album_artist_label: gtk::Label,
    album_meta_label: gtk::Label,
    tracklist_box: gtk::Box,
}

impl SimpleComponent for AppModel {
    type Input = AppMsg;
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        adw::Window::builder()
            .title("AlbumCard")
            .default_width(900)
            .default_height(600)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut album = Album::new(
            AlbumMode::Single,
            "Iron Lotus",
            "Mili",
            "2021-02-27",
            "Electronic",
            None,
        )
        .expect("hardcoded demo album should be valid");
        let mut track = Track::new(PathBuf::from("/home/pisek/Hudba/Mili - Iron Lotus.mp3"));
        track.title = "Iron Lotus".to_string();
        album.add_track(track);

        let split_view = adw::NavigationSplitView::new();

        // Sidebar pane
        let hamburger = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();

        let collapse_button = gtk::Button::from_icon_name("sidebar-show-symbolic");
        {
            let split_view = split_view.clone();
            collapse_button.connect_clicked(move |_| {
                split_view.set_collapsed(!split_view.is_collapsed());
            });
        }

        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.pack_start(&hamburger);
        sidebar_header.pack_end(&collapse_button);
        sidebar_header.set_title_widget(Some(&adw::WindowTitle::new("AlbumCard", "")));

        let new_album_content = adw::ButtonContent::builder()
            .icon_name("list-add-symbolic")
            .label("New Album")
            .build();
        let new_album_button = gtk::Button::builder().child(&new_album_content).build();
        new_album_button.add_css_class("suggested-action");
        {
            let sender = sender.clone();
            new_album_button.connect_clicked(move |_| {
                sender.input(AppMsg::NewAlbum);
            });
        }

        let sidebar_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .build();
        sidebar_list.append(&gtk::Label::new(Some(&album.title)));

        let sidebar_content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        sidebar_content_box.append(&new_album_button);
        sidebar_content_box.append(&sidebar_list);

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.set_content(Some(&sidebar_content_box));
        let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "Albums");

        // Content pane
        let content_header = adw::HeaderBar::new();

        let undo_button = gtk::Button::from_icon_name("edit-undo-symbolic");
        undo_button.set_sensitive(false);
        let redo_button = gtk::Button::from_icon_name("edit-redo-symbolic");
        redo_button.set_sensitive(false);
        content_header.pack_start(&undo_button);
        content_header.pack_start(&redo_button);

        let add_popover = gtk::Popover::new();
        let add_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        for kind in album.allowed_item_types() {
            let label = match kind {
                AlbumItemType::Disk => "Disk",
                AlbumItemType::Track => "Track",
                AlbumItemType::Void => "Void",
            };
            let item_button = gtk::Button::builder()
                .label(label)
                .css_classes(["flat"])
                .halign(gtk::Align::Start)
                .build();
            let sender = sender.clone();
            let popover = add_popover.clone();
            item_button.connect_clicked(move |_| {
                sender.input(AppMsg::AddItem(kind));
                popover.popdown();
            });
            add_popover_box.append(&item_button);
        }
        add_popover.set_child(Some(&add_popover_box));

        let add_button = adw::SplitButton::builder()
            .icon_name("list-add-symbolic")
            .popover(&add_popover)
            .build();
        {
            let sender = sender.clone();
            add_button.connect_clicked(move |_| {
                sender.input(AppMsg::AddItem(AlbumItemType::Track));
            });
        }
        content_header.pack_start(&add_button);

        let edit_album_button = gtk::Button::from_icon_name("document-edit-symbolic");
        content_header.pack_start(&edit_album_button);

        let save_button = gtk::Button::from_icon_name("document-save-symbolic");
        save_button.add_css_class("suggested-action");
        content_header.pack_start(&save_button);

        let art_icon = gtk::Image::from_icon_name("folder-music-symbolic");
        art_icon.set_pixel_size(48);
        let art_placeholder = gtk::Frame::builder()
            .width_request(96)
            .height_request(96)
            .child(&art_icon)
            .build();

        let album_title_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .css_classes(["title-1"])
            .build();
        let album_artist_label = gtk::Label::builder().halign(gtk::Align::Start).build();
        let album_meta_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();

        let album_text_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        album_text_box.append(&album_title_label);
        album_text_box.append(&album_artist_label);
        album_text_box.append(&album_meta_label);

        let album_header_box = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        album_header_box.append(&art_placeholder);
        album_header_box.append(&album_text_box);

        let tracklist_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let tracklist_scroller = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&tracklist_box)
            .build();

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(64)
            .margin_bottom(64)
            .margin_start(64)
            .margin_end(64)
            .build();
        content_box.append(&album_header_box);
        content_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        content_box.append(&tracklist_scroller);

        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&content_header);
        content_toolbar.set_content(Some(&content_box));
        let content_page = adw::NavigationPage::new(&content_toolbar, &album.title);

        split_view.set_sidebar(Some(&sidebar_page));
        split_view.set_content(Some(&content_page));
        root.set_content(Some(&split_view));

        // Album filling
        album_title_label.set_label(&album.title);
        album_artist_label.set_label(&album.album_artist);
        album_meta_label.set_label(&format_album_meta(&album));
        populate_tracklist(&tracklist_box, &album, sender);

        let model = AppModel {
            album,
            window: root.clone(),
            track_edit_dialog: None,
        };
        let widgets = AppWidgets {
            split_view,
            album_title_label,
            album_artist_label,
            album_meta_label,
            tracklist_box,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppMsg::AddItem(kind) => match kind {
                AlbumItemType::Disk => {
                    self.album.add_disk(None);
                }
                AlbumItemType::Track => {
                    self.album.add_track(Track::new(PathBuf::new()));
                }
                AlbumItemType::Void => {
                    self.album.add_void(DEFAULT_VOID_SIZE);
                }
            },
            AppMsg::NewAlbum => {
                eprintln!("TODO: open the new-album dialog");
            }
            AppMsg::EditTrackRequest(index) => {
                let Some(AlbumItem::Track(track)) = self.album.get_items().get(index) else {
                    return;
                };
                let dialog = TrackEditDialog::builder()
                    .launch((index, track.edit_snapshot()))
                    .forward(sender.input_sender(), AppMsg::TrackEditResult);
                dialog.widget().present(Some(&self.window));

                self.track_edit_dialog = Some(dialog);
            }
            AppMsg::TrackEditResult(TrackEditOutput::Saved { index, edit }) => {
                if let Some(AlbumItem::Track(track)) = self.album.get_item_mut(index) {
                    track.apply_edit(edit).ok();
                }
                if let Some(dialog) = self.track_edit_dialog.take() {
                    dialog.widget().close();
                }
            }
            AppMsg::TrackEditResult(TrackEditOutput::Cancelled) => {
                if let Some(dialog) = self.track_edit_dialog.take() {
                    dialog.widget().close();
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        widgets.album_title_label.set_label(&self.album.title);
        widgets
            .album_artist_label
            .set_label(&self.album.album_artist);
        widgets
            .album_meta_label
            .set_label(&format_album_meta(&self.album));
        populate_tracklist(&widgets.tracklist_box, &self.album, sender);
        let _ = &widgets.split_view;
    }
}

fn format_album_meta(album: &Album) -> String {
    let type_str = match album.mode {
        AlbumMode::Single => "Single",
        AlbumMode::Ep => "EP",
    };

    let mut parts = vec![type_str.to_string()];

    if matches!(album.mode, AlbumMode::Ep) {
        let track_count = album
            .get_items()
            .iter()
            .filter(|item| matches!(item, AlbumItem::Track(_)))
            .count();
        let disk_count = album
            .get_items()
            .iter()
            .filter(|item| matches!(item, AlbumItem::Disk { .. }))
            .count();
        parts.push(format!("{track_count}/{disk_count}"));
    }

    let date_str = album
        .release_date
        .map(|d| d.format("%b %e, %Y").to_string())
        .unwrap_or_else(|| "Unknown date".to_string());
    parts.push(date_str);

    parts.push(album.genre.clone());

    parts.join(" | ")
}

fn clear_children(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn populate_tracklist(container: &gtk::Box, album: &Album, sender: ComponentSender<AppModel>) {
    clear_children(container);

    let positions = album.item_positions();
    for (idx, (item, position)) in album.get_items().iter().zip(positions).enumerate() {
        let row: gtk::Widget = match item {
            AlbumItem::Disk { title } => gtk::Label::builder()
                .label(title.clone().unwrap_or_else(|| "Disk".to_string()))
                .halign(gtk::Align::Start)
                .css_classes(["heading"])
                .build()
                .upcast(),
            AlbumItem::Track(track) => {
                let pos_str = position.map(|p| p.to_string()).unwrap_or_default();
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let label = gtk::Label::builder()
                    .label(format!("{pos_str}. {}", track.title))
                    .hexpand(true)
                    .halign(gtk::Align::Start)
                    .build();
                let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
                {
                    let sender = sender.clone();
                    edit_button.connect_clicked(move |_| {
                        sender.input(AppMsg::EditTrackRequest(idx));
                    });
                }
                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.add_css_class("destructive-action");
                row_box.append(&label);
                row_box.append(&edit_button);
                row_box.append(&delete_button);
                row_box.upcast()
            }
            AlbumItem::Void { size } => {
                let pos_str = position.map(|p| p.to_string()).unwrap_or_default();
                gtk::Label::builder()
                    .label(format!("{pos_str} - Void ({size})"))
                    .halign(gtk::Align::Start)
                    .css_classes(["dim-label"])
                    .build()
                    .upcast()
            }
        };
        container.append(&row);
    }
}

fn main() {
    let app = RelmApp::new("com.github.pisekpiskovec.AlbumCard");
    app.run::<AppModel>(());
}
