//! The per-frame pilot session: when to arm P1's AI at a level, when to let
//! it think and drive the pad, and when to release it.
//!
//! The host calls `step` once per battle frame for P1 with what it observed.
//! Arming sets the AI's rank (and the host saves the old rank); releasing
//! restores it. Arm and Rearm also think on the same frame. Any frame without
//! `Think`/`Arm`/`Rearm` leaves P1's physical input untouched, so turning
//! the setting off returns the pad on the next frame.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Frame {
    /// `Some(level)` while the setting says the CPU drives P1.
    pub desired: Option<u8>,
    /// A supported battle with P1 bound (not results, not a scene change).
    pub in_battle: bool,
    /// The bound controller and AI are the ones that were armed. Only read
    /// while armed.
    pub same_identity: bool,
    /// Ready-go, the native AI ready, and no menu holding input.
    pub can_think: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Restore the saved rank (when the objects are still valid); stop driving.
    Release,
    /// Save the rank, arm at this level, then think.
    Arm(u8),
    /// Restore the old rank, arm at this level, then think.
    Rearm(u8),
    /// Think and drive the pad this frame.
    Think,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Session {
    armed: Option<u8>,
}

impl Session {
    pub const fn new() -> Self {
        Self { armed: None }
    }

    /// The level P1's AI is armed at, if any. The HUD mark shows while this
    /// is `Some`.
    pub const fn armed(&self) -> Option<u8> {
        self.armed
    }

    pub fn step(&mut self, frame: Frame) -> Action {
        if let Some(armed) = self.armed {
            if !frame.in_battle || !frame.same_identity || frame.desired.is_none() {
                self.armed = None;
                return Action::Release;
            }
            let level = frame.desired.unwrap_or(armed);
            if !frame.can_think {
                return Action::None;
            }
            if level != armed {
                self.armed = Some(level);
                return Action::Rearm(level);
            }
            return Action::Think;
        }
        match frame.desired {
            Some(level) if frame.in_battle && frame.can_think => {
                self.armed = Some(level);
                Action::Arm(level)
            }
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn f(desired: Option<u8>, in_battle: bool, same: bool, think: bool) -> Frame {
        Frame {
            desired,
            in_battle,
            same_identity: same,
            can_think: think,
        }
    }

    fn run(frames: &[Frame]) -> Vec<Action> {
        let mut s = Session::new();
        frames.iter().map(|&x| s.step(x)).collect()
    }

    #[test]
    fn off_does_nothing() {
        assert_eq!(run(&[f(None, true, true, true); 3]), [Action::None; 3]);
    }

    #[test]
    fn arms_when_ready_then_thinks_then_releases_on_off() {
        let a = run(&[
            f(Some(3), true, true, false),
            f(Some(3), true, true, true),
            f(Some(3), true, true, true),
            f(None, true, true, true),
            f(None, true, true, true),
        ]);
        assert_eq!(
            a,
            [Action::None, Action::Arm(3), Action::Think, Action::Release, Action::None]
        );
    }

    #[test]
    fn toggled_before_the_match_waits_for_battle() {
        let a = run(&[f(Some(5), false, true, true), f(Some(5), true, true, true)]);
        assert_eq!(a, [Action::None, Action::Arm(5)]);
    }

    #[test]
    fn pause_keeps_arming_but_does_not_drive() {
        let a = run(&[
            f(Some(3), true, true, true),
            f(Some(3), true, true, false),
            f(Some(3), true, true, true),
        ]);
        assert_eq!(a, [Action::Arm(3), Action::None, Action::Think]);
    }

    #[test]
    fn level_change_rearms_once_ready() {
        let a = run(&[
            f(Some(3), true, true, true),
            f(Some(4), true, true, false),
            f(Some(4), true, true, true),
            f(Some(4), true, true, true),
        ]);
        assert_eq!(a, [Action::Arm(3), Action::None, Action::Rearm(4), Action::Think]);
    }

    #[test]
    fn match_end_or_identity_change_releases_and_rearms_next_battle() {
        let a = run(&[
            f(Some(3), true, true, true),
            f(Some(3), false, true, true),
            f(Some(3), false, true, true),
            f(Some(3), true, true, true),
            f(Some(3), true, false, true),
            f(Some(3), true, true, true),
        ]);
        assert_eq!(
            a,
            [
                Action::Arm(3),
                Action::Release,
                Action::None,
                Action::Arm(3),
                Action::Release,
                Action::Arm(3)
            ]
        );
    }

    #[test]
    fn armed_reports_level_for_the_hud() {
        let mut s = Session::new();
        assert_eq!(s.armed(), None);
        s.step(f(Some(7), true, true, true));
        assert_eq!(s.armed(), Some(7));
        s.step(f(None, true, true, true));
        assert_eq!(s.armed(), None);
    }
}
