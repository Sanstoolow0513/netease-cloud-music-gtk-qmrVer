//
// system_tray_stub.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use async_channel::Sender;

use crate::application::Action;

#[derive(Debug, Default)]
pub struct TrayHandle;

impl TrayHandle {
    pub fn start(&mut self, _sender: Sender<Action>) {}

    pub fn stop(&mut self) {}

    pub fn update_playing(&self, _playing: bool) {}

    pub fn update_song_title(&self, _title: String, _artist: String, _album_id: u64) {}

    pub fn is_running(&self) -> bool {
        false
    }
}
