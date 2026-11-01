//! ssbu-cpu-pilot: on demand, Smash Ultimate 13.0.5's native CPU drives P1's
//! pad (Training, local and online battles), with three rows on Online
//! Deluxe's CPU Pilot page: P1 (`Player`, `CPU (L1)` … `CPU (L9)`; ZR+Left/Right
//! toggles, L+R+X steps the level), automatic rematch and automatic replay
//! saving. The logic lives in the
//! `ssbu-cpu-pilot-core` crate (core/), which other plugins can depend on.
pub mod chain;
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
pub mod share;

#[cfg(target_os = "switch")]
mod results;
