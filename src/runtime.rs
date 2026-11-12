//! Switch only: version check, site guards, the two inline hooks on final
//! input mapping (`0x1750dd0`), the P1 pilot, the HUD mark, the Deluxe rows,
//! the exports and the install line.
//!
//! Offline and in Quickplay's waiting practice: P1 in battles with a native
//! CPU. Online: the sole locally mapped human slot in the audited 1v1 scope.
//! Analysis uses an isolated AI RNG lease; decisions replace mapped input
//! before the normal pack/send path.
use crate::chain::{self, hook::Outcome, Entry};
use crate::model::{self, buttons, Kind, Line, Site, TagSlot, TagWrite};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use ssbu_cpu_pilot_core::combo::{self, Combo};
use ssbu_cpu_pilot_core::deluxe::{self, DeluxeOverlayStateV1};
use ssbu_cpu_pilot_core::hud;
use ssbu_cpu_pilot_core::native::{self, Native, Stream};
use ssbu_cpu_pilot_core::online::{self as scopes, Route, Scope};
use ssbu_cpu_pilot_core::pad::{self, MappedInputs};
use ssbu_cpu_pilot_core::session::{Action, Frame, Session};
use ssbu_cpu_pilot_core::settings::{self, Settings, SharedSettings};
use ssbu_cpu_pilot_core::level;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};

#[path = "online_runtime.rs"]
mod online_runtime;
#[path = "online_hooks.rs"]
mod online_hooks;

const OPEN_READ: i32 = 1;
const OPEN_WRITE_APPEND: i32 = 2 | 4;

/// Counts calls into the heap. Nothing on the game thread is meant to make one.
struct CountingAllocator;
static HEAP_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Shared cell whose accesses are ordered by the surrounding atomics.
struct Owned<T>(UnsafeCell<T>);
// SAFETY: see each use.
unsafe impl<T> Sync for Owned<T> {}

static TEXT_BASE: AtomicUsize = AtomicUsize::new(0);
static SETTINGS: SharedSettings = SharedSettings::new(Settings {
    pilot: false,
    level: settings::DEFAULT_LEVEL,
    auto_rematch: false,
    auto_save_replay: false,
});
/// P1 or the level changed (the pad's L+R+X, or the P1 row); the worker saves
/// `config.ini`.
static DIRTY: AtomicBool = AtomicBool::new(false);
/// Set (1 + controller present) at `fim_valid`, consumed at `fim_exit`.
static FIM_CALL: AtomicU8 = AtomicU8::new(0);
/// 0 while P1 is the player, else the CPU level driving P1.
static ARMED_LEVEL: AtomicU8 = AtomicU8::new(0);
/// The last input the CPU gave P1 (`pad::pack`).
static LAST_INPUT: AtomicU64 = AtomicU64::new(0);
/// Hooks installed and every called site usable.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Game functions resolved by exported symbol at boot.
static MANAGER_SLOT: AtomicUsize = AtomicUsize::new(0);
static GET_ENTRY: AtomicUsize = AtomicUsize::new(0);
static IS_READY_GO: AtomicUsize = AtomicUsize::new(0);
static IS_RESULT_MODE: AtomicUsize = AtomicUsize::new(0);

const SYM_MANAGER: &[u8] = b"_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\0";
const SYM_GET_ENTRY: &[u8] =
    b"_ZN3app8lua_bind38FighterManager__get_fighter_entry_implEPNS_14FighterManagerENS_14FighterEntryIDE\0";
const SYM_READY_GO: &[u8] = b"_ZN3app8lua_bind32FighterManager__is_ready_go_implEPNS_14FighterManagerE\0";
const SYM_RESULT_MODE: &[u8] =
    b"_ZN3app8lua_bind35FighterManager__is_result_mode_implEPNS_14FighterManagerE\0";

/// The pilot's state. Only `fim_exit` touches it, on the game thread, for
/// the selected local player slot.
struct Pilot {
    session: Session,
    combo: Combo,
    armed: Option<Native>,
    /// The scope `armed` was armed in (`scopes::same_scene`).
    armed_scope: Scope,
    old_rank: i32,
    stream: Stream,
    /// The 13.0.5 level ladder, checked at boot.
    ranks: [u8; 9],
}

static PILOT: Owned<Pilot> = Owned(UnsafeCell::new(Pilot {
    session: Session::new(),
    combo: Combo::new(),
    armed: None,
    armed_scope: Scope::Unsupported,
    old_rank: 0,
    stream: [0; 20],
    ranks: [0; 9],
}));

fn tick() -> u64 {
    unsafe { nn::os::GetSystemTick() as u64 }
}

fn base() -> usize {
    TEXT_BASE.load(Ordering::Relaxed)
}

unsafe fn read_u32(address: usize) -> u32 {
    (address as *const u32).read_unaligned()
}

unsafe fn read_usize(address: usize) -> usize {
    (address as *const usize).read_unaligned()
}

fn lookup(symbol: &[u8]) -> usize {
    let mut address = 0usize;
    unsafe { nn::ro::LookupSymbol(&mut address, symbol.as_ptr()) };
    address
}

/// The live FighterManager, if a battle scene has one.
unsafe fn manager() -> Option<usize> {
    let slot = MANAGER_SLOT.load(Ordering::Relaxed);
    let manager = read_usize(slot);
    (manager != 0 && manager % 8 == 0).then_some(manager)
}

unsafe fn call_bool(function: &AtomicUsize, manager: usize) -> bool {
    let f: extern "C" fn(usize) -> bool = core::mem::transmute(function.load(Ordering::Relaxed));
    f(manager)
}

unsafe fn fighter_entry(manager: usize, index: i32) -> usize {
    let f: extern "C" fn(usize, i32) -> usize = core::mem::transmute(GET_ENTRY.load(Ordering::Relaxed));
    f(manager, index)
}

/// What the pilot needs to know about P1 this frame.
struct Seen {
    native: Option<Native>,
    in_battle: bool,
    can_think: bool,
    gate: &'static str,
    scope: Scope,
}

/// P1 as in Training (`Route::LocalP1`): FighterEntry 0 on controller slot 0.
unsafe fn observe(scope: Scope) -> Seen {
    let none = |gate| Seen {
        native: None,
        in_battle: false,
        can_think: false,
        gate,
        scope,
    };
    let Some(manager) = manager() else {
        return none("no_manager");
    };
    if call_bool(&IS_RESULT_MODE, manager) {
        return none("result_mode");
    }
    let entry = fighter_entry(manager, 0);
    if entry == 0 || entry % 8 != 0 {
        return none("no_p1_entry");
    }
    let (ctrl, info) = match native::controller(base(), entry) {
        Ok(found) => found,
        Err(gate) => return none(gate),
    };
    let n = match native::resolve(base(), ctrl, info, 0) {
        Ok(n) => n,
        Err(gate) => return none(gate),
    };
    let ready_go = call_bool(&IS_READY_GO, manager);
    let ready = native::ready(n);
    Seen {
        native: Some(n),
        in_battle: true,
        can_think: ready_go && ready.is_ok(),
        gate: if !ready_go { "not_ready_go" } else { ready.err().unwrap_or("ready") },
        scope,
    }
}

/// One P1 frame: the L+R+X combo, then the session's action.
unsafe fn pilot_frame(out: *mut MappedInputs, raw_buttons: u32, slot: u8, scope: Scope) {
    let p = &mut *PILOT.0.get();
    // L+R+X steps the level whenever P1 is set to CPU (0.2.1; before, only
    // once the CPU was driving), except while Online Deluxe's window shows a
    // page with rows (0.2.4): the press is the window's then, and on the P1
    // row Deluxe asks for the step itself (`menu::Handler::request`). Deluxe
    // is asked only while the three buttons are held.
    let cpu = SETTINGS.load().pilot;
    let held = buttons::combo_held(raw_buttons);
    let rows = cpu && held && deluxe_rows_page();
    if p.combo.step(held, deluxe::pad_steps(cpu, rows)) {
        let now = SETTINGS.update(|s| Settings {
            level: combo::next_level(s.level),
            ..s
        });
        DIRTY.store(true, Ordering::Release);
        diag_event(format_args!("combo level={}", now.level));
    }
    let wanted = SETTINGS.load();
    let seen = online_runtime::observe(slot, scope);
    if STALE.swap(false, Ordering::AcqRel) {
        // The battle the session was armed in ended (see `watch_battle_end`):
        // release on the objects only if they are still the armed ones, and
        // start this battle's session afresh.
        release(p, &seen);
        p.session = Session::new();
    }
    // The armed objects only in the scope they were armed in: a found match
    // tears the waiting practice down, and its battle arms afresh.
    let same = match (p.armed, seen.native) {
        (Some(a), Some(n)) => a.ctrl == n.ctrl && a.ai == n.ai && scopes::same_scene(p.armed_scope, seen.scope),
        (Some(_), None) => false,
        (None, _) => true,
    };
    let action = p.session.step(Frame {
        desired: wanted.pilot.then_some(wanted.level),
        in_battle: seen.in_battle,
        same_identity: same,
        can_think: seen.can_think,
    });
    diag_gate(seen.gate);
    let think = match action {
        Action::None => false,
        Action::Release => {
            release(p, &seen);
            false
        }
        Action::Arm(level) => arm(p, &seen, level),
        Action::Rearm(level) => {
            release(p, &seen);
            arm(p, &seen, level)
        }
        Action::Think => true,
    };
    if !think {
        return;
    }
    let (Some(n), Some(level)) = (p.armed, p.session.armed()) else {
        return;
    };
    let input = native::think(base(), n, &mut p.stream);
    *out = input;
    LAST_INPUT.store(pad::pack(input), Ordering::Release);
    ARMED_LEVEL.store(level, Ordering::Release);
}

unsafe fn arm(p: &mut Pilot, seen: &Seen, level: u8) -> bool {
    let Some(n) = seen.native else {
        return false;
    };
    let rank = p.ranks[usize::from(settings::clamp_level(level) - 1)];
    let (old, stream) = native::arm(base(), n, rank);
    p.armed = Some(n);
    p.armed_scope = seen.scope;
    p.old_rank = old;
    p.stream = stream;
    ARMED_SLOT.store(n.slot, Ordering::Release);
    ARMED_LEVEL.store(level, Ordering::Release);
    diag_event(format_args!("arm level={level} rank={rank} old_rank={old} slot={}", n.slot));
    true
}

unsafe fn release(p: &mut Pilot, seen: &Seen) {
    ARMED_LEVEL.store(0, Ordering::Release);
    let Some(armed) = p.armed.take() else {
        return;
    };
    // Restore only on the same live objects in the scope they were armed in;
    // after a scene change they are gone (the waiting practice when a match
    // is found).
    let scene = scopes::same_scene(p.armed_scope, seen.scope);
    if !scene {
        diag_event(format_args!("handover from={:?} to={:?}", p.armed_scope, seen.scope));
    }
    let restored = scene && seen.native.is_some_and(|n| n == armed);
    if restored {
        native::release(base(), armed, p.old_rank);
    }
    diag_event(format_args!("release restored={restored} old_rank={}", p.old_rank));
}

/// `0x1750df0`: the index is not -1, so `x19`/`x23` will hold out/index at
/// the exit; `x3` (the controller struct) decides whether `x20` is loaded.
unsafe extern "C" fn fim_valid(ctx: &mut InlineCtx) {
    let controller = ctx.registers[3].x() != 0;
    FIM_CALL.store(1 + u8::from(controller), Ordering::Release);
}

/// `0x1751ce8`: `*out` is complete, still before input packing/transmission.
unsafe extern "C" fn fim_exit(ctx: &mut InlineCtx) {
    let marker = FIM_CALL.swap(0, Ordering::AcqRel);
    if marker == 0 || !ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let out = ctx.registers[19].x() as usize;
    let slot = ctx.registers[23].x();
    if slot >= 8 || out == 0 || out % 4 != 0 {
        return;
    }
    let scope = native::scope(base());
    if scopes::route(scope, slot as u8) == Route::Skip {
        return;
    }
    let controller = if marker == 2 { ctx.registers[20].x() as usize } else { 0 };
    let raw = if controller != 0 && controller % 8 == 0 {
        read_u32(controller + 8)
    } else {
        0
    };
    diag_count();
    LAST_PILOT_TICK.store(tick(), Ordering::Release);
    pilot_frame(out as *mut MappedInputs, raw, slot as u8, scope);
}

/// Final input mapping runs only in battle, so the pilot never sees the
/// battle end. The worker clears the armed state once these calls have
/// stopped for a second (results, menus, scene change); the next battle
/// frame then releases the session on whatever objects are live.
static LAST_PILOT_TICK: AtomicU64 = AtomicU64::new(0);
static STALE: AtomicBool = AtomicBool::new(false);
/// One second of `nn::os` ticks (19.2 MHz).
const STALE_TICKS: u64 = 19_200_000;
/// A quarter second: P1's battle input mapping ran this recently.
const BATTLE_TICKS: u64 = STALE_TICKS / 4;

/// P1's final input mapping ran within the last quarter second: a battle is
/// running and `pilot_frame` reads L+R+X from P1's pad itself.
fn battle_frames_running() -> bool {
    tick().saturating_sub(LAST_PILOT_TICK.load(Ordering::Acquire)) < BATTLE_TICKS
}

/// Online Deluxe's `ssbu_online_deluxe_overlay_state_v1` (0.10.5+), 0 until
/// found. Only `find_deluxe_overlay` looks it up, at boot and then on the
/// worker; hooks only call what it stored.
static DELUXE_OVERLAY: AtomicUsize = AtomicUsize::new(0);
/// After a miss, looked up again at most once a second.
const DELUXE_RETRY_TICKS: u64 = STALE_TICKS;
static DELUXE_RETRY: Owned<deluxe::Retry> = Owned(UnsafeCell::new(deluxe::Retry::new()));

/// Finds Deluxe's export if it is not found yet and a lookup is due.
///
/// # Safety
/// Only `main` (before the worker starts) and then the worker call this.
unsafe fn find_deluxe_overlay() -> bool {
    if DELUXE_OVERLAY.load(Ordering::Acquire) != 0 {
        return true;
    }
    if !(*DELUXE_RETRY.0.get()).due(tick(), DELUXE_RETRY_TICKS) {
        return false;
    }
    let address = lookup(deluxe::SYMBOL);
    if address == 0 {
        return false;
    }
    DELUXE_OVERLAY.store(address, Ordering::Release);
    diag_event(format_args!("deluxe_overlay found"));
    true
}

/// Deluxe's window shows a page with rows, per its export. Without the export
/// (an older Deluxe, or none) there is none. Lock-free on Deluxe's side.
fn deluxe_rows_page() -> bool {
    let address = DELUXE_OVERLAY.load(Ordering::Acquire);
    if address == 0 {
        return false;
    }
    // SAFETY: a found address is that export, whose ABI this declares
    // (`deluxe::SYMBOL`); `state` is a writable `DeluxeOverlayStateV1`.
    let filled = unsafe {
        let f: unsafe extern "C" fn(*mut DeluxeOverlayStateV1) -> bool = core::mem::transmute(address);
        let mut state = DeluxeOverlayStateV1::default();
        f(&mut state).then_some(state)
    };
    deluxe::rows_page(filled)
}

fn watch_battle_end() {
    if ARMED_LEVEL.load(Ordering::Acquire) == 0 {
        return;
    }
    let last = LAST_PILOT_TICK.load(Ordering::Acquire);
    if tick().saturating_sub(last) > STALE_TICKS {
        ARMED_LEVEL.store(0, Ordering::Release);
        STALE.store(true, Ordering::Release);
        diag_event(format_args!("stale no_battle_frames armed_cleared=1"));
    }
}

/// Completed UI port array (0x3762ec0 single exit). x22 advanced by
/// ten 0x30-byte slots after its initial +0x10.
unsafe extern "C" fn results_frame(ctx: &mut InlineCtx) {
    let end = ctx.registers[22].x() as usize;
    if end >= 0x1f0 {
        crate::results::frame((end - 0x1f0) as *mut _, SETTINGS.load());
    }
}

// ---------------------------------------------------------------------------
// The HUD mark: while the CPU drives P1, P1's tag text reads "CPU Lv<n>".
// UI only: nothing of the battle is touched. The text is written from the UI
// manager's per-frame pass (`model::HUD_PASS`, the UI job's thread), which
// reads `ARMED_LEVEL` (the only thing the game thread shares with it) and
// writes only when the wanted text differs from what it last handled.

/// The slot the pilot is armed on (online the local player may be slot 1).
static ARMED_SLOT: AtomicU8 = AtomicU8::new(0);
/// The `model::hud_wanted` byte the UI thread has handled (0 = the game's text).
static HUD_SETTLED: AtomicU8 = AtomicU8::new(0);
/// The slot whose tag carries the label, for the restore.
static HUD_SHOWN_SLOT: AtomicU8 = AtomicU8::new(0);
/// The slow path is never entered twice at once.
static HUD_BUSY: AtomicBool = AtomicBool::new(false);
/// P1's tag text in each HUD variant (`model::HUD_PARTS`); only the slow
/// path, under `HUD_BUSY`, touches it.
static HUD_TAGS: Owned<[TagSlot; 2]> = Owned(UnsafeCell::new([TagSlot::new(), TagSlot::new()]));

unsafe fn read_u16(address: usize) -> u16 {
    (address as *const u16).read_unaligned()
}

/// `nn::ui2d::Pane::FindPaneByName` over `from`'s subtree; `name` is NUL-terminated.
unsafe fn find_pane(from: usize, name: &[u8]) -> Option<usize> {
    let f: extern "C" fn(usize, *const u8) -> usize =
        core::mem::transmute(base() + model::FIND_PANE.offset);
    let pane = f(from, name.as_ptr());
    (pane != 0 && pane % 8 == 0).then_some(pane)
}

/// The tag TextBox under one of P1's Parts panes.
unsafe fn tag_pane(root: usize, parts: &[u8]) -> Option<usize> {
    find_pane(find_pane(root, parts)?, model::HUD_TAG_PANE)
}

/// The TextBox's text, copied out; `None` unless it is a plain UTF-16 text.
unsafe fn read_tag(pane: usize, out: &mut [u16; model::TAG_CAP + 1]) -> Option<usize> {
    let buf = read_usize(pane + model::ui2d::TEXT_BUF);
    let units = model::tag_text_units(
        ((pane + model::ui2d::TEXT_IS_UTF8) as *const u8).read(),
        buf,
        read_u16(pane + model::ui2d::TEXT_LEN),
        read_u16(pane + model::ui2d::TEXT_CAP),
    )?;
    if units != 0 {
        core::ptr::copy_nonoverlapping(buf as *const u16, out.as_mut_ptr(), units);
    }
    Some(units)
}

/// The game's UTF-16 text setter; it grows the TextBox's buffer when needed.
unsafe fn set_text(pane: usize, text: &[u16]) {
    let f: extern "C" fn(usize, *const u16, i32, u32) =
        core::mem::transmute(base() + model::SET_TEXT.offset);
    f(pane, text.as_ptr(), text.len() as i32, 0);
}

/// Puts `wanted` (a level, 0 = the game's text) on the tag panes of player
/// `player` under the HUD layout's root pane; returns how many were written.
unsafe fn hud_write(root: usize, wanted: u8, player: u8) -> usize {
    // SAFETY: `HUD_BUSY` is held by the caller.
    let tags = &mut *HUD_TAGS.0.get();
    let Some(parts_of_player) = model::hud_parts(player) else {
        return 0;
    };
    let mut label = [0u16; model::LABEL_CAP];
    let label_len = if wanted != 0 { hud::label_utf16(wanted, &mut label) } else { 0 };
    let mut written = 0;
    for (slot, parts) in tags.iter_mut().zip(parts_of_player) {
        let Some(pane) = tag_pane(root, parts) else {
            slot.forget();
            continue;
        };
        let mut text = [0u16; model::TAG_CAP + 1];
        let Some(units) = read_tag(pane, &mut text) else {
            slot.forget();
            continue;
        };
        match slot.step(wanted, pane, &text[..units], &label[..label_len]) {
            TagWrite::Keep => {}
            TagWrite::Label => {
                set_text(pane, &label[..label_len]);
                written += 1;
            }
            TagWrite::Original(original) => {
                set_text(pane, original);
                written += 1;
            }
        }
    }
    written
}

/// The slow path: `holder`'s layout is the match HUD, so settle on `wanted`.
unsafe fn hud_handle(holder: usize, wanted: u8) {
    if holder == 0 || holder % 8 != 0 {
        return;
    }
    let layout = read_usize(holder + model::ui2d::HOLDER_LAYOUT);
    if layout == 0 || layout % 8 != 0 {
        return;
    }
    let name = read_usize(layout + model::ui2d::LAYOUT_NAME);
    if name == 0 || !model::name_is(|i| ((name + i) as *const u8).read(), model::HUD_LAYOUT) {
        return;
    }
    if HUD_BUSY.swap(true, Ordering::Acquire) {
        return;
    }
    let root = read_usize(layout + model::ui2d::LAYOUT_ROOT);
    let (level, slot) = model::hud_split(wanted);
    let shown = HUD_SHOWN_SLOT.load(Ordering::Relaxed);
    let mut written = 0;
    if root != 0 && root % 8 == 0 {
        // A label on another slot's tag goes back first.
        if level == 0 || slot != shown {
            written += hud_write(root, 0, shown);
        }
        if level != 0 {
            written += hud_write(root, level, slot);
            HUD_SHOWN_SLOT.store(slot, Ordering::Relaxed);
        }
    }
    // Settled whether or not a pane was there: a HUD without the tag is not
    // searched again every frame, and the next change is handled afresh.
    HUD_SETTLED.store(wanted, Ordering::Release);
    HUD_BUSY.store(false, Ordering::Release);
    if level != 0 {
        diag_event(format_args!("hud set level={level} slot={slot} panes={written}"));
    } else {
        diag_event(format_args!("hud restore panes={written}"));
    }
}

/// `model::HUD_PASS`: runs for every UI holder every frame, with `x23` = the
/// holder. Costs two atomic loads unless the wanted level changed.
unsafe extern "C" fn hud_tick(ctx: &mut InlineCtx) {
    let wanted = model::hud_wanted(
        ARMED_LEVEL.load(Ordering::Acquire),
        ARMED_SLOT.load(Ordering::Acquire),
    );
    if wanted == HUD_SETTLED.load(Ordering::Relaxed) {
        return;
    }
    hud_handle(ctx.registers[23].x() as usize, wanted);
}

// ---------------------------------------------------------------------------
// Exports: other plugins (the Training Modpack, smash-gamepad-overlay,
// automation) read the state and may change the settings.

/// `ssbu_cpu_pilot_state_v1` fills this.
#[repr(C)]
pub struct PilotStateV1 {
    pub abi_version: u32,
    /// `Settings::pack`.
    pub settings: u32,
    /// 0 while P1 is the player, else the CPU level driving P1.
    pub armed_level: u8,
    pub _reserved: [u8; 7],
    /// The last input the CPU gave P1: the 8-byte `MappedInputs` image.
    pub last_input: u64,
}

/// # Safety
/// `out` must be null or point to a writable `PilotStateV1`.
#[no_mangle]
pub unsafe extern "C" fn ssbu_cpu_pilot_state_v1(out: *mut PilotStateV1) -> bool {
    if out.is_null() || !ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    out.write(PilotStateV1 {
        abi_version: 1,
        settings: SETTINGS.load().pack(),
        armed_level: ARMED_LEVEL.load(Ordering::Acquire),
        _reserved: [0; 7],
        last_input: LAST_INPUT.load(Ordering::Acquire),
    });
    true
}

/// Replaces the settings (`Settings::pack` bits); for automation.
#[no_mangle]
pub extern "C" fn ssbu_cpu_pilot_set_v1(bits: u32) -> bool {
    if !ACTIVE.load(Ordering::Acquire) {
        return false;
    }
    SETTINGS.store(Settings::unpack(bits));
    DIRTY.store(true, Ordering::Release);
    true
}

// ---------------------------------------------------------------------------
// Online Deluxe rows (its shared Plugins page).

mod menu {
    use super::*;
    use deluxe_plugin_sdk::{register, OptionSpec, PluginHandler, PluginSpec, RequestStatus, RowLayout};

    const fn row(
        id: &'static str,
        label: &'static str,
        choices: &'static [&'static str],
        values: &'static [&'static str],
        no_persist: bool,
    ) -> OptionSpec {
        OptionSpec {
            id,
            label,
            choices,
            config_key: None,
            values: Some(values),
            no_persist,
            tones: None,
            read_only: false,
            layout: RowLayout::Value,
        }
    }

    static OPTIONS: [OptionSpec; 3] = [
        // `Player`, `CPU (L1)` … `CPU (L9)`. Deluxe would persist the choice it
        // asked for, but a ZR press asks for a neighbouring level and gets the
        // Player/CPU toggle (`settings::p1_request`), so the pilot writes `p1`
        // and `level` itself.
        row(settings::KEY_PILOT, "P1", &settings::P1_CHOICES, &settings::P1_VALUES, true),
        row(settings::KEY_REMATCH, "Auto rematch", &["Off", "On"], &["off", "on"], false),
        row(settings::KEY_SAVE, "Auto save replay", &["Off", "On"], &["off", "on"], false),
    ];

    static SPEC: PluginSpec = PluginSpec {
        id: "ssbu_cpu_pilot",
        label: "CPU Pilot",
        options: &OPTIONS,
        config_path: Some(model::CONFIG_INI),
        // Its own page in Deluxe's window cycle, titled "CPU Pilot": Deluxe
        // 0.10.0 shows or hides it on its own, apart from other plugins.
        own_page: true,
    };

    struct Handler;

    impl PluginHandler for Handler {
        fn current(&self, option: usize) -> Option<usize> {
            let s = SETTINGS.load();
            match option {
                0 => Some(settings::p1_choice(s)),
                1 => Some(usize::from(s.auto_rematch)),
                2 => Some(usize::from(s.auto_save_replay)),
                _ => None,
            }
        }

        fn request(&self, option: usize, choice: usize) -> RequestStatus {
            if !ACTIVE.load(Ordering::Acquire) {
                return RequestStatus::Rejected;
            }
            match option {
                0 if choice < settings::P1_CHOICES.len() => {
                    // Deluxe's request for the current choice is its L+R+X.
                    // The P1 row is on a rows page, where P1's pad stands down
                    // (`pilot_frame`), so this is the press's one step; only
                    // an older Deluxe without the export reads as no rows
                    // page, and in battle its press was stepped by the pad.
                    let skip = !deluxe::request_steps(battle_frames_running(), deluxe_rows_page());
                    SETTINGS.update(|s| {
                        if skip && choice == settings::p1_choice(s) {
                            s
                        } else {
                            settings::p1_request(s, choice)
                        }
                    });
                    DIRTY.store(true, Ordering::Release);
                }
                1 | 2 if choice < 2 => {
                    let on = choice == 1;
                    SETTINGS.update(|s| match option {
                        1 => Settings { auto_rematch: on, ..s },
                        _ => Settings { auto_save_replay: on, ..s },
                    });
                }
                _ => return RequestStatus::Rejected,
            }
            RequestStatus::Applied
        }
    }

    pub fn register_rows() -> &'static str {
        match register(&SPEC, Handler) {
            Ok(_) => "registered",
            Err(deluxe_plugin_sdk::RegisterError::HostMissing) => "host_missing",
            Err(_) => "refused",
        }
    }
}

// ---------------------------------------------------------------------------
// Files: the install line, config.ini, and (diag) per-event lines.

type Path = Line<96>;

fn path(args: core::fmt::Arguments) -> Path {
    let mut path = Path::new();
    let _ = path.write_fmt(args);
    path
}

struct NxFile {
    handle: nn::fs::FileHandle,
    offset: i64,
}

impl NxFile {
    fn open(path: &mut Path, truncate: bool) -> Option<Self> {
        let path = path.with_nul()?.as_ptr();
        unsafe {
            if truncate {
                nn::fs::DeleteFile(path);
            }
            nn::fs::CreateFile(path, 0);
            let mut handle = nn::fs::FileHandle { handle: 0 };
            if nn::fs::OpenFile(&mut handle, path, OPEN_WRITE_APPEND) != 0 {
                return None;
            }
            let mut offset = 0;
            if nn::fs::GetFileSize(&mut offset, handle) != 0 {
                nn::fs::CloseFile(handle);
                return None;
            }
            Some(Self { handle, offset })
        }
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        let option = nn::fs::WriteOption { flags: 0 };
        let written = unsafe {
            nn::fs::WriteFile(self.handle, self.offset, bytes.as_ptr(), bytes.len() as u64, &option)
        } == 0;
        if written {
            self.offset += bytes.len() as i64;
        }
        written
    }
}

impl Drop for NxFile {
    fn drop(&mut self) {
        unsafe {
            nn::fs::FlushFile(self.handle);
            nn::fs::CloseFile(self.handle);
        }
    }
}

fn append_log(bytes: &[u8]) {
    if let Some(mut log) = NxFile::open(&mut path(format_args!("{}/events.log", model::DATA_DIR)), false) {
        log.write(bytes);
    }
}

fn read_file(name: &mut Path, buf: &mut [u8]) -> usize {
    let Some(file) = name.with_nul() else {
        return 0;
    };
    let mut read = 0u64;
    unsafe {
        let mut handle = nn::fs::FileHandle { handle: 0 };
        if nn::fs::OpenFile(&mut handle, file.as_ptr(), OPEN_READ) != 0 {
            return 0;
        }
        let ok = nn::fs::ReadFile1(&mut read, handle, 0, buf.as_mut_ptr(), buf.len() as u64) == 0;
        nn::fs::CloseFile(handle);
        if !ok {
            return 0;
        }
    }
    (read as usize).min(buf.len())
}

/// Loads `config.ini`. P1 always starts as the player: a CPU left on before
/// a reboot does not take the pad at the next boot.
fn load_settings() -> Settings {
    let mut buf = [0u8; 512];
    let n = read_file(&mut path(format_args!("{}", model::CONFIG_INI)), &mut buf);
    let loaded = settings::parse_ini(core::str::from_utf8(&buf[..n]).unwrap_or(""));
    Settings { pilot: false, ..loaded }
}

fn save_settings() {
    let mut buf = [0u8; 96];
    if let Some(n) = settings::write_ini(SETTINGS.load(), &mut buf) {
        if let Some(mut file) = NxFile::open(&mut path(format_args!("{}", model::CONFIG_INI)), true) {
            file.write(&buf[..n]);
        }
    }
}

#[cfg(feature = "diag")]
mod diag {
    use super::*;
    use std::sync::atomic::fence;

    const EVENT_RING: usize = 32;
    const EVENT_LEN: usize = 256;

    struct EventSlot {
        seq: AtomicU64,
        text: Owned<Line<EVENT_LEN>>,
    }

    static EVENTS: [EventSlot; EVENT_RING] = [const {
        EventSlot {
            seq: AtomicU64::new(0),
            text: Owned(UnsafeCell::new(Line::new())),
        }
    }; EVENT_RING];
    static EVENT_HEAD: AtomicU64 = AtomicU64::new(0);
    static LAST_GATE: AtomicUsize = AtomicUsize::new(0);
    static P1_CALLS: AtomicU64 = AtomicU64::new(0);

    pub fn push(args: core::fmt::Arguments) {
        let index = EVENT_HEAD.fetch_add(1, Ordering::AcqRel);
        let slot = &EVENTS[index as usize % EVENT_RING];
        slot.seq.store(0, Ordering::Release);
        // SAFETY: this index owns the slot until it publishes `seq`.
        let text = unsafe { &mut *slot.text.0.get() };
        text.clear();
        let _ = write!(text, "{} ", tick());
        let _ = text.write_fmt(args);
        let _ = text.write_char('\n');
        slot.seq.store(index + 1, Ordering::Release);
    }

    pub fn gate(name: &'static str) {
        let id = name.as_ptr() as usize;
        if LAST_GATE.swap(id, Ordering::AcqRel) != id {
            push(format_args!("gate {name} armed={}", ARMED_LEVEL.load(Ordering::Acquire)));
        }
    }

    /// One line per 600 P1 input mappings, to show the call rate.
    pub fn count() {
        let n = P1_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
        if n % 600 == 0 {
            push(format_args!("p1_inputs calls={n} last_input={:#018x}", LAST_INPUT.load(Ordering::Acquire)));
        }
    }

    static DIAG_HASH: AtomicU64 = AtomicU64::new(0);

    /// Eden switch: when `diag.txt` changes, its `key=value` lines (the
    /// `config.ini` keys) replace the settings, including `p1=cpu`.
    pub fn reload() {
        let mut buf = [0u8; 256];
        let n = read_file(&mut path(format_args!("{}/diag.txt", model::DATA_DIR)), &mut buf);
        let hash = buf[..n]
            .iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3));
        if n == 0 || DIAG_HASH.swap(hash, Ordering::AcqRel) == hash {
            return;
        }
        let wanted = settings::parse_ini(core::str::from_utf8(&buf[..n]).unwrap_or(""));
        SETTINGS.store(wanted);
        push(format_args!(
            "diag settings p1={} level={} auto_rematch={} auto_save_replay={}",
            u8::from(wanted.pilot),
            wanted.level,
            u8::from(wanted.auto_rematch),
            u8::from(wanted.auto_save_replay)
        ));
    }

    pub fn drain(tail: &mut u64) {
        let head = EVENT_HEAD.load(Ordering::Acquire);
        if head == *tail {
            return;
        }
        if head - *tail > EVENT_RING as u64 {
            *tail = head - EVENT_RING as u64;
        }
        let Some(mut file) = NxFile::open(&mut path(format_args!("{}/events.log", model::DATA_DIR)), false) else {
            return;
        };
        while *tail < head {
            let slot = &EVENTS[*tail as usize % EVENT_RING];
            let seq = slot.seq.load(Ordering::Acquire);
            if seq == 0 || seq < *tail + 1 {
                break;
            }
            let mut copy = Line::<EVENT_LEN>::new();
            // SAFETY: read under the sequence check below.
            let text = unsafe { &*slot.text.0.get() };
            let _ = copy.write_str(core::str::from_utf8(text.as_bytes()).unwrap_or(""));
            fence(Ordering::Acquire);
            if seq == *tail + 1 && slot.seq.load(Ordering::Relaxed) == seq {
                file.write(copy.as_bytes());
            }
            *tail += 1;
        }
    }
}

#[inline]
pub(crate) fn diag_event(_args: core::fmt::Arguments) {
    #[cfg(feature = "diag")]
    diag::push(_args);
}

#[inline]
fn diag_gate(_name: &'static str) {
    #[cfg(feature = "diag")]
    diag::gate(_name);
}

#[inline]
fn diag_count() {
    #[cfg(feature = "diag")]
    diag::count();
}

// The worker thread: saves config.ini after a pad level change, looks up
// Deluxe's window-state export until found and (diag) appends the event
// lines. Its object and stack are static.
const WORKER_STACK: usize = 32 * 1024;
#[repr(C, align(4096))]
struct ThreadStack(UnsafeCell<[u8; WORKER_STACK]>);
#[repr(C, align(16))]
struct ThreadObject(UnsafeCell<[u8; 0x1c0]>);
// SAFETY: handed to nn::os once, at startup, and never touched again.
unsafe impl Sync for ThreadStack {}
unsafe impl Sync for ThreadObject {}
static WORKER_STACK_MEMORY: ThreadStack = ThreadStack(UnsafeCell::new([0; WORKER_STACK]));
static WORKER_THREAD: ThreadObject = ThreadObject(UnsafeCell::new([0; 0x1c0]));

extern "C" fn worker_main(_: *mut skyline::libc::c_void) {
    #[cfg(feature = "diag")]
    let mut tail = 0;
    #[cfg(feature = "diag")]
    let mut round = 0u32;
    loop {
        unsafe { nn::os::SleepThread(nn::TimeSpan { nanoseconds: 200_000_000 }) };
        if DIRTY.swap(false, Ordering::AcqRel) {
            save_settings();
        }
        watch_battle_end();
        // SAFETY: `main` stopped calling it before starting this thread.
        unsafe { find_deluxe_overlay() };
        #[cfg(feature = "diag")]
        {
            round = round.wrapping_add(1);
            if round % 10 == 0 {
                diag::reload();
            }
            diag::drain(&mut tail);
        }
    }
}

fn start_worker() -> bool {
    unsafe {
        let thread = WORKER_THREAD.0.get().cast::<nn::os::ThreadType>();
        let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
        let created = nn::os::CreateThread(
            thread,
            worker_main,
            core::ptr::null_mut(),
            WORKER_STACK_MEMORY.0.get().cast::<u8>(),
            WORKER_STACK,
            priority,
        );
        if created != 0 {
            return false;
        }
        nn::os::SetThreadNamePointer(thread, c"ssbu-cpu-pilot".as_ptr().cast());
        nn::os::StartThread(thread);
        true
    }
}

// ---------------------------------------------------------------------------
// Install.

pub(crate) unsafe fn site_address(site: &Site) -> Option<u64> {
    let start = getRegionAddress(Region::Text) as u64;
    let end = getRegionAddress(Region::Rodata) as u64;
    let address = start + site.offset as u64;
    (address % 4 == 0 && address + 64 <= end).then_some(address)
}

/// `chain` has one stub: the first site that needs it (another plugin's far
/// jump over the site) takes it; a later such site is left alone.
static STUB_USED: AtomicBool = AtomicBool::new(false);

pub(crate) unsafe fn hook(site: &Site, callback: unsafe extern "C" fn(&mut InlineCtx)) -> (Entry, Outcome) {
    let Some(address) = site_address(site) else {
        return (Entry::Foreign, Outcome::Skipped);
    };
    let stock = site.stock();
    let entry = chain::entry(&stock, &chain::hook::words_at(address), address);
    if matches!(entry, Entry::Chain(_)) && STUB_USED.swap(true, Ordering::Relaxed) {
        return (entry, Outcome::Skipped);
    }
    chain::hook::inline(address, &stock, callback)
}

pub(crate) fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    core::str::from_utf8(&version.name[..end]).is_ok_and(model::version_supported)
}

fn report_install(report: &mut Line<768>) {
    skyline::logging::log(core::str::from_utf8(report.as_bytes()).unwrap_or(""));
    let _ = report.write_char('\n');
    for directory in ["sd:/ultimate", model::DATA_DIR] {
        if let Some(directory) = path(format_args!("{directory}")).with_nul() {
            unsafe { nn::fs::CreateDirectory(directory.as_ptr()) };
        }
    }
    append_log(report.as_bytes());
}

#[skyline::main(name = "ssbu-cpu-pilot")]
pub fn main() {
    let mut report = Line::<768>::new();
    let _ = write!(report, "{} plugin={}/{}", tick(), model::PLUGIN, model::VERSION);
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    TEXT_BASE.store(unsafe { getRegionAddress(Region::Text) as usize }, Ordering::Relaxed);
    // The pilot's hooks first: they get the one chain stub if they need it.
    let result = unsafe { install(&mut report) };
    // Online: the AI-service hooks. A failure there (or in the mode predicate
    // it relies on) turns online off only; offline piloting stays installed.
    // Owners keep the shared manager hooks available to guests regardless.
    let mode_ok = unsafe { site_usable(&model::MODE_GLOBALS) };
    if !mode_ok {
        let _ = write!(report, " site={} guard=mismatch online=off", model::MODE_GLOBALS.name);
    }
    unsafe { online_hooks::install(&mut report, result == "installed" && mode_ok) };
    unsafe { crate::results::install(&mut report, results_frame) };
    let _ = write!(report, " result={result} heap_allocations={}", HEAP_ALLOCATIONS.load(Ordering::Relaxed));
    // The Deluxe SDK allocates while registering; that count is kept apart
    // from the hooks' (which never allocate).
    if result == "installed" {
        let before = HEAP_ALLOCATIONS.load(Ordering::Relaxed);
        let menu = menu::register_rows();
        let _ = write!(
            report,
            " menu={menu} menu_allocations={}",
            HEAP_ALLOCATIONS.load(Ordering::Relaxed) - before
        );
        // SAFETY: the worker, the only other caller, is not started yet.
        let overlay = unsafe { find_deluxe_overlay() };
        let _ = write!(report, " deluxe_overlay={}", if overlay { "found" } else { "missing" });
        if !start_worker() {
            let _ = write!(report, " worker=not_started");
        }
    }
    report_install(&mut report);
}

unsafe fn install(report: &mut Line<768>) -> &'static str {
    for (slot, symbol) in [
        (&MANAGER_SLOT, SYM_MANAGER),
        (&GET_ENTRY, SYM_GET_ENTRY),
        (&IS_READY_GO, SYM_READY_GO),
        (&IS_RESULT_MODE, SYM_RESULT_MODE),
    ] {
        let address = lookup(symbol);
        if address == 0 {
            let name = core::str::from_utf8(&symbol[..symbol.len() - 1]).unwrap_or("?");
            let _ = write!(report, " missing_symbol={name}");
            return "not_installed";
        }
        slot.store(address, Ordering::Relaxed);
    }
    let Some(ranks) = level::ranks(native::rank_table(base())) else {
        let _ = write!(report, " rank_table=mismatch");
        return "not_installed";
    };
    (*PILOT.0.get()).ranks = ranks;
    for site in model::SITES.iter().filter(|s| s.kind == Kind::Called && !s.hud) {
        if !site_usable(site) {
            let _ = write!(report, " site={} guard=mismatch", site.name);
            return "not_installed";
        }
    }
    let loaded = load_settings();
    SETTINGS.store(loaded);
    let _ = write!(
        report,
        " level={} auto_rematch={} auto_save_replay={}",
        loaded.level,
        u8::from(loaded.auto_rematch),
        u8::from(loaded.auto_save_replay)
    );
    let (entry, outcome) = hook(&model::FIM_VALID, fim_valid);
    let _ = write!(report, " fim_valid={}/{}", entry.name(), outcome.name());
    if !outcome.hooked() {
        return "not_installed";
    }
    let (entry, outcome) = hook(&model::FIM_EXIT, fim_exit);
    let _ = write!(report, " fim_exit={}/{}", entry.name(), outcome.name());
    if !outcome.hooked() {
        // The exit hook never ran, so the marker hook only sets a flag.
        return "not_installed";
    }
    install_hud(report);
    ACTIVE.store(true, Ordering::Release);
    "installed"
}

unsafe fn site_usable(site: &Site) -> bool {
    site_address(site).is_some_and(|a| site.usable(&chain::hook::words_at(a), a))
}

/// The HUD mark. Never fails the plugin: whatever is wrong, the pilot is
/// installed and the line says `hud=off`.
unsafe fn install_hud(report: &mut Line<768>) {
    for site in model::SITES.iter().filter(|s| s.kind == Kind::Called && s.hud) {
        if !site_usable(site) {
            let _ = write!(report, " hud=off site={} guard=mismatch", site.name);
            return;
        }
    }
    let (entry, outcome) = hook(&model::HUD_PASS, hud_tick);
    if outcome.hooked() {
        let _ = write!(report, " hud={}/{}", entry.name(), outcome.name());
    } else {
        let _ = write!(report, " hud=off hud_pass={}/{}", entry.name(), outcome.name());
    }
}
