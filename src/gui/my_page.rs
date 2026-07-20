//
// my_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use async_channel::Sender;
use gio::Settings;
use gtk::{CompositeTemplate, gio, glib, prelude::*, subclass::prelude::*};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

use crate::{
    APP_ID,
    application::Action,
    gui::{SongListGridItem, songlist_row::SonglistRow},
    model::{MyPageRequestId, MyPageRequestTokens, MyPageSection},
};

glib::wrapper! {
    pub struct MyPage(ObjectSubclass<imp::MyPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl MyPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn reset(&self) {
        let imp = self.imp();
        imp.request_tokens.borrow_mut().invalidate_all();
        self.clear_active_preview_row();
        for list in [
            imp.daily_left.get(),
            imp.daily_right.get(),
            imp.favorite_songs_left.get(),
            imp.favorite_songs_right.get(),
        ] {
            Self::clear_listbox(&list);
        }
        SongListGridItem::box_clear(imp.albums_grid.get());
        SongListGridItem::box_clear(imp.songlists_grid.get());
        imp.albums.borrow_mut().clear();
        imp.songlists.borrow_mut().clear();
        for section in MyPageSection::ALL {
            self.set_section_state(section, "loading");
        }
    }

    pub fn invalidate_requests(&self) {
        self.imp().request_tokens.borrow_mut().invalidate_all();
    }

    pub fn begin_request(&self, section: MyPageSection) -> MyPageRequestId {
        let request_id = self.imp().request_tokens.borrow_mut().begin(section);
        self.set_section_state(section, "loading");
        request_id
    }

    pub fn is_current_request(&self, section: MyPageSection, request_id: MyPageRequestId) -> bool {
        self.imp()
            .request_tokens
            .borrow()
            .is_current(section, request_id)
    }

    pub fn set_failed(&self, section: MyPageSection) {
        self.set_section_state(section, "error");
    }

    pub fn update_songs(&self, section: MyPageSection, songs: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let (left, right) = match section {
            MyPageSection::DailyRec => (imp.daily_left.get(), imp.daily_right.get()),
            MyPageSection::FavoriteSongs => (
                imp.favorite_songs_left.get(),
                imp.favorite_songs_right.get(),
            ),
            _ => return,
        };

        self.clear_active_preview_row();
        Self::clear_listbox(&left);
        Self::clear_listbox(&right);
        if songs.is_empty() {
            self.set_section_state(section, "empty");
            return;
        }

        let split = songs.len().min(4);
        Self::set_song_column_visible(&left, true);
        Self::set_song_column_visible(&right, songs.len() > split);
        self.fill_song_list(&left, &songs[..split], &likes[..split]);
        self.fill_song_list(&right, &songs[split..], &likes[split..]);
        self.set_section_state(section, "content");
    }

    pub fn update_collections(&self, section: MyPageSection, items: Vec<SongList>) {
        let imp = self.imp();
        let (grid, show_author) = match section {
            MyPageSection::FavoriteAlbums => (imp.albums_grid.get(), true),
            MyPageSection::FavoriteSongLists => (imp.songlists_grid.get(), false),
            _ => return,
        };

        SongListGridItem::box_clear(grid.clone());
        match section {
            MyPageSection::FavoriteAlbums => imp.albums.borrow_mut().clear(),
            MyPageSection::FavoriteSongLists => imp.songlists.borrow_mut().clear(),
            _ => unreachable!(),
        }
        if items.is_empty() {
            self.set_section_state(section, "empty");
            return;
        }

        let sender = imp.sender.get().unwrap();
        SongListGridItem::box_update_songlist(grid, &items, 140, show_author, sender);
        match section {
            MyPageSection::FavoriteAlbums => imp.albums.replace(items),
            MyPageSection::FavoriteSongLists => imp.songlists.replace(items),
            _ => unreachable!(),
        };
        self.set_section_state(section, "content");
    }

    fn fill_song_list(&self, list: &gtk::ListBox, songs: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().clone();
        let settings = imp.settings.get().unwrap();

        for (song, like_song) in songs.iter().zip(likes.iter()) {
            let row = SonglistRow::new(sender.clone(), song);
            row.set_property("like", like_song);
            row.set_my_page_preview_mode();
            settings
                .bind("not-ignore-grey", &row, "not-ignore-grey")
                .get_only()
                .build();

            let song = song.clone();
            gtk::prelude::ListBoxRowExt::connect_activate(
                &row,
                glib::clone!(
                    #[weak(rename_to = page)]
                    self,
                    #[strong]
                    sender,
                    move |row| {
                        if row.is_activatable() || row.not_ignore_grey() {
                            page.activate_preview_row(row);
                            sender.send_blocking(Action::AddPlay(song.clone())).unwrap();
                        }
                    }
                ),
            );
            list.append(&row);
        }
    }

    fn activate_preview_row(&self, row: &SonglistRow) {
        let imp = self.imp();
        if let Some(old_row) = imp
            .active_preview_row
            .borrow()
            .as_ref()
            .and_then(|row| row.upgrade())
        {
            old_row.switch_image(false);
        }
        row.switch_image(true);
        imp.active_preview_row.replace(Some(row.downgrade()));
    }

    fn clear_active_preview_row(&self) {
        let active_row = self.imp().active_preview_row.borrow_mut().take();
        if let Some(row) = active_row.and_then(|row| row.upgrade()) {
            row.switch_image(false);
        }
    }

    fn set_section_state(&self, section: MyPageSection, state: &str) {
        let imp = self.imp();
        let (stack, more_button) = match section {
            MyPageSection::DailyRec => (imp.daily_state.get(), imp.daily_more_button.get()),
            MyPageSection::FavoriteSongs => (
                imp.favorite_songs_state.get(),
                imp.favorite_songs_more_button.get(),
            ),
            MyPageSection::FavoriteAlbums => (imp.albums_state.get(), imp.albums_more_button.get()),
            MyPageSection::FavoriteSongLists => {
                (imp.songlists_state.get(), imp.songlists_more_button.get())
            }
        };
        stack.set_visible_child_name(state);
        more_button.set_sensitive(state == "content");
    }

    fn clear_listbox(list: &gtk::ListBox) {
        while let Some(child) = list.last_child() {
            list.remove(&child);
        }
    }

    /// Hide/show the FlowBoxChild wrapper so homogeneous FlowBox does not
    /// reserve an empty column when the right list has no songs.
    fn set_song_column_visible(list: &gtk::ListBox, visible: bool) {
        match list
            .parent()
            .and_then(|parent| parent.downcast::<gtk::FlowBoxChild>().ok())
        {
            Some(wrapper) => wrapper.set_visible(visible),
            None => list.set_visible(visible),
        }
    }
}

impl Default for MyPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/my-page.ui")]
    pub struct MyPage {
        #[template_child]
        pub daily_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub favorite_songs_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub albums_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub songlists_state: TemplateChild<gtk::Stack>,
        #[template_child]
        pub daily_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub favorite_songs_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub albums_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub songlists_more_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub daily_left: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub daily_right: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub favorite_songs_left: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub favorite_songs_right: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub albums_grid: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub songlists_grid: TemplateChild<gtk::FlowBox>,

        pub albums: RefCell<Vec<SongList>>,
        pub songlists: RefCell<Vec<SongList>>,
        pub active_preview_row: RefCell<Option<glib::WeakRef<SonglistRow>>>,
        pub request_tokens: RefCell<MyPageRequestTokens>,
        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MyPage {
        const NAME: &'static str = "MyPage";
        type Type = super::MyPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl MyPage {
        #[template_callback]
        fn daily_rec_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageDailyRec)
                .unwrap();
        }

        #[template_callback]
        fn heartbeat_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageHeartbeat)
                .unwrap();
        }

        #[template_callback]
        fn collection_album_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageAlbums)
                .unwrap();
        }

        #[template_callback]
        fn collection_songlist_cb(&self) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::ToMyPageSonglist)
                .unwrap();
        }

        #[template_callback]
        fn retry_daily_cb(&self) {
            self.load_section(MyPageSection::DailyRec);
        }

        #[template_callback]
        fn retry_favorite_songs_cb(&self) {
            self.load_section(MyPageSection::FavoriteSongs);
        }

        #[template_callback]
        fn retry_albums_cb(&self) {
            self.load_section(MyPageSection::FavoriteAlbums);
        }

        #[template_callback]
        fn retry_songlists_cb(&self) {
            self.load_section(MyPageSection::FavoriteSongLists);
        }

        fn load_section(&self, section: MyPageSection) {
            self.sender
                .get()
                .unwrap()
                .send_blocking(Action::LoadMyPageSection(section))
                .unwrap();
        }
    }

    impl ObjectImpl for MyPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.settings.set(Settings::new(APP_ID)).unwrap();
            let obj = self.obj();

            self.albums_grid.connect_child_activated(glib::clone!(
                #[weak]
                obj,
                move |_, child| {
                    let imp = obj.imp();
                    if let Some(item) = imp.albums.borrow().get(child.index() as usize) {
                        imp.sender
                            .get()
                            .unwrap()
                            .send_blocking(Action::ToAlbumPage(item.clone()))
                            .unwrap();
                    }
                }
            ));
            self.songlists_grid.connect_child_activated(glib::clone!(
                #[weak]
                obj,
                move |_, child| {
                    let imp = obj.imp();
                    if let Some(item) = imp.songlists.borrow().get(child.index() as usize) {
                        imp.sender
                            .get()
                            .unwrap()
                            .send_blocking(Action::ToSongListPage(item.clone()))
                            .unwrap();
                    }
                }
            ));
        }
    }
    impl WidgetImpl for MyPage {}
    impl BoxImpl for MyPage {}
}
