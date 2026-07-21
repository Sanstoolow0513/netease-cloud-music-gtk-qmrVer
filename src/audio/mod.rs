//
// mod.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

#[cfg(target_os = "linux")]
mod mpris;
#[cfg(not(target_os = "linux"))]
mod mpris_stub;
mod playlist;

#[cfg(target_os = "linux")]
pub use mpris::*;
#[cfg(not(target_os = "linux"))]
pub use mpris_stub::*;
pub use playlist::*;
