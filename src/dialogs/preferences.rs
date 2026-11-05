use adw::prelude::*;
use relm4::SimpleComponent;

use crate::config::Config;

#[derive(Debug)]
pub enum PreferencesMsg {
    DefaultVoidSize(u32),
    MaxTracksInSingle(u32),
    SkipPositionInSingle(bool),
    RenamePatterinInEp(String),
    RenamePatterinInSingle(String),
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
        adw::PreferencesDialog::builder().title("Preferences").build()
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
        void_row.set_title("Default Void size");
        void_row.set_subtitle("Size give to a newly added Void");
        void_row.set_value(config.default_void_size as f64);
        {
            let sender = sender.clone();
            void_row.connect_value_notify(move |row| {
                sender.input(PreferencesMsg::DefaultVoidSize(row.value() as u32));
            });
        }
        let albums_group = adw::PreferencesGroup::builder().title("Albums").build();
        albums_group.add(&void_row);

        // Singles
        let max_row = adw::SpinRow::with_range(1.0, 99.0, 1.0);
        max_row.set_title("Tracks in a Single");
        max_row.set_subtitle("How many tracks a Single may hold");
        max_row.set_value(config.max_count_of_tracks_in_single as f64);
        {
            let sender = sender.clone();
            max_row.connect_value_notify(move |row| {
                sender.input(PreferencesMsg::MaxTracksInSingle(row.value() as u32));
            });
        }
        let skip_row = adw::SwitchRow::builder()
            .title("Don't write track number for Singles")
            .active(config.skip_saving_track_position_in_single)
            .build();
        {
            let sender = sender.clone();
            skip_row.connect_active_notify(move |row| {
                sender.input(PreferencesMsg::SkipPositionInSingle(row.is_active()));
            });
        }
        let singles_group = adw::PreferencesGroup::builder().title("Singles").build();
        singles_group.add(&max_row);
        singles_group.add(&skip_row);

        let page = adw::PreferencesPage::new();
        page.add(&albums_group);
        page.add(&singles_group);
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
            PreferencesMsg::RenamePatterinInEp(v) => self.config.rename_pattern_for_ep = v,
            PreferencesMsg::RenamePatterinInSingle(v) => self.config.rename_pattern_for_single = v,
            PreferencesMsg::Closed => {
                sender.output(PreferencesOutput::Closed).ok();
                return;
            }
        }
        sender.output(PreferencesOutput::Changed(self.config.clone())).ok();
    }
}
