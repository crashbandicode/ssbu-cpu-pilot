//! P1-as-CPU core for Super Smash Bros. Ultimate 13.0.5 Skyline plugins.
//!
//! Everything here except `native` is pure and host-tested: the three
//! settings and their INI form, the L+R+X level cycle, the level-to-rank
//! ladder, the AI-to-pad encoding, the per-frame pilot session and the HUD
//! label. `native` (feature `native`, Switch only) holds the raw game calls;
//! it has no Skyline dependency, so a host with its own Skyline version (the
//! Training Modpack) can use it and install the hooks itself.
pub mod combo;
pub mod hud;
pub mod level;
#[cfg(all(feature = "native", target_os = "switch"))]
pub mod native;
pub mod pad;
pub mod session;
pub mod settings;

pub mod results;
