//! L+R+X: cycles the CPU level while P1 is set to CPU.
//!
//! The host passes whether all three physical buttons are held this frame
//! (read before the plugin overwrites P1's input) and whether the combo may
//! act now (the plugin passes `deluxe::pad_steps`: P1 is set to CPU and Online
//! Deluxe's window shows no page with rows). One press, one step: the combo fires on
//! the frame all three become held, never again until one is released. Online
//! Deluxe 0.10.1+ sends the same press on the P1 row as a request for its
//! current choice (`settings::p1_request`, `deluxe::request_steps`).

#[derive(Clone, Copy, Debug, Default)]
pub struct Combo {
    held: bool,
}

impl Combo {
    pub const fn new() -> Self {
        Self { held: false }
    }

    /// True on the rising edge of `held`, when `enabled`. A press made while
    /// disabled does not fire later when it becomes enabled.
    pub fn step(&mut self, held: bool, enabled: bool) -> bool {
        let rising = held && !self.held;
        self.held = held;
        rising && enabled
    }
}

/// 1, 2, ... 9, then back to 1.
pub const fn next_level(level: u8) -> u8 {
    if level >= crate::settings::LEVEL_MAX || level < crate::settings::LEVEL_MIN {
        crate::settings::LEVEL_MIN
    } else {
        level + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_once_per_press() {
        let mut c = Combo::new();
        let fired: Vec<bool> = [false, true, true, true, false, true]
            .iter()
            .map(|&h| c.step(h, true))
            .collect();
        assert_eq!(fired, [false, true, false, false, false, true]);
    }

    #[test]
    fn press_held_through_enable_does_not_fire() {
        let mut c = Combo::new();
        assert!(!c.step(true, false));
        assert!(!c.step(true, true));
        assert!(!c.step(false, true));
        assert!(c.step(true, true));
    }

    #[test]
    fn level_cycle_wraps() {
        let seq: Vec<u8> = (1..=9).map(next_level).collect();
        assert_eq!(seq, [2, 3, 4, 5, 6, 7, 8, 9, 1]);
        assert_eq!(next_level(0), 1);
    }
}
