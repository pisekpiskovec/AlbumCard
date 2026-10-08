mod config;
mod dialogs;
mod models;
mod storage;
mod tests;

use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::VERSION;
use crate::dialogs::album_edit::{AlbumEditDialog, AlbumEditOutput, AlbumEditPurpose};
use crate::dialogs::track_edit::{TrackEditDialog, TrackEditOutput};
use crate::models::album::AlbumEdit;
use crate::models::{
    album::{Album, AlbumItem, AlbumItemType, AlbumMode},
    track::Track,
};

#[derive(Debug)]
enum Direction {
    Up,
    Down,
}

struct AppModel {
    album: Album,
    sidebar_albums: Vec<(Uuid, String)>,
    window: adw::Window,
    track_edit_dialog: Option<Controller<TrackEditDialog>>,
    album_edit_dialog: Option<Controller<AlbumEditDialog>>,
    toast_overlay: adw::ToastOverlay,
}

#[derive(Debug)]
enum AppMsg {
    About,
    AddItem(AlbumItemType),
    NewAlbumRequest,
    EditAlbumRequest,
    AlbumEditResult(AlbumEditOutput),
    EditTrackRequest(usize),
    TrackEditResult(TrackEditOutput),
    MoveItem(usize, Direction),
    DeleteItem(usize),
    SetVoidSize(usize, u32),
    OpenAlbum(Uuid),
    SaveAlbum,
}

struct AppWidgets {
    split_view: adw::NavigationSplitView,
    sidebar_list: gtk::ListBox,
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
        let album = Album::empty();
        let sidebar_albums = storage::list_albums();

        let split_view = adw::NavigationSplitView::new();

        // Sidebar pane
        let ham_popover = gtk::Popover::new();
        let ham_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let about_button = gtk::Button::builder()
            .label("About AlbumCard")
            .css_classes(["flat"])
            .halign(gtk::Align::Start)
            .build();
        let s = sender.clone();
        let popover = ham_popover.clone();
        about_button.connect_clicked(move |_| {
            s.input(AppMsg::About);
            popover.popdown();
        });
        ham_popover_box.append(&about_button);
        ham_popover.set_child(Some(&ham_popover_box));
        let hamburger = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .popover(&ham_popover)
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
                sender.input(AppMsg::NewAlbumRequest);
            });
        }

        let sidebar_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .build();
        populate_sidebar(&sidebar_list, &sidebar_albums, album.id, sender.clone());

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
        {
            let sender = sender.clone();
            edit_album_button.connect_clicked(move |_| {
                sender.input(AppMsg::EditAlbumRequest);
            });
        }
        content_header.pack_start(&edit_album_button);

        let save_button = gtk::Button::from_icon_name("document-save-symbolic");
        save_button.add_css_class("suggested-action");
        {
            let sender = sender.clone();
            save_button.connect_clicked(move |_| {
                sender.input(AppMsg::SaveAlbum);
            });
        }
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

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&content_box));
        content_toolbar.set_content(Some(&toast_overlay));

        let content_page = adw::NavigationPage::new(&content_toolbar, &String::new());

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
            album_edit_dialog: None,
            sidebar_albums,
            toast_overlay,
        };
        let widgets = AppWidgets {
            split_view,
            album_title_label,
            album_artist_label,
            album_meta_label,
            tracklist_box,
            sidebar_list,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppMsg::AddItem(kind) => match kind {
                AlbumItemType::Disk => {
                    self.album.add_disk();
                }
                AlbumItemType::Track => {
                    self.album.add_track(Track::new(PathBuf::new()));
                }
                AlbumItemType::Void => {
                    self.album.add_void(config::DEFAULT_VOID_SIZE);
                }
            },
            AppMsg::NewAlbumRequest => {
                let edit = AlbumEdit {
                    title: String::new(),
                    album_artist: String::new(),
                    release_date: String::new(),
                    genre: String::new(),
                    mode: AlbumMode::Ep,
                };
                let dialog = AlbumEditDialog::builder()
                    .launch((AlbumEditPurpose::New, edit, true))
                    .forward(sender.input_sender(), AppMsg::AlbumEditResult);
                dialog.widget().present(Some(&self.window));
                self.album_edit_dialog = Some(dialog);
            }
            AppMsg::EditAlbumRequest => {
                let edit = AlbumEdit {
                    title: self.album.title.clone(),
                    album_artist: self.album.album_artist.clone(),
                    release_date: self
                        .album
                        .release_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                    genre: self.album.genre.clone(),
                    mode: self.album.mode,
                };
                let could_be_single = self.album.could_be_single();
                let dialog = AlbumEditDialog::builder()
                    .launch((AlbumEditPurpose::Edit, edit, could_be_single))
                    .forward(sender.input_sender(), AppMsg::AlbumEditResult);
                dialog.widget().present(Some(&self.window));
                self.album_edit_dialog = Some(dialog);
            }
            AppMsg::AlbumEditResult(AlbumEditOutput::Saved { purpose, edit }) => {
                match purpose {
                    AlbumEditPurpose::New => {
                        match Album::new(
                            edit.mode,
                            &edit.title,
                            &edit.album_artist,
                            &edit.release_date,
                            &edit.genre,
                            None,
                        ) {
                            Ok(new_album) => {
                                if let Err(e) = crate::storage::save_album(&new_album){
                                    eprintln!("failed to save new album: {e}");
                                }
                                self.album = new_album;
                                self.sidebar_albums = crate::storage::list_albums();
                            }
                            Err(e) => {
                                eprintln!("unexpected error creating album: {e}");
                            }
                        }
                    }
                    AlbumEditPurpose::Edit => {
                        if let Err(errors) = self.album.apply_edit(edit) {
                            eprintln!(
                                "unecpected validation failure applying album edit: {errors:?}"
                            );
                        }
                    }
                }
                if let Some(dialog) = self.album_edit_dialog.take() {
                    dialog.widget().close();
                }
            }
            AppMsg::AlbumEditResult(AlbumEditOutput::Cancelled) => {
                if let Some(dialog) = self.album_edit_dialog.take() {
                    dialog.widget().close();
                }
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
            AppMsg::MoveItem(index, direction) => match direction {
                Direction::Up => {
                    self.album.move_item(index, index.saturating_sub(1));
                }
                Direction::Down => {
                    self.album.move_item(index, index.saturating_add(1));
                }
            },
            AppMsg::About => {
                let about = adw::AboutDialog::builder()
                    .application_name("AlbumCard")
                    // .application_icon()
                    .developer_name("Písek Pískovec")
                    .version(VERSION)
                    .developers(vec!["Písek Pískovec"])
                    // .artists(vec![]})
                    // .translator_credits(vec![])
                    .copyright("TBD")
                    .comments("\\o")
                    .website("https://github.com/PisekPiskovec/AlbumCard")
                    .issue_url("https://github.com/PisekPiskovec/AlbumCard/issues")
                    // .license_type()
                    .build();

                let app = relm4::main_adw_application();
                if let Some(win) = app.active_window() {
                    about.present(Some(&win));
                }
            }
            AppMsg::DeleteItem(index) => {
                self.album
                    .remove_item(index)
                    .expect("Index out of bounds? Why?");
            }
            AppMsg::SetVoidSize(index, size) => {
                if let Some(AlbumItem::Void { size: void_size }) = self.album.get_item_mut(index) {
                    *void_size = size.max(1);
                }
            }
            AppMsg::OpenAlbum(id) => {
                if id == self.album.id {
                    return;
                }
                match crate::storage::load_album(id) {
                    Ok(loaded) => self.album = loaded,
                    Err(e) => eprintln!("failed to load album {id}: {e}"),
                }
            }
            AppMsg::SaveAlbum => {
                if let Err(problems) = self.album.validate_for_save() {
                    let text = match problems.len() {
                        1 => problems[0].clone(),
                        n => format!("{} (and {} more)", problems[0], n - 1),
                    };
                    self.toast_overlay.add_toast(adw::Toast::builder().title(text).build());
                    return;
                }
                if let Err(e) = crate::storage::save_album(&self.album) {
                    self.toast_overlay.add_toast(adw::Toast::builder().title(format!("{e}")).build());
                    eprintln!("failed to save album: {e}");
                }
                self.sidebar_albums = crate::storage::list_albums();
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
        populate_sidebar(
            &widgets.sidebar_list,
            &self.sidebar_albums,
            self.album.id,
            sender.clone(),
        );
        populate_tracklist(&widgets.tracklist_box, &self.album, sender.clone());
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
            .filter(|item| matches!(item, AlbumItem::Disk))
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

fn populate_sidebar(
    list: &gtk::ListBox,
    albums: &[(Uuid, String)],
    current_id: Uuid,
    sender: ComponentSender<AppModel>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for (id, title) in albums {
        let id = *id;
        let sender = sender.clone();
        let row = adw::ActionRow::builder()
            .title(title)
            .activatable(true)
            .build();
        if id == current_id {
            list.select_row(Some(&row));
        }
        row.connect_activated(move |_| {
            sender.input(AppMsg::OpenAlbum(id));
        });
        list.append(&row);
    }
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
            AlbumItem::Disk => {
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let disk_label = match album.disk_number_at(idx) {
                    Some(n) => format!("Disc {n}"),
                    None => "Disc".to_string(), // fallback that should not happen
                };
                let label = gtk::Label::builder()
                    .label(disk_label)
                    .halign(gtk::Align::Start)
                    .hexpand(true)
                    .css_classes(["heading"])
                    .build();
                let move_up_button = gtk::Button::from_icon_name("go-up-symbolic");
                move_up_button.set_visible(idx > 0);
                {
                    let sender = sender.clone();
                    move_up_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Up));
                    });
                }
                let move_down_button = gtk::Button::from_icon_name("go-down-symbolic");
                move_down_button.set_visible(idx < album.get_items().len() - 1);
                {
                    let sender = sender.clone();
                    move_down_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Down));
                    });
                }
                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.add_css_class("destructive-action");
                {
                    let sender = sender.clone();
                    delete_button.connect_clicked(move |_| {
                        sender.input(AppMsg::DeleteItem(idx));
                    });
                }
                row_box.append(&label);
                row_box.append(&move_up_button);
                row_box.append(&move_down_button);
                row_box.append(&delete_button);
                row_box.upcast()
            }
            AlbumItem::Track(track) => {
                let pos_str = position.map(|p| p.to_string()).unwrap_or_default();
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let label = gtk::Label::builder()
                    .label(format!("{pos_str}. {}", track.title))
                    .hexpand(true)
                    .halign(gtk::Align::Start)
                    .build();
                let move_up_button = gtk::Button::from_icon_name("go-up-symbolic");
                move_up_button.set_visible(idx > 0);
                {
                    let sender = sender.clone();
                    move_up_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Up));
                    });
                }
                let move_down_button = gtk::Button::from_icon_name("go-down-symbolic");
                move_down_button.set_visible(idx < album.get_items().len() - 1);
                {
                    let sender = sender.clone();
                    move_down_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Down));
                    });
                }
                let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
                {
                    let sender = sender.clone();
                    edit_button.connect_clicked(move |_| {
                        sender.input(AppMsg::EditTrackRequest(idx));
                    });
                }
                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.add_css_class("destructive-action");
                {
                    let sender = sender.clone();
                    delete_button.connect_clicked(move |_| {
                        sender.input(AppMsg::DeleteItem(idx));
                    });
                }
                row_box.append(&label);
                row_box.append(&move_up_button);
                row_box.append(&move_down_button);
                row_box.append(&edit_button);
                row_box.append(&delete_button);
                row_box.upcast()
            }
            AlbumItem::Void { size } => {
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let label = gtk::Label::builder()
                    .label(format!("Void ({size})"))
                    .halign(gtk::Align::Start)
                    .hexpand(true)
                    .css_classes(["dim-label"])
                    .build();
                let adjustment = gtk::Adjustment::new(*size as f64, 1.0, 999.0, 1.0, 10.0, 0.0);
                let size_spinner = gtk::SpinButton::new(Some(&adjustment), 1.0, 0);
                size_spinner.set_valign(gtk::Align::Center);
                {
                    let sender = sender.clone();
                    size_spinner.connect_value_changed(move |spin| {
                        sender.input(AppMsg::SetVoidSize(idx, spin.value() as u32));
                    });
                }
                let move_up_button = gtk::Button::from_icon_name("go-up-symbolic");
                move_up_button.set_visible(idx > 0);
                {
                    let sender = sender.clone();
                    move_up_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Up));
                    });
                }
                let move_down_button = gtk::Button::from_icon_name("go-down-symbolic");
                move_down_button.set_visible(idx < album.get_items().len() - 1);
                {
                    let sender = sender.clone();
                    move_down_button.connect_clicked(move |_| {
                        sender.input(AppMsg::MoveItem(idx, Direction::Down));
                    });
                }
                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.add_css_class("destructive-action");
                {
                    let sender = sender.clone();
                    delete_button.connect_clicked(move |_| {
                        sender.input(AppMsg::DeleteItem(idx));
                    });
                }
                row_box.append(&label);
                row_box.append(&move_up_button);
                row_box.append(&move_down_button);
                row_box.append(&size_spinner);
                row_box.append(&delete_button);
                row_box.upcast()
            }
        };
        container.append(&row);
    }
}

fn main() {
    let app = RelmApp::new("com.github.pisekpiskovec.AlbumCard");
    app.run::<AppModel>(());
}
