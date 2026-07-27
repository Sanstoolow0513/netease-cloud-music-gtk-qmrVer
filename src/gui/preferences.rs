//
// preferences.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use crate::audio::output_device::{self, AudioOutputDevice};
use crate::gui::theme::{self, THEME_IDS};
use crate::gui::typography::{self, FONT_PRESET_IDS};
use crate::i18n::{self, LANGUAGE_IDS};
use crate::utils::sanitize_pages_order;
use adw::prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt};
use gettextrs::gettext;
use gio::Settings;
use gtk::gio::SettingsBindFlags;
use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*, *};
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};

/// Ordered style-variant ids; index matches the Style ComboRow.
const STYLE_VARIANT_IDS: &[&str] = &["system", "light", "dark"];

fn style_variant_label(id: &str) -> String {
    match id {
        "system" => gettext("Follow System"),
        "light" => gettext("Light"),
        "dark" => gettext("Dark"),
        _ => id.to_string(),
    }
}

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

    fn bind_optional_switch(
        &self,
        supported: bool,
        key: &str,
        switch: &Switch,
        row: &adw::ActionRow,
    ) {
        if supported {
            self.settings()
                .bind(key, switch, "active")
                .flags(SettingsBindFlags::DEFAULT)
                .build();
        } else {
            switch.set_active(false);
            row.set_visible(false);
        }
    }

    fn bind_settings(&self) {
        self.bind_optional_switch(
            crate::platform::HAS_SYSTEM_TRAY,
            "exit-switch",
            &self.imp().exit_switch.get(),
            &self.imp().exit_row.get(),
        );

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

        // The model has to exist before `selected` is bound, otherwise the
        // binding starts out on an empty model.
        self.refresh_cache_clear_model();
        let cache_clear = self.imp().cache_clear.get();
        self.settings()
            .bind("cache-clear", &cache_clear, "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        self.bind_optional_switch(
            crate::platform::HAS_DESKTOP_LYRICS,
            "desktop-lyrics",
            &self.imp().desktop_lyrics.get(),
            &self.imp().desktop_lyrics_row.get(),
        );

        self.bind_audio_device();

        self.bind_ui_language();
        self.bind_font_preset();
        self.bind_ui_style();
        self.bind_theme();
        self.bind_font_scale("ui-font-scale", &self.imp().ui_font_scale.get());
        self.bind_font_scale(
            "list-title-font-scale",
            &self.imp().list_title_font_scale.get(),
        );
        self.bind_font_scale(
            "list-meta-font-scale",
            &self.imp().list_meta_font_scale.get(),
        );
        self.bind_font_scale("player-font-scale", &self.imp().player_font_scale.get());
        self.bind_font_scale("lyrics-font-scale", &self.imp().lyrics_font_scale.get());
    }

    fn bind_font_scale(&self, key: &str, scale: &Scale) {
        self.settings()
            .bind(key, &scale.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }

    fn bind_ui_language(&self) {
        self.refresh_ui_language_model();

        let settings = self.settings();
        self.imp().ui_language.connect_selected_notify(glib::clone!(
            #[strong]
            settings,
            #[weak(rename_to = dialog)]
            self,
            move |combo| {
                if dialog.imp().refreshing_models.get() {
                    return;
                }
                let idx = combo.selected() as usize;
                if let Some(id) = LANGUAGE_IDS.get(idx) {
                    let _ = settings.set_string("ui-language", id);
                }
            }
        ));
    }

    fn bind_font_preset(&self) {
        self.refresh_font_preset_model();

        let settings = self.settings();
        self.imp().font_preset.connect_selected_notify(glib::clone!(
            #[strong]
            settings,
            #[weak(rename_to = dialog)]
            self,
            move |combo| {
                if dialog.imp().refreshing_models.get() {
                    return;
                }
                let idx = combo.selected() as usize;
                if let Some(id) = FONT_PRESET_IDS.get(idx) {
                    let _ = settings.set_string("ui-font-preset", id);
                }
            }
        ));
    }

    fn bind_theme(&self) {
        self.refresh_theme_model();

        let settings = self.settings();
        self.imp().ui_theme.connect_selected_notify(glib::clone!(
            #[strong]
            settings,
            #[weak(rename_to = dialog)]
            self,
            move |combo| {
                if dialog.imp().refreshing_models.get() {
                    return;
                }
                let idx = combo.selected() as usize;
                if let Some(id) = THEME_IDS.get(idx) {
                    let _ = settings.set_string("ui-theme", id);
                }
            }
        ));
    }

    fn bind_ui_style(&self) {
        self.refresh_ui_style_model();

        let settings = self.settings();
        self.imp().ui_style.connect_selected_notify(glib::clone!(
            #[strong]
            settings,
            #[weak(rename_to = dialog)]
            self,
            move |combo| {
                if dialog.imp().refreshing_models.get() {
                    return;
                }
                let idx = combo.selected() as usize;
                if let Some(id) = STYLE_VARIANT_IDS.get(idx) {
                    let _ = settings.set_string("style-variant", id);
                }
            }
        ));
    }

    fn set_combo_items(combo: &adw::ComboRow, labels: &[String], selected: u32) {
        let model = StringList::new(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        combo.set_model(Some(&model));
        combo.set_selected(selected);
    }

    fn bind_audio_device(&self) {
        // 先刷新模型再连接信号：模型初始填充不应回写 GSettings
        self.refresh_audio_device_model();

        let settings = self.settings();
        self.imp()
            .audio_device
            .connect_selected_notify(glib::clone!(
                #[strong]
                settings,
                #[weak(rename_to = dialog)]
                self,
                move |combo| {
                    if dialog.imp().refreshing_models.get() {
                        return;
                    }
                    let idx = combo.selected() as usize;
                    let id = if idx == 0 {
                        String::new()
                    } else {
                        dialog
                            .imp()
                            .audio_devices
                            .borrow()
                            .get(idx - 1)
                            .map(|d| d.id.clone())
                            .unwrap_or_default()
                    };
                    let _ = settings.set_string("audio-device", &id);
                }
            ));
    }

    fn refresh_audio_device_model(&self) {
        let imp = self.imp();
        let stored = self.settings().string("audio-device");
        // 对话框每次打开都会重建（application.rs），此处枚举到的总是最新设备
        let devices = output_device::enumerate();
        let selected = devices
            .iter()
            .position(|d| d.id == stored.as_str())
            .map(|i| i + 1)
            .unwrap_or(0);
        let mut labels = vec![gettext("Follow system default")];
        labels.extend(devices.iter().map(|d| d.name.clone()));
        *imp.audio_devices.borrow_mut() = devices;
        Self::set_combo_items(&imp.audio_device.get(), &labels, selected as u32);
    }

    fn refresh_ui_language_model(&self) {
        let language = self.settings().string("ui-language");
        let selected = LANGUAGE_IDS
            .iter()
            .position(|&id| id == language.as_str())
            .unwrap_or(0);
        let labels: Vec<String> = LANGUAGE_IDS
            .iter()
            .map(|id| i18n::language_label(id))
            .collect();
        Self::set_combo_items(&self.imp().ui_language.get(), &labels, selected as u32);
    }

    fn refresh_font_preset_model(&self) {
        let preset = self.settings().string("ui-font-preset");
        let selected = FONT_PRESET_IDS
            .iter()
            .position(|&id| id == preset.as_str())
            .unwrap_or(0);
        let labels: Vec<String> = FONT_PRESET_IDS
            .iter()
            .map(|id| typography::preset_label(id))
            .collect();
        Self::set_combo_items(&self.imp().font_preset.get(), &labels, selected as u32);
    }

    fn refresh_theme_model(&self) {
        let theme_id = self.settings().string("ui-theme");
        let selected = THEME_IDS
            .iter()
            .position(|&id| id == theme_id.as_str())
            .unwrap_or(0);
        let labels: Vec<String> = THEME_IDS.iter().map(|id| theme::theme_label(id)).collect();
        Self::set_combo_items(&self.imp().ui_theme.get(), &labels, selected as u32);
    }

    fn refresh_ui_style_model(&self) {
        let variant = self.settings().string("style-variant");
        let selected = STYLE_VARIANT_IDS
            .iter()
            .position(|&id| id == variant.as_str())
            .unwrap_or(0);
        let labels: Vec<String> = STYLE_VARIANT_IDS
            .iter()
            .map(|id| style_variant_label(id))
            .collect();
        Self::set_combo_items(&self.imp().ui_style.get(), &labels, selected as u32);
    }

    fn refresh_cache_clear_model(&self) {
        // Keep the stored value: swapping the model resets `selected`, which is
        // bound to GSettings.
        let selected = self.settings().uint("cache-clear");
        let labels = vec![
            gettext("Never"),
            gettext("Daily"),
            gettext("Weekly"),
            gettext("Monthly"),
        ];
        Self::set_combo_items(&self.imp().cache_clear.get(), &labels, selected);
    }

    /// Rebuild everything whose text was baked in when the dialog was built.
    pub fn retranslate(&self, retranslator: &crate::i18n::Retranslator) {
        retranslator.retranslate_widget(self);

        // Combo rows render their rows from the model, which keeps the strings
        // it was created with.
        self.imp().refreshing_models.set(true);
        self.refresh_ui_language_model();
        self.refresh_font_preset_model();
        self.refresh_ui_style_model();
        self.refresh_theme_model();
        self.refresh_cache_clear_model();
        self.refresh_audio_device_model();
        self.imp().refreshing_models.set(false);

        self.rebuild_page_rows();
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
        /// Set while combo models are swapped, so the `selected` handlers do not
        /// write the transient selection back to GSettings.
        pub refreshing_models: Cell<bool>,
        #[template_child]
        pub exit_row: TemplateChild<adw::ActionRow>,
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
        pub audio_device: TemplateChild<adw::ComboRow>,
        /// 与 audio_device 下拉项一一对应（第 0 项为「跟随系统默认」，不占此表）
        pub audio_devices: RefCell<Vec<AudioOutputDevice>>,
        #[template_child]
        pub cache_clear: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub ui_language: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub desktop_lyrics_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub desktop_lyrics: TemplateChild<Switch>,
        #[template_child]
        pub pages_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub font_preset: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub ui_style: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub ui_theme: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub ui_font_scale: TemplateChild<Scale>,
        #[template_child]
        pub list_title_font_scale: TemplateChild<Scale>,
        #[template_child]
        pub list_meta_font_scale: TemplateChild<Scale>,
        #[template_child]
        pub player_font_scale: TemplateChild<Scale>,
        #[template_child]
        pub lyrics_font_scale: TemplateChild<Scale>,
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
