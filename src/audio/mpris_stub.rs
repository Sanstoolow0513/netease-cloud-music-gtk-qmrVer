//
// mpris_stub.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use anyhow::{Result, anyhow};
use ncm_api::SongInfo;

use crate::gui::PlayerControls;

use super::LoopsState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct MprisController;

impl MprisController {
    pub async fn new() -> Result<Self> {
        Err(anyhow!("MPRIS is only available on Linux"))
    }

    pub async fn update_metadata(&self, _song: &SongInfo) -> Result<()> {
        Ok(())
    }

    pub async fn set_volume(&self, _volume: f64) -> Result<()> {
        Ok(())
    }

    pub async fn set_playback_status(&self, _state: PlaybackStatus) -> Result<()> {
        Ok(())
    }

    pub fn get_loop_status(&self) -> Result<LoopsState> {
        Ok(LoopsState::None)
    }

    pub async fn set_loop_status(&self, _status: LoopsState) -> Result<()> {
        Ok(())
    }

    pub fn set_position(&self, _value: i64) {}

    pub async fn seeked(&self, _value: i64) -> Result<()> {
        Ok(())
    }

    pub fn setup_signals(&self, _player_controls: &PlayerControls) {}
}
