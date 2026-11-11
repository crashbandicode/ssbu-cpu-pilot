//! Online admission and local-pad ownership. Entry indices are not pad slots.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Offline,
    /// The practice in Quickplay's (and Elite Smash's) waiting room while
    /// matchmaking runs. P1 is driven there as in Training (`route`); the
    /// battle a found match starts is `Online1v1`.
    WaitingPractice,
    Online1v1,
    Unsupported,
}

/// What the final-input-mapping hook does with one controller slot's mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// Not P1's mapping in a scope where only P1 is driven: left alone.
    Skip,
    /// P1 (FighterEntry 0 on controller slot 0) and its native AI, as in Training.
    LocalP1,
    /// The online battle: local slot election, then the analysis lease.
    Online,
    /// The pilot never drives here (it still releases an armed session).
    Off,
}

pub const fn route(scope: Scope, slot: u8) -> Route {
    match scope {
        Scope::Offline | Scope::WaitingPractice => {
            if slot == 0 {
                Route::LocalP1
            } else {
                Route::Skip
            }
        }
        Scope::Online1v1 => Route::Online,
        Scope::Unsupported => Route::Off,
    }
}

/// An armed pilot belongs to the scope it armed in: it keeps driving, or
/// restores the rank of, the objects it armed only while the scope is the
/// same. When a match is found the waiting practice is torn down and the
/// battle (`Online1v1`) arms afresh on its own objects; the practice's are
/// never touched again, whatever their addresses.
pub const fn same_scene(armed_in: Scope, now: Scope) -> bool {
    armed_in as u8 == now as u8
}

/// Quickplay's mode word (`0x53050f0`).
pub const QUICKPLAY_MODE: u32 = 0x0701_0101;
/// Elite Smash: the game promotes the Quickplay mode word to this value.
pub const ELITE_MODE: u32 = 0x0701_0400;
/// The melee byte (`0x52c51b0`) of a Quickplay or Elite battle: `0x0e` in
/// Eden's Quickplay battles, `0x10` in Elite battles on a Switch
/// (2026-11-02). Either is admitted in either mode.
pub const ONLINE_BATTLE_MELEE: [u8; 2] = [0x0e, 0x10];
/// The waiting practice before a Quickplay or local-wireless match (the
/// shared PracticeScene setup), in any mode. Only Quickplay's and Elite's
/// are `Scope::WaitingPractice`.
pub const WAITING_PRACTICE_MELEE: u8 = 0x0d;
/// A local-wireless (LDN) battle: the Training Modpack's 13.0.5 trace. Local
/// Smash on one console is melee `0x00` in the same mode (Eden p01).
pub const LOCAL_WIRELESS_BATTLE: (u8, u32) = (0x01, 0x0201_0000);

/// The audited online battle scope (a Quickplay or Elite battle) and the
/// waiting practice of those modes (since 0.2.3); never a local-wireless
/// battle or waiting practice (other consoles share it, and it has no online
/// isolation), other online modes, or an uninitialized game-mode singleton.
/// The mode's top bit is ignored: any `0x07xxxxxx` mode is online and never
/// offline.
pub const fn scope(initialized: bool, melee: u8, mode: u32) -> Scope {
    let mode = mode & 0x7fff_ffff;
    let battle = melee == ONLINE_BATTLE_MELEE[0] || melee == ONLINE_BATTLE_MELEE[1];
    let quickplay = mode == QUICKPLAY_MODE || mode == ELITE_MODE;
    if !initialized {
        Scope::Unsupported
    } else if battle && quickplay {
        Scope::Online1v1
    } else if melee == WAITING_PRACTICE_MELEE && quickplay {
        Scope::WaitingPractice
    } else if melee == ONLINE_BATTLE_MELEE[0]
        || melee == WAITING_PRACTICE_MELEE
        || (melee == LOCAL_WIRELESS_BATTLE.0 && mode == LOCAL_WIRELESS_BATTLE.1)
        || mode >> 24 == 7
    {
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
    use crate::session::{Action, Frame, Session};

    #[test]
    fn scope_excludes_waiting_and_other_online_modes() {
        assert_eq!(scope(true, 0x0e, 0x07010101), Scope::Online1v1);
        assert_eq!(scope(false, 0x0e, 0x07010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0e, 0x07010102), Scope::Unsupported);
        assert_eq!(scope(true, 0x0c, 0x07010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0c, 0x01010101), Scope::Offline);
    }

    #[test]
    fn elite_and_quickplay_battles_are_online_but_their_waiting_practice_is_not() {
        // A Switch in Elite Smash, 2026-11-02: waiting practice melee 0x0d,
        // then the battle at melee 0x10, both in mode 0x07010400.
        assert_eq!(scope(true, 0x10, 0x07010400), Scope::Online1v1);
        assert_eq!(scope(true, 0x10, 0x87010400), Scope::Online1v1);
        assert_eq!(scope(false, 0x10, 0x07010400), Scope::Unsupported);
        // Since 0.2.3 the waiting practice is its own scope, never Online1v1.
        assert_eq!(scope(true, 0x0d, 0x07010400), Scope::WaitingPractice);
        assert_eq!(scope(true, 0x00, 0x07010400), Scope::Unsupported);
        // Either battle melee in either mode is the same Quickplay battle.
        assert_eq!(scope(true, 0x0e, 0x07010400), Scope::Online1v1);
        assert_eq!(scope(true, 0x10, 0x07010101), Scope::Online1v1);
        // Other Quickplay/online mode words are not.
        assert_eq!(scope(true, 0x10, 0x07010100), Scope::Unsupported);
        assert_eq!(scope(true, 0x10, 0x07020000), Scope::Unsupported);
    }

    #[test]
    fn local_wireless_battles_and_waiting_practice_are_never_offline() {
        // Local wireless (LDN): waiting practice melee 0x0d, battle melee 0x01,
        // both in mode 0x02010000; local Smash on one console is melee 0x00.
        assert_eq!(scope(true, 0x01, 0x02010000), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x02010000), Scope::Unsupported);
        assert_eq!(scope(true, 0x00, 0x02010000), Scope::Offline);
        // Training (melee 0x20, mode 0x04040000) stays offline.
        assert_eq!(scope(true, 0x20, 0x04040000), Scope::Offline);
    }

    #[test]
    fn an_online_mode_with_its_top_bit_set_is_never_offline() {
        for mode in [0x87010101, 0x87010400, 0x87020000] {
            for melee in [0x00, 0x01, 0x0c, 0x0d, 0x0e, 0x10, 0x20] {
                assert_ne!(scope(true, melee, mode), Scope::Offline, "melee {melee:#x} mode {mode:#x}");
            }
        }
        assert_eq!(scope(true, 0x00, 0x82010000), Scope::Offline);
    }

    const SCOPES: [Scope; 4] = [Scope::Offline, Scope::WaitingPractice, Scope::Online1v1, Scope::Unsupported];

    #[test]
    fn the_waiting_practice_is_its_own_scope() {
        // Quickplay's waiting room, and Elite Smash's (the Switch, 2026-11-02).
        assert_eq!(scope(true, 0x0d, 0x07010101), Scope::WaitingPractice);
        assert_eq!(scope(true, 0x0d, 0x07010400), Scope::WaitingPractice);
        assert_eq!(scope(true, 0x0d, 0x87010101), Scope::WaitingPractice);
        assert_eq!(scope(true, 0x0d, 0x87010400), Scope::WaitingPractice);
        // Not before the game mode is initialized, not local wireless, not
        // other online or offline modes.
        assert_eq!(scope(false, 0x0d, 0x07010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x02010000), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x07010100), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x07010102), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x07020000), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x01010101), Scope::Unsupported);
        assert_eq!(scope(true, 0x0d, 0x04040000), Scope::Unsupported);
        // The battle a found match starts stays Online1v1.
        assert_eq!(scope(true, 0x0e, 0x07010101), Scope::Online1v1);
        assert_eq!(scope(true, 0x10, 0x07010400), Scope::Online1v1);
    }

    #[test]
    fn the_waiting_practice_drives_p1_like_training() {
        // One table: Training (melee 0x20, mode 0x04040000), local Smash and
        // the two waiting rooms take the same route for every slot.
        let training = scope(true, 0x20, 0x04040000);
        let local = scope(true, 0x00, 0x02010000);
        assert_eq!(training, Scope::Offline);
        assert_eq!(local, Scope::Offline);
        for waiting in [scope(true, 0x0d, 0x07010101), scope(true, 0x0d, 0x07010400)] {
            for slot in 0..=u8::MAX {
                assert_eq!(route(waiting, slot), route(training, slot), "slot {slot}");
                assert_eq!(route(waiting, slot), route(local, slot), "slot {slot}");
            }
            assert_eq!(route(waiting, 0), Route::LocalP1);
            for slot in 1..8 {
                assert_eq!(route(waiting, slot), Route::Skip);
            }
        }
    }

    #[test]
    fn online_routes_are_unchanged() {
        for slot in 0..=u8::MAX {
            assert_eq!(route(Scope::Online1v1, slot), Route::Online);
            assert_eq!(route(Scope::Unsupported, slot), Route::Off);
        }
        // Battles, local wireless and other online modes: as in 0.2.2.
        for (melee, mode, wanted) in [
            (0x0e, 0x07010101, Route::Online),
            (0x10, 0x07010400, Route::Online),
            (0x0e, 0x87010400, Route::Online),
            (0x01, 0x02010000, Route::Off),
            (0x0d, 0x02010000, Route::Off),
            (0x0e, 0x07010102, Route::Off),
            (0x0c, 0x07010101, Route::Off),
            (0x00, 0x07010400, Route::Off),
        ] {
            for slot in 0..8 {
                assert_eq!(route(scope(true, melee, mode), slot), wanted, "melee {melee:#x} mode {mode:#x}");
            }
        }
    }

    #[test]
    fn armed_objects_are_never_touched_in_another_scope() {
        for armed_in in SCOPES {
            for now in SCOPES {
                assert_eq!(same_scene(armed_in, now), armed_in == now, "{armed_in:?} -> {now:?}");
            }
        }
    }

    /// The runtime's frame (`pilot_frame`): identity is the same objects in
    /// the same scope, and a release restores the rank only on those.
    fn frame(armed_in: Scope, now: Scope, same_objects: bool, think: bool) -> (Frame, bool) {
        let same = same_objects && same_scene(armed_in, now);
        let f = Frame {
            desired: Some(3),
            in_battle: true,
            same_identity: same,
            can_think: think,
        };
        (f, same)
    }

    #[test]
    fn the_waiting_practice_hands_over_to_the_battle() {
        let mut session = Session::new();
        let practice = scope(true, 0x0d, 0x07010101);
        let battle = scope(true, 0x0e, 0x07010101);
        let (f, _) = frame(practice, practice, true, true);
        assert_eq!(session.step(f), Action::Arm(3));
        let (f, _) = frame(practice, practice, true, true);
        assert_eq!(session.step(f), Action::Think);
        // The first battle frame, even if the battle's objects sit at the
        // practice's addresses: released, and the old objects not restored.
        for same_objects in [false, true] {
            let mut s = session;
            let (f, restores) = frame(practice, battle, same_objects, false);
            assert_eq!(s.step(f), Action::Release);
            assert!(!restores);
            assert_eq!(s.armed(), None);
            // The battle then arms afresh once its own path is ready.
            let (f, _) = frame(battle, battle, true, false);
            assert_eq!(s.step(f), Action::None);
            let (f, _) = frame(battle, battle, true, true);
            assert_eq!(s.step(f), Action::Arm(3));
        }
        // Back in the waiting practice after the battle: the same again.
        let mut s = Session::new();
        let (f, _) = frame(battle, battle, true, true);
        assert_eq!(s.step(f), Action::Arm(3));
        let (f, restores) = frame(battle, practice, true, true);
        assert_eq!(s.step(f), Action::Release);
        assert!(!restores);
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
