//! ssbu-cpu-pilot: on demand, Smash Ultimate 13.0.5's native CPU drives P1's
//! pad (Training and local battles), with three rows on Online Deluxe's
//! Plugins page: P1 Player/CPU (L+R+X cycles the CPU level), automatic
//! rematch and automatic replay saving. The logic lives in the
//! `ssbu-cpu-pilot-core` crate (core/), which other plugins can depend on.
pub mod chain;
pub mod model;
#[cfg(target_os = "switch")]
mod runtime;
pub mod share;
