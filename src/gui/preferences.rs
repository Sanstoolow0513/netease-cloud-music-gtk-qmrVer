//
// preferences.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use crate::utils::sanitize_pages_order;
use adw::prelude::{ActionRowExt, PreferencesGroupExt};
use gettextrs::gettext;
use gio::Settings;
use gtk::gio::SettingsBindFlags;
use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*, *};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

glib::wrapper! {
    pub struct NeteaseCloudMusicGtk4Preferences(ObjectSubclass<imp::NeteaseCloudMusicGtk4Preferences>)
        @extends adw::PreferencesDialog, adw::Dialog, Widget,
        @implements Accessible, Buildable, ConstraintTarget, Native, Root, ShortcutManager;
}

impl NeteaseCloudMusicGtk4Preferences {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);
        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect("Could not get settings.")
    }

    fn bind_settings(&self) {
        let switch = self.imp().exit_switch.get();
        self.settings()
            .bind("exit-switch", &switch, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let mute_start_switch = self.imp().mute_start_switch.get();
        self.settings()
            .bind("mute-start", &mute_start_switch, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let not_ignore_grey_switch = self.imp().not_ignore_grey_switch.get();
        self.settings()
            .bind("not-ignore-grey", &not_ignore_grey_switch, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let entry = self.imp().proxy_entry.get();
        self.settings()
            .bind("proxy-address", &entry, "text")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let rate = self.imp().switch_rate.get();
        self.settings()
            .bind("music-rate", &rate, "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let cache_clear = self.imp().cache_clear.get();
        self.settings()
            .bind("cache-clear", &cache_clear, "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        let desktop_lyrics = self.imp().desktop_lyrics.get();
        self.settings()
            .bind("desktop-lyrics", &desktop_lyrics, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }

    pub fn set_cache_size_label(&self, size: f64, unit: String) {
        self.imp()
            .cache_clear
            .get()
            .set_property("subtitle", format!("{:.1} {}", size, unit));
    }

    fn page_title(name: &str) -> String {
        match name {
            "discover" => gettext("Discover"),
            "toplist" => gettext("Toplist"),
            "my" => gettext("My"),
            _ => name.to_string(),
        }
    }

    fn show_setting_key(name: &str) -> Option<&'static str> {
        match name {
            "discover" => Some("show-discover"),
            "toplist" => Some("show-toplist"),
            _ => None,
        }
    }

    fn rebuild_page_rows(&self) {
        let imp = self.imp();
        let pages_group = imp.pages_group.get();

        for row in imp.page_rows.borrow_mut().drain(..) {
            pages_group.remove(&row);
        }

        let settings = self.settings();
        let order =
            sanitize_pages_order(settings.strv("pages-order").iter().map(|s| s.to_string()));
        let order_len = order.len();

        for (index, name) in order.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(Self::page_title(name))
                .build();

            if let Some(key) = Self::show_setting_key(name) {
                let switch = Switch::builder().valign(Align::Center).build();
                settings
                    .bind(key, &switch, "active")
                    .flags(SettingsBindFlags::DEFAULT)
                    .build();
                row.add_suffix(&switch);
                row.set_activatable_widget(Some(&switch));
            }

            let up_button = Button::builder()
                .icon_name("go-up-symbolic")
                .valign(Align::Center)
                .sensitive(index > 0)
                .build();
            up_button.add_css_class("flat");

            let down_button = Button::builder()
                .icon_name("go-down-symbolic")
                .valign(Align::Center)
                .sensitive(index + 1 < order_len)
                .build();
            down_button.add_css_class("flat");

            let this = self.clone();
            let idx = index;
            up_button.connect_clicked(move |_| {
                this.move_page(idx, idx - 1);
            });

            let this = self.clone();
            let idx = index;
            down_button.connect_clicked(move |_| {
                this.move_page(idx, idx + 1);
            });

            row.add_suffix(&up_button);
            row.add_suffix(&down_button);

            pages_group.add(&row);
            imp.page_rows.borrow_mut().push(row);
        }
    }

    fn move_page(&self, from: usize, to: usize) {
        let settings = self.settings();
        let mut order =
            sanitize_pages_order(settings.strv("pages-order").iter().map(|s| s.to_string()));
        if from >= order.len() || to >= order.len() {
            return;
        }
        order.swap(from, to);
        let refs: Vec<&str> = order.iter().map(|s| s.as_str()).collect();
        let _ = settings.set_strv("pages-order", refs);
        self.rebuild_page_rows();
    }
}

impl Default for NeteaseCloudMusicGtk4Preferences {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use adw::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/preferences.ui")]
    pub struct NeteaseCloudMusicGtk4Preferences {
        pub settings: OnceCell<Settings>,
        pub page_rows: RefCell<Vec<adw::ActionRow>>,
        #[template_child]
        pub exit_switch: TemplateChild<Switch>,
        #[template_child]
        pub mute_start_switch: TemplateChild<Switch>,
        #[template_child]
        pub not_ignore_grey_switch: TemplateChild<Switch>,
        #[template_child]
        pub proxy_entry: TemplateChild<Entry>,
        #[template_child]
        pub switch_rate: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub cache_clear: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub desktop_lyrics: TemplateChild<Switch>,
        #[template_child]
        pub pages_group: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NeteaseCloudMusicGtk4Preferences {
        const NAME: &'static str = "NeteaseCloudMusicGtk4Preferences";
        type Type = super::NeteaseCloudMusicGtk4Preferences;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NeteaseCloudMusicGtk4Preferences {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.setup_settings();
            obj.bind_settings();
            obj.rebuild_page_rows();
        }
    }
    impl WidgetImpl for NeteaseCloudMusicGtk4Preferences {}
    impl AdwDialogImpl for NeteaseCloudMusicGtk4Preferences {}
    impl PreferencesDialogImpl for NeteaseCloudMusicGtk4Preferences {}
}
