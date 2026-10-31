//! Switch-only raw game calls for the AI-as-pad (13.0.5). Ported from the
//! Training Modpack fork's `cpu_pilot.rs` (`native`, `ready`, `set_rank`,
//! `think`, `finish`), unchanged in what they read, write and call.
//!
//! The caller must already have checked the game version and the 64-byte
//! guards of every function called here, and must call these only on the
//! game thread during a battle. Every AI call runs on a private copy of the
//! AI random stream (stream 4, `ai`), and the shared stream is put back
//! before returning, so the AI's draws never reach the checked stream.
use crate::pad::{self, MappedInputs};
use core::ptr;

pub mod offsets {
    pub const AI_THINK: usize = 0x2da180;
    pub const AI_SET_RANK: usize = 0x6b8400;
    pub const AI_RESET: usize = 0x2d8d40;
    pub const RANK_TABLE: usize = 0x4470dc0;
    pub const AI_SERVICE: usize = 0x52b6fd8;
    pub const RNG_ROOT: usize = 0x52c35b0;
    pub const CONTROLLER_VTABLE: usize = 0x4f85d30;
    /// Byte offset of stream 4 (`ai`) among the 0x14-byte random streams.
    pub const AI_STREAM: usize = 0x50;
    /// FighterEntry fields.
    pub const ENTRY_CONTROLLER: usize = 0x4178;
    pub const ENTRY_INFO: usize = 0xf8;
    pub const MODE_INITIALIZED: usize = 0x53144d8;
    pub const MELEE_MODE: usize = 0x52c51b0;
    pub const GAME_MODE: usize = 0x53050f0;
}

/// The AI stream is 20 bytes.
pub type Stream = [u8; 20];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Native {
    pub ctrl: usize,
    pub ai: usize,
    pub service: usize,
    pub random: usize,
    pub slot: u8,
}

#[inline]
unsafe fn read<T: Copy>(address: usize) -> T {
    ptr::read_unaligned(address as *const T)
}

#[inline]
unsafe fn write<T: Copy>(address: usize, value: T) {
    ptr::write_unaligned(address as *mut T, value)
}

#[inline]
unsafe fn ptr_at(address: usize) -> Option<usize> {
    let value: usize = read(address);
    (value != 0 && value % 8 == 0).then_some(value)
}

/// The controller and fighter-info pointers of a `FighterEntry`.
pub unsafe fn controller(base: usize, entry: usize) -> Result<(usize, usize), &'static str> {
    let ctrl = ptr_at(entry + offsets::ENTRY_CONTROLLER).ok_or("controller_pointer")?;
    if read::<usize>(ctrl) != base + offsets::CONTROLLER_VTABLE {
        return Err("controller_vtable");
    }
    let info = ptr_at(entry + offsets::ENTRY_INFO).ok_or("fighter_info_pointer")?;
    Ok((ctrl, info))
}

/// The address of the shared AI random stream.
pub unsafe fn ai_stream(base: usize) -> Option<usize> {
    ptr_at(ptr_at(ptr_at(base + offsets::RNG_ROOT)?)?).map(|state| state + offsets::AI_STREAM)
}

/// The AI service singleton.
pub unsafe fn service(base: usize) -> Option<usize> {
    ptr_at(ptr_at(base + offsets::AI_SERVICE)?)
}

pub unsafe fn scope(base: usize) -> crate::online::Scope {
    crate::online::scope(
        read::<u8>(base + offsets::MODE_INITIALIZED) & 1 != 0,
        read(base + offsets::MELEE_MODE),
        read(base + offsets::GAME_MODE),
    )
}

/// Human-only online isolation must not hide native subfighter AI draws.
/// set_rank also follows ai+0x980; reject that second AI before arming.
pub unsafe fn single_human(n: Native) -> bool {
    read::<usize>(n.ctrl + 0x60) == 0 && read::<usize>(n.ai + 0x980) == 0
}

/// Native manager inputs/identity used for the scene-wide online lease.
pub unsafe fn stage_state(service: usize) -> (i32, u8, i32) {
    (read(service + 0x1fc), read(service + 0xd0), read(service + 0x150))
}

/// Saved shared inputs for one synchronous process_begin call. Only its
/// matching post-body hook may finish it, before the native epilogue runs.
pub struct StageCall {
    service: usize,
    random: usize,
    shared: Stream,
    d0: u8,
}

pub unsafe fn stage_begin(service: usize, random: usize, private: Stream) -> StageCall {
    let call = StageCall {
        service,
        random,
        shared: read(random),
        d0: read(service + 0xd0),
    };
    write(random, private);
    write(service + 0xd0, call.d0 | 1);
    call
}

/// Restore each storage independently, and never dereference a dead singleton.
pub unsafe fn stage_end(base: usize, call: StageCall) -> Option<(Stream, i32)> {
    let same_random = ai_stream(base) == Some(call.random);
    let same_service = service(base) == Some(call.service);
    let private = if same_random {
        let private = read(call.random);
        write(call.random, call.shared);
        Some(private)
    } else {
        None
    };
    if same_service {
        write(call.service + 0xd0, call.d0);
        private.map(|private| (private, read(call.service + 0x1fc)))
    } else {
        None
    }
}

pub unsafe fn stream_copy(random: usize) -> Stream {
    read(random)
}

/// Qualifies P1's own (human) AI object: back-links, slot, loaded scripts,
/// and no operation-CPU flag anywhere (a real CPU is never piloted).
pub unsafe fn resolve(base: usize, ctrl: usize, info: usize, slot: u8) -> Result<Native, &'static str> {
    let ai = ptr_at(ctrl + 0x58).ok_or("ai_pointer")?;
    let context = ptr_at(ptr_at(ai).ok_or("ai_context_root")?).ok_or("ai_context")?;
    let service = service(base).ok_or("service")?;
    if read::<usize>(ai + 8) != ctrl {
        return Err("ai_controller_backlink");
    }
    if read::<usize>(context + 0x30) != ai {
        return Err("context_ai_backlink");
    }
    let s = i32::from(slot);
    if read::<i32>(ctrl + 8) != s || read::<i32>(ctrl + 0x6c) != s || read::<i32>(ai + 0x160) != s {
        return Err("native_slot_mismatch");
    }
    if read::<u8>(ctrl + 0x69) != 1 {
        return Err("controller_not_loaded");
    }
    if read::<u8>(ctrl + 0x70) != 0 {
        return Err("controller_operation_cpu");
    }
    if read::<u8>(info + 0x92) != 0 {
        return Err("fighter_info_operation_cpu");
    }
    if read::<u8>(service + 0x80 + usize::from(slot & 7)) != 0 {
        return Err("service_operation_cpu");
    }
    if read::<u8>(context + 0x38) == 0 {
        return Err("context_not_loaded");
    }
    if read::<usize>(context + 0x10) == 0 || read::<usize>(context + 0x18) == 0 {
        return Err("context_scripts_missing");
    }
    let random = ai_stream(base).ok_or("rng_state")?;
    Ok(Native {
        ctrl,
        ai,
        service,
        random,
        slot,
    })
}

/// The native AI may think this frame.
pub unsafe fn ready(n: Native) -> Result<(), &'static str> {
    let record = ptr_at(n.ai + 0x168).ok_or("ai_record_pointer")?;
    if read::<i32>(n.service + 0x1fc) != 2 {
        return Err("service_not_battle");
    }
    if read::<u8>(n.ctrl + 0x69) == 0 {
        return Err("ready_controller_not_loaded");
    }
    if read::<u32>(n.ai + 0xc44) & 0x8000 == 0 {
        return Err("ai_active_flag_missing");
    }
    if read::<u8>(n.ai + 0xc08) == 0x24 {
        return Err("ai_control_mode");
    }
    let flags: u32 = read(record + 0x54);
    if flags & 0x8000_0000 == 0 {
        return Err("record_active_flag_missing");
    }
    if flags & 0x800 != 0 {
        return Err("record_pause_flag");
    }
    if read::<u8>(record + 0x55) & 8 != 0 {
        return Err("record_inactive_flag");
    }
    if read::<u8>(record + 0x260) != 0 {
        return Err("record_blocked_state");
    }
    Ok(())
}

pub unsafe fn rank_table(base: usize) -> [i32; 8] {
    read(base + offsets::RANK_TABLE)
}

pub unsafe fn rank(n: Native) -> i32 {
    read(n.ai + 0xc0c)
}

unsafe fn set_rank(base: usize, n: Native, rank: i32, initialize: bool) {
    let call: unsafe extern "C" fn(usize, i32) = core::mem::transmute(base + offsets::AI_SET_RANK);
    call(n.ctrl, rank);
    if initialize {
        // set_rank returns early when the rank is unchanged; initialize the
        // slot's scripts explicitly, as the native 0-to-1 operation switch does.
        let flag = n.service + 0x80 + usize::from(n.slot & 7);
        let previous: u8 = read(flag);
        let reset: unsafe extern "C" fn(usize, u32) = core::mem::transmute(base + offsets::AI_RESET);
        write(flag, 1u8);
        reset(n.ai, 1);
        write(flag, previous);
    }
}

/// Arms P1's AI at `rank`. Returns the old rank (for `release`) and the
/// private stream the AI continues from.
pub unsafe fn arm(base: usize, n: Native, rank_value: u8) -> (i32, Stream) {
    let shared: Stream = read(n.random);
    let old = rank(n);
    set_rank(base, n, i32::from(rank_value), true);
    let private: Stream = read(n.random);
    write(n.random, shared);
    (old, private)
}

/// One AI decision on the private stream, as P1's mapped input. The
/// controller's input fields (`+0xc..+0x58`) and the shared stream are
/// restored before returning.
pub unsafe fn think(base: usize, n: Native, private: &mut Stream) -> MappedInputs {
    let controller: [u8; 0x4c] = read(n.ctrl + 0xc);
    let shared: Stream = read(n.random);
    write(n.random, *private);
    let call: unsafe extern "C" fn(usize) = core::mem::transmute(base + offsets::AI_THINK);
    call(n.ai);
    let decision = pad::from_ai(read(n.ctrl + 0xc), read(n.ctrl + 0x10), read(n.ctrl + 0x14));
    write(n.ctrl + 0xc, controller);
    *private = read(n.random);
    write(n.random, shared);
    decision
}

/// Restores the rank saved by `arm`, keeping the shared stream unchanged.
pub unsafe fn release(base: usize, n: Native, old_rank: i32) {
    let shared: Stream = read(n.random);
    set_rank(base, n, old_rank, false);
    write(n.random, shared);
}
