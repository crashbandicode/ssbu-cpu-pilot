//! Online Deluxe's window state, read through its export
//! `ssbu_online_deluxe_overlay_state_v1` (Deluxe 0.10.5+).
//!
//! While Deluxe's window is visible, an L+R+X press belongs to the window:
//! Deluxe toggles what is highlighted, or on the pilot's P1 row asks for the
//! current choice (`settings::p1_request` steps the level). So P1's pad stands
//! down then, and Deluxe's request is the one step. An older Deluxe has no
//! export: a missing symbol means "no window", and the pad steps as in 0.2.3.
//!
//! The host looks the symbol up off the game thread, once and again on a miss
//! at most once a second ([`Retry`]); hooks only call the address it found.

/// The export's out value. `size` is 8 for this version.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeluxeOverlayStateV1 {
    pub size: u32,
    pub flags: u32,
}

/// `extern "C" fn(out: *mut DeluxeOverlayStateV1) -> bool`: fills `*out` and
/// returns true; false if `out` is null. Lock-free, any game thread.
pub const SYMBOL: &[u8] = b"ssbu_online_deluxe_overlay_state_v1\0";
/// Deluxe's window is drawn (any page).
pub const WINDOW_VISIBLE: u32 = 1 << 0;
/// The shown page has selectable or editable rows.
pub const ROWS_PAGE: u32 = 1 << 1;
/// Deluxe's Debug view is open.
pub const DEBUG_VIEW: u32 = 1 << 2;

const V1_SIZE: u32 = core::mem::size_of::<DeluxeOverlayStateV1>() as u32;

/// The window is visible, given what the export returned (`None`: no export,
/// or it returned false). A value shorter than version 1 counts as no window.
pub const fn window_visible(answer: Option<DeluxeOverlayStateV1>) -> bool {
    match answer {
        Some(state) => state.size >= V1_SIZE && state.flags & WINDOW_VISIBLE != 0,
        None => false,
    }
}

/// Whether P1's pad steps the level on an L+R+X press: P1 is set to CPU and
/// Deluxe's window is not visible.
pub const fn pad_steps(cpu: bool, window: bool) -> bool {
    cpu && !window
}

/// Whether Deluxe's request for the P1 row's current choice (its L+R+X) steps
/// the level. Not while battle frames run with the window seen as hidden:
/// that is an older Deluxe without the export, and P1's pad already stepped
/// for the press (0.2.1).
pub const fn request_steps(battle_frames: bool, window: bool) -> bool {
    !battle_frames || window
}

/// When to look the export up: at the first chance, then after a miss at most
/// once per `interval` (in the host's ticks). The host stops asking once it
/// has the address.
#[derive(Clone, Copy, Debug, Default)]
pub struct Retry {
    next: Option<u64>,
}

impl Retry {
    pub const fn new() -> Self {
        Self { next: None }
    }

    /// True if a lookup is due at `now`; it then counts as made.
    pub fn due(&mut self, now: u64, interval: u64) -> bool {
        if self.next.is_some_and(|next| now < next) {
            return false;
        }
        self.next = Some(now.saturating_add(interval));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combo::{next_level, Combo};
    use crate::settings::{p1_choice, p1_request, Settings};

    fn state(flags: u32) -> Option<DeluxeOverlayStateV1> {
        Some(DeluxeOverlayStateV1 { size: 8, flags })
    }

    #[test]
    fn layout_is_the_contract() {
        assert_eq!(core::mem::size_of::<DeluxeOverlayStateV1>(), 8);
        assert_eq!(core::mem::align_of::<DeluxeOverlayStateV1>(), 4);
        assert_eq!((WINDOW_VISIBLE, ROWS_PAGE, DEBUG_VIEW), (1, 2, 4));
        assert_eq!(SYMBOL.last(), Some(&0));
        assert!(!SYMBOL[..SYMBOL.len() - 1].contains(&0));
    }

    #[test]
    fn only_the_visible_bit_counts() {
        assert!(window_visible(state(WINDOW_VISIBLE)));
        assert!(window_visible(state(WINDOW_VISIBLE | ROWS_PAGE | DEBUG_VIEW)));
        assert!(!window_visible(state(0)));
        assert!(!window_visible(state(ROWS_PAGE | DEBUG_VIEW)));
    }

    #[test]
    fn missing_export_or_short_answer_is_no_window() {
        assert!(!window_visible(None));
        let short = DeluxeOverlayStateV1 { size: 4, flags: WINDOW_VISIBLE };
        assert!(!window_visible(Some(short)));
        let longer = DeluxeOverlayStateV1 { size: 16, flags: WINDOW_VISIBLE };
        assert!(window_visible(Some(longer)));
    }

    #[test]
    fn pad_stands_down_while_the_window_is_visible() {
        assert!(pad_steps(true, false));
        assert!(!pad_steps(true, true));
        assert!(!pad_steps(false, false));
        assert!(!pad_steps(false, true));
    }

    /// Steps from one L+R+X press: the pad's (in battle) plus Deluxe's request
    /// (when its window shows the P1 row), each by its own rule.
    fn steps(battle: bool, window: bool, forwarded: bool) -> u8 {
        let s = Settings { pilot: true, level: 5, ..Settings::default() };
        let mut now = s;
        let mut combo = Combo::new();
        if battle && combo.step(true, pad_steps(now.pilot, window)) {
            now = Settings { level: next_level(now.level), ..now };
        }
        if forwarded && request_steps(battle, window) {
            now = p1_request(now, p1_choice(now));
        }
        (now.level + 9 - s.level) % 9
    }

    #[test]
    fn one_press_one_step_on_the_p1_row() {
        // Deluxe 0.10.5+: the window is visible whenever it forwards.
        assert_eq!(steps(true, true, true), 1);
        assert_eq!(steps(false, true, true), 1);
        // An older Deluxe forwards too, but reads as no window (0.2.3).
        assert_eq!(steps(true, false, true), 1);
        assert_eq!(steps(false, false, true), 1);
    }

    #[test]
    fn the_pad_alone_steps_once_and_not_under_the_window() {
        assert_eq!(steps(true, false, false), 1);
        // Visible on another page or row: the press is Deluxe's.
        assert_eq!(steps(true, true, false), 0);
    }

    #[test]
    fn press_held_through_the_window_closing_does_not_step() {
        let mut combo = Combo::new();
        assert!(!combo.step(true, pad_steps(true, true)));
        assert!(!combo.step(true, pad_steps(true, false)));
        assert!(!combo.step(false, pad_steps(true, false)));
        assert!(combo.step(true, pad_steps(true, false)));
    }

    #[test]
    fn lookup_once_then_at_most_once_per_interval() {
        let second = 19_200_000;
        let mut retry = Retry::new();
        // The worker's 200 ms rounds over three seconds.
        let due: Vec<u64> = (0..=15)
            .map(|round| round * second / 5)
            .filter(|&now| retry.due(now, second))
            .collect();
        assert_eq!(due, [0, second, 2 * second, 3 * second]);
    }

    #[test]
    fn first_lookup_is_due_at_any_tick() {
        let mut retry = Retry::new();
        assert!(retry.due(123, 10));
        assert!(!retry.due(123, 10));
        assert!(!retry.due(132, 10));
        assert!(retry.due(133, 10));
    }
}
