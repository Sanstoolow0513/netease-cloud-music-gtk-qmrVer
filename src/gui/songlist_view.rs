//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use adw::prelude::*;
use gio::Settings;
use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib, *};

use crate::{application::Action, gui::songlist_row::SonglistRow};
use async_channel::Sender;
use glib::{
    ParamSpec, ParamSpecBoolean, ParamSpecInt, RustClosure, SignalHandlerId, Value, clone,
    subclass::Signal,
};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct SongListView(ObjectSubclass<imp::SongListView>)
        @extends Widget, Box,
        @implements Accessible, Actionable, Buildable, ConstraintTarget;
}

impl Default for SongListView {
    fn default() -> Self {
        Self::new()
    }
}

impl SongListView {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_sender(&self, _sender: Sender<Action>) {
        let sender = &self.imp().sender;
        if sender.get().is_none() {
            sender.set(_sender).unwrap();
        }
    }

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);

        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    fn wants_dual(&self) -> bool {
        let imp = self.imp();
        imp.max_columns.get() >= 2 && imp.wide_layout.get()
    }

    /// 双列时全局下标奇偶分列：偶→左，奇→右。追加必须用全局下标，不能用本批 enumerate。
    fn dual_column_is_right(dual: bool, global_index: usize) -> bool {
        dual && global_index % 2 == 1
    }

    pub fn init_new_list(&self, sis: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().to_owned();
        let settings = imp.settings.get().unwrap();

        let dual = self.wants_dual();
        let left = imp.listbox_left.get();
        let right = imp.listbox_right.get();
        right.set_visible(dual);

        let no_act_like = self.property::<bool>("no-act-like");
        let no_act_album = self.property::<bool>("no-act-album");
        let no_act_remove = self.property::<bool>("no-act-remove");

        // 相对已有行的全局起点，避免搜索分页追加时用本批下标错列。
        let base =
            Self::collect_list_rows(&left).len() + Self::collect_list_rows(&right).len();

        for (i, (si, like)) in sis.iter().zip(likes.iter()).enumerate() {
            let sender = sender.clone();

            let row = SonglistRow::new(sender.clone(), si);
            row.set_property("like", like);
            row.set_like_button_visible(!no_act_like);
            row.set_album_button_visible(!no_act_album);
            row.set_remove_button_visible(!no_act_remove);
            row.set_dual_column_compact(dual);

            let si = si.clone();
            gtk::prelude::ListBoxRowExt::connect_activate(
                &row,
                clone!(
                    #[weak(rename_to = s)]
                    self,
                    move |row| {
                        if row.is_activatable() || row.not_ignore_grey() {
                            s.clear_playing_except(row);
                            row.switch_image(true);
                            s.imp().playing_row.replace(Some(row.downgrade()));
                            sender.send_blocking(Action::AddPlay(si.clone())).unwrap();
                            s.emit_row_activated(row);
                        }
                    }
                ),
            );

            settings
                .bind("not-ignore-grey", &row, "not-ignore-grey")
                .get_only()
                .build();

            if Self::dual_column_is_right(dual, base + i) {
                right.append(&row);
            } else {
                left.append(&row);
            }
        }
    }

    fn clear_playing_except(&self, keep: &SonglistRow) {
        if let Some(prev) = self.imp().playing_row.borrow().as_ref().and_then(|w| w.upgrade()) {
            if prev != *keep {
                prev.switch_image(false);
            }
        }
    }

    fn collect_rows_in_order(&self) -> Vec<SonglistRow> {
        let imp = self.imp();
        let left = Self::collect_list_rows(&imp.listbox_left.get());
        let right = Self::collect_list_rows(&imp.listbox_right.get());
        if right.is_empty() {
            return left;
        }
        let mut rows = Vec::with_capacity(left.len() + right.len());
        let max = left.len().max(right.len());
        for i in 0..max {
            if let Some(row) = left.get(i) {
                rows.push(row.clone());
            }
            if let Some(row) = right.get(i) {
                rows.push(row.clone());
            }
        }
        rows
    }

    fn collect_list_rows(list: &ListBox) -> Vec<SonglistRow> {
        let mut rows = Vec::new();
        let mut child = list.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Ok(row) = widget.downcast::<SonglistRow>() {
                rows.push(row);
            }
        }
        rows
    }

    fn clear_listbox(list: &ListBox) {
        while let Some(child) = list.last_child() {
            list.remove(&child);
        }
    }

    /// 按当前宽/窄与 max-columns 重新分配已有行。
    fn redistribute_rows(&self) {
        let imp = self.imp();
        let dual = self.wants_dual();
        let left = imp.listbox_left.get();
        let right = imp.listbox_right.get();

        let rows = self.collect_rows_in_order();
        // 先全部摘下，避免仍挂在某一列上
        for row in &rows {
            if let Some(parent) = row.parent() {
                if let Ok(list) = parent.downcast::<ListBox>() {
                    list.remove(row);
                }
            }
        }

        right.set_visible(dual);
        for (i, row) in rows.into_iter().enumerate() {
            row.set_dual_column_compact(dual);
            if Self::dual_column_is_right(dual, i) {
                right.append(&row);
            } else {
                left.append(&row);
            }
        }
    }

    pub fn get_songinfo_list(&self) -> Vec<SongInfo> {
        self.collect_rows_in_order()
            .into_iter()
            .filter_map(|row| row.get_song_info())
            .collect()
    }

    pub fn clear_list(&self) {
        let imp = self.imp();
        Self::clear_listbox(&imp.listbox_left.get());
        Self::clear_listbox(&imp.listbox_right.get());
        imp.playing_row.replace(None);
    }

    fn row_at_global_index(&self, index: i32) -> Option<SonglistRow> {
        if index < 0 {
            return None;
        }
        let imp = self.imp();
        if self.wants_dual() && imp.listbox_right.is_visible() {
            let list = if index % 2 == 0 {
                imp.listbox_left.get()
            } else {
                imp.listbox_right.get()
            };
            list.row_at_index(index / 2)
                .and_then(|r| r.downcast::<SonglistRow>().ok())
        } else {
            imp.listbox_left
                .get()
                .row_at_index(index)
                .and_then(|r| r.downcast::<SonglistRow>().ok())
        }
    }

    pub fn mark_new_row_playing(&self, index: i32, do_active: bool) {
        if let Some(row) = self.row_at_global_index(index) {
            if do_active {
                gtk::prelude::ListBoxRowExt::emit_activate(&row);
            } else {
                self.clear_playing_except(&row);
                row.switch_image(true);
                self.imp().playing_row.replace(Some(row.downgrade()));
            }
            let list = row.parent().and_then(|p| p.downcast::<ListBox>().ok());
            if let Some(list) = list {
                list.emit_by_name_with_values("row-activated", &[row.to_value()]);
            }
        }
    }

    pub fn emit_row_activated(&self, row: &SonglistRow) {
        self.emit_by_name::<()>("row-activated", &[&row]);
    }

    pub fn connect_row_activated(&self, f: RustClosure) -> SignalHandlerId {
        self.connect_closure("row-activated", false, f)
    }

    fn setup_breakpoint(&self) {
        let imp = self.imp();
        let bin = imp.breakpoint_bin.get();

        let condition = adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            900.0,
            adw::LengthUnit::Sp,
        );
        let bp = adw::Breakpoint::new(condition);
        bp.add_setter(
            &imp.listbox_right.get(),
            "visible",
            Some(&false.to_value()),
        );

        bp.connect_apply(clone!(
            #[weak(rename_to = s)]
            self,
            move |_| {
                s.imp().wide_layout.set(false);
                if s.imp().max_columns.get() >= 2 {
                    s.redistribute_rows();
                }
            }
        ));
        bp.connect_unapply(clone!(
            #[weak(rename_to = s)]
            self,
            move |_| {
                s.imp().wide_layout.set(true);
                if s.imp().max_columns.get() >= 2 {
                    s.redistribute_rows();
                }
            }
        ));

        bin.add_breakpoint(bp);
    }
}

#[gtk::template_callbacks]
impl SongListView {}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/songlist-view.ui")]
    pub struct SongListView {
        #[template_child]
        pub scroll_win: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub adw_clamp: TemplateChild<adw::Clamp>,
        #[template_child]
        pub breakpoint_bin: TemplateChild<adw::BreakpointBin>,
        #[template_child]
        pub listbox_left: TemplateChild<ListBox>,
        #[template_child]
        pub listbox_right: TemplateChild<ListBox>,

        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,

        pub playing_row: RefCell<Option<glib::WeakRef<SonglistRow>>>,

        no_act_like: Cell<bool>,
        no_act_album: Cell<bool>,
        no_act_remove: Cell<bool>,
        /// 允许的最大列数（1=始终单列，2=宽屏双列）
        pub max_columns: Cell<i32>,
        /// 断点判定为宽屏（未命中 max-width: 900sp）
        pub wide_layout: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongListView {
        const NAME: &'static str = "SongListView";
        type Type = super::SongListView;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_callbacks();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SongListView {}

    impl ObjectImpl for SongListView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.max_columns.set(2);
            self.wide_layout.set(true);

            obj.setup_settings();
            obj.setup_breakpoint();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("row-activated")
                        .param_types([SonglistRow::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecBoolean::builder("no-act-remove").build(),
                    ParamSpecInt::builder("max-columns")
                        .minimum(1)
                        .maximum(2)
                        .default_value(2)
                        .build(),
                    ParamSpecInt::builder("clamp-margin-top").build(),
                    ParamSpecInt::builder("clamp-margin-bottom").build(),
                    ParamSpecInt::builder("clamp-margin-start").build(),
                    ParamSpecInt::builder("clamp-margin-end").build(),
                    ParamSpecInt::builder("clamp-maximum-size").build(),
                    ParamSpecInt::builder("clamp-tightening-threshold").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "no-act-like" => {
                    let val = value.get().unwrap();
                    self.no_act_like.replace(val);
                }
                "no-act-album" => {
                    let val = value.get().unwrap();
                    self.no_act_album.replace(val);
                }
                "no-act-remove" => {
                    let val = value.get().unwrap();
                    self.no_act_remove.replace(val);
                }
                "max-columns" => {
                    let val: i32 = value.get().unwrap();
                    let val = val.clamp(1, 2);
                    let prev = self.max_columns.replace(val);
                    if prev != val {
                        let obj = self.obj();
                        if val < 2 {
                            self.listbox_right.set_visible(false);
                        }
                        obj.redistribute_rows();
                    }
                }
                "clamp-margin-top" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_top(val);
                }
                "clamp-margin-bottom" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_bottom(val);
                }
                "clamp-margin-start" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_start(val);
                }
                "clamp-margin-end" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_end(val);
                }
                "clamp-maximum-size" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_maximum_size(val);
                }
                "clamp-tightening-threshold" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_tightening_threshold(val);
                }
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "no-act-like" => self.no_act_like.get().to_value(),
                "no-act-album" => self.no_act_album.get().to_value(),
                "no-act-remove" => self.no_act_remove.get().to_value(),
                "max-columns" => self.max_columns.get().to_value(),
                "clamp-margin-top" => self.adw_clamp.margin_top().to_value(),
                "clamp-margin-bottom" => self.adw_clamp.margin_bottom().to_value(),
                "clamp-margin-start" => self.adw_clamp.margin_start().to_value(),
                "clamp-margin-end" => self.adw_clamp.margin_end().to_value(),
                "clamp-maximum-size" => self.adw_clamp.maximum_size().to_value(),
                "clamp-tightening-threshold" => self.adw_clamp.tightening_threshold().to_value(),
                n => unimplemented!("{}", n),
            }
        }
    }
    impl WidgetImpl for SongListView {}
    impl BoxImpl for SongListView {}
}

#[cfg(test)]
mod tests {
    use super::SongListView;

    #[test]
    fn dual_column_uses_global_parity_not_batch_index() {
        // 单列：一律左列
        assert!(!SongListView::dual_column_is_right(false, 0));
        assert!(!SongListView::dual_column_is_right(false, 1));

        // 双列：偶左奇右
        assert!(!SongListView::dual_column_is_right(true, 0));
        assert!(SongListView::dual_column_is_right(true, 1));
        assert!(!SongListView::dual_column_is_right(true, 2));

        // 已有奇数行（base=1）再追加：本批 i=0 的全局下标为 1，应进右列
        let base = 1usize;
        assert!(SongListView::dual_column_is_right(true, base));
        assert!(!SongListView::dual_column_is_right(true, base + 1));
    }
}
