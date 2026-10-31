//! Online admission and local-pad ownership. Entry indices are not pad slots.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Offline,
    Online1v1,
    Unsupported,
}

/// Only the fork's audited online battle scope; never its waiting practice,
/// other online modes, or an uninitialized game-mode singleton.
pub const fn scope(initialized: bool, melee: u8, mode: u32) -> Scope {
    if !initialized {
        Scope::Unsupported
    } else if melee == 0x0e && mode == 0x07010101 {
        Scope::Online1v1
    } else if melee == 0x0e || mode >> 24 == 7 {
        Scope::Unsupported
    } else {
        Scope::Offline
    }
}

pub fn local_slot(mask: u8) -> Option<u8> {
    (mask.count_ones() == 1).then(|| mask.trailing_zeros() as u8)
}

/// Exactly two entries, exactly one matching the owned slot. The caller
/// separately validates that both entries are human, loaded native objects.
pub fn local_entry(mask: u8, slots: &[Option<u8>; 8]) -> Option<usize> {
    let slot = local_slot(mask)?;
    let mut count = 0;
    let mut found = None;
    for (index, value) in slots.iter().enumerate() {
        if let Some(value) = value {
            if *value >= 8 {
                return None;
            }
            count += 1;
            if *value == slot {
                if found.is_some() {
                    return None;
                }
                found = Some(index);
            }
        }
    }
    if count == 2 { found } else { None }
}

/// A manager lease is scene-wide, not pilot-wide: after successful analysis,
/// turning the pilot off cannot undo the native phase transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Pending(u8),
    Ready,
    Stopped,
}

impl Stage {
    pub fn start(wanted: bool, human_only: bool, phase: i32, d0: u8) -> Option<Self> {
        (wanted && human_only && phase == 1 && d0 & 1 == 0).then_some(Self::Pending(0))
    }

    /// True means isolate this entire manager call and temporarily supply d0.
    pub fn before(&mut self, wanted: bool) -> bool {
        match *self {
            Self::Ready => true,
            Self::Pending(frames) if wanted && frames < 120 => {
                *self = Self::Pending(frames + 1);
                true
            }
            _ => {
                *self = Self::Stopped;
                false
            }
        }
    }

    pub fn after(&mut self, phase: i32) {
        if phase == 2 {
            *self = Self::Ready;
        } else if matches!(self, Self::Pending(120)) {
            *self = Self::Stopped;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_excludes_waiting_and_other_online_modes() {
        assert_eq!(scope(true, 0x0e, 0x07010101), Scope::Online1v1);
        assert_eq!(scope(false, 0x0e, 0x07010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0e, 0x07010102), Scope::Unsupported);
        assert_eq!(scope(true, 0x0c, 0x07010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0c, 0x01010101), Scope::Offline);
    }

    #[test]
    fn every_mask_elects_only_one_slot() {
        for mask in 0..=u8::MAX {
            let selected = local_slot(mask);
            assert_eq!(selected.is_some(), mask.count_ones() == 1);
            if let Some(slot) = selected {
                assert_eq!(mask, 1 << slot);
            }
        }
    }

    #[test]
    fn slot_is_not_entry_index_and_pair_must_be_unique() {
        let mut slots = [None; 8];
        slots[1] = Some(5);
        slots[6] = Some(2);
        assert_eq!(local_entry(1 << 5, &slots), Some(1));
        assert_eq!(local_entry(1 << 2, &slots), Some(6));
        assert_eq!(local_entry(1, &slots), None);
        assert_eq!(local_entry((1 << 2) | (1 << 5), &slots), None);
        slots[0] = Some(0);
        assert_eq!(local_entry(1 << 5, &slots), None);
        slots[0] = None;
        slots[6] = Some(5);
        assert_eq!(local_entry(1 << 5, &slots), None);
        slots[6] = None;
        assert_eq!(local_entry(1 << 5, &slots), None);
    }

    #[test]
    fn stage_needs_intent_and_no_existing_cpu() {
        assert_eq!(Stage::start(true, true, 1, 0), Some(Stage::Pending(0)));
        assert_eq!(Stage::start(false, true, 1, 0), None);
        assert_eq!(Stage::start(true, false, 1, 0), None);
        assert_eq!(Stage::start(true, true, 2, 0), None);
        assert_eq!(Stage::start(true, true, 1, 1), None);
    }

    #[test]
    fn successful_stage_keeps_rng_lease_after_pilot_off() {
        let mut stage = Stage::Pending(0);
        assert!(stage.before(true));
        stage.after(2);
        assert!(stage.before(false));
        assert_eq!(stage, Stage::Ready);
        // Online d1 rechecks stage analysis; keep the lease on a regression.
        stage.after(1);
        assert!(stage.before(false));
    }

    #[test]
    fn stage_attempts_are_bounded_and_cancellation_is_terminal() {
        let mut stage = Stage::Pending(0);
        for _ in 0..120 {
            assert!(stage.before(true));
            stage.after(1);
        }
        assert_eq!(stage, Stage::Stopped);
        assert!(!stage.before(true));
        let mut stage = Stage::Pending(0);
        assert!(!stage.before(false));
        assert!(!stage.before(true));
        let mut stage = Stage::Pending(119);
        assert!(stage.before(true));
        stage.after(2);
        assert_eq!(stage, Stage::Ready);
    }
}
