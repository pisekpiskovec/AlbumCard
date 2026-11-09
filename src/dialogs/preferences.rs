use adw::prelude::*;
use gettextrs::gettext;
use relm4::SimpleComponent;

use crate::{config::Config, renaming::validate_pattern};

#[derive(Debug)]
pub enum PreferencesMsg {
    DefaultVoidSize(u32),
    MaxTracksInSingle(u32),
    SkipPositionInSingle(bool),
    RenamePatterinInEp(String),
    RenamePatterinInSingle(String),
    RenameFileOnSave(bool),
    Closed,
}

#[derive(Debug)]
pub enum PreferencesOutput {
    /// Sent ont every change; the app keeps the latest copy.
    Changed(Config),
    /// The dialog was dismissed; the app should save now.
    Closed,
}

pub struct PreferencesDialog {
    config: Config,
}

impl SimpleComponent for PreferencesDialog {
    type Input = PreferencesMsg;
    type Output = PreferencesOutput;
    type Init = Config;
    type Root = adw::PreferencesDialog;
    type Widgets = ();

    fn init_root() -> Self::Root {
        adw::PreferencesDialog::builder().title(gettext("Preferences")).build()
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: relm4::prelude::ComponentSender<Self>,
    ) -> relm4::prelude::ComponentParts<Self> {
        {
            let sender = sender.clone();
            root.connect_closed(move |_| sender.input(PreferencesMsg::Closed));
        }

        // Albums
        let void_row = adw::SpinRow::with_range(1.0, 999.0, 1.0);
        void_row.set_title(&gettext("Default Void size"));
        void_row.set_subtitle(&gettext("Size give to a newly added Void"));
        void_row.set_value(config.default_void_size as f64);
        {
            let sender = sender.clone();
            void_row.connect_value_notify(move |row| {
                sender.input(PreferencesMsg::DefaultVoidSize(row.value() as u32));
            });
        }
        let albums_group = adw::PreferencesGroup::builder().title(gettext("Albums")).build();
        albums_group.add(&void_row);

        // Singles
        let max_row = adw::SpinRow::with_range(1.0, 99.0, 1.0);
        max_row.set_title(&gettext("Tracks in a Single"));
        max_row.set_subtitle(&gettext("How many tracks a Single may hold"));
        max_row.set_value(config.max_count_of_tracks_in_single as f64);
        {
            let sender = sender.clone();
            max_row.connect_value_notify(move |row| {
                sender.input(PreferencesMsg::MaxTracksInSingle(row.value() as u32));
            });
        }
        let skip_row = adw::SwitchRow::builder()
            .title(gettext("Don't write track number for Singles"))
            .active(config.skip_saving_track_position_in_single)
            .build();
        {
            let sender = sender.clone();
            skip_row.connect_active_notify(move |row| {
                sender.input(PreferencesMsg::SkipPositionInSingle(row.is_active()));
            });
        }
        let singles_group = adw::PreferencesGroup::builder().title(gettext("Singles")).build();
        singles_group.add(&max_row);
        singles_group.add(&skip_row);

        // File renaming
        let rename_on_save_row = adw::SwitchRow::builder()
            .title(gettext("Rename files on save"))
            .active(config.rename_on_save)
            .build();
        {
            let sender = sender.clone();
            rename_on_save_row.connect_active_notify(move |row| {
                sender.input(PreferencesMsg::RenameFileOnSave(row.is_active()));
            });
        }
        let single_pattern_row = adw::EntryRow::new();
        single_pattern_row.set_title(&gettext("Filename pattern used when renaming items in Singles"));
        single_pattern_row.set_text(&config.rename_pattern_single);
        {
            let sender = sender.clone();
            single_pattern_row.connect_changed(move |row| {
                let valid = validate_pattern(row.text().as_str());
                if valid.is_ok() {
                    row.remove_css_class("error");
                    row.set_tooltip_text(Some(""));
                    sender.input(PreferencesMsg::RenamePatterinInSingle(row.text().to_string()));
                } else {
                    row.add_css_class("error");
                    row.set_tooltip_text(valid.err().as_deref());
                }
            });
        }
        let ep_pattern_row = adw::EntryRow::new();
        ep_pattern_row.set_title(&gettext("Filename pattern used when renaming items in EPs"));
        ep_pattern_row.set_text(&config.rename_pattern_ep);
        {
            let sender = sender.clone();
            ep_pattern_row.connect_changed(move |row| {
                let valid = validate_pattern(row.text().as_str());
                if valid.is_ok() {
                    row.remove_css_class("error");
                    row.set_tooltip_text(Some(""));
                    sender.input(PreferencesMsg::RenamePatterinInEp(row.text().to_string()));
                } else {
                    row.add_css_class("error");
                    row.set_tooltip_text(valid.err().as_deref());
                }
            });
        }
        let rename_group = adw::PreferencesGroup::builder().title(gettext("File Renaming")).build();
        rename_group.add(&rename_on_save_row);
        rename_group.add(&single_pattern_row);
        rename_group.add(&ep_pattern_row);

        let page = adw::PreferencesPage::new();
        page.add(&albums_group);
        page.add(&singles_group);
        page.add(&rename_group);
        root.add(&page);

        relm4::ComponentParts {
            model: PreferencesDialog { config },
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {
        match message {
            PreferencesMsg::DefaultVoidSize(v) => self.config.default_void_size = v.max(1),
            PreferencesMsg::MaxTracksInSingle(v) => self.config.max_count_of_tracks_in_single = v.max(1),
            PreferencesMsg::SkipPositionInSingle(v) => self.config.skip_saving_track_position_in_single = v,
            PreferencesMsg::RenamePatterinInEp(v) => self.config.rename_pattern_ep = v,
            PreferencesMsg::RenamePatterinInSingle(v) => self.config.rename_pattern_single = v,
            PreferencesMsg::RenameFileOnSave(v) => self.config.rename_on_save = v,
            PreferencesMsg::Closed => {
                sender.output(PreferencesOutput::Closed).ok();
                return;
            }
        }
        sender.output(PreferencesOutput::Changed(self.config.clone())).ok();
    }
}
