use std::path::PathBuf;
use gtk::prelude::*;

pub mod album_edit;
pub mod track_edit;

/// Opens a "pick a .templ file" dialog over watherver window `anchor` is in.
pub fn choose_template<W: IsA<gtk::Widget>>(anchor: &W, on_pick: impl FnOnce(PathBuf) + 'static) {
    let filter = gtk::FileFilter::new();
    filter.add_suffix("templ");
    filter.set_name(Some("Template"));
    let dialog = gtk::FileDialog::builder()
        .title("Load Template")
        .default_filter(&filter)
        .build();
    let window = anchor.root().and_downcast::<gtk::Window>();
    dialog.open(window.as_ref(), None::<&gtk::gio::Cancellable>, move |result| {
        if let Ok(file) = result
            && let Some(path) = file.path()
        {
            on_pick(path);
        }
    });
}
