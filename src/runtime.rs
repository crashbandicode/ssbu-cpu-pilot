//! Switch only: version check, site guards, the two inline hooks on final
//! input mapping (`0x1750dd0`), the P1 pilot, the Deluxe rows, the exports
//! and the install line.
//!
//! Scope: offline battles with a CPU in them (Training, local Smash). The
//! native AI is ready only while the AI service is in its battle phase
//! (`service+0x1fc == 2`), which an online match between two people never
//! reaches without the extra initialization the fork's online path forces;
//! this plugin does not force it, so it never drives P1 online.
use crate::chain::{self, hook::Outcome, Entry};
use crate::model::{self, buttons, Kind, Line, Site};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use ssbu_cpu_pilot_core::combo::{self, Combo};
use ssbu_cpu_pilot_core::native::{self, Native, Stream};
use ssbu_cpu_pilot_core::pad::{self, MappedInputs};
use ssbu_cpu_pilot_core::session::{Action, Frame, Session};
use ssbu_cpu_pilot_core::settings::{self, Settings, SharedSettings};
use ssbu_cpu_pilot_core::level;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};

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
/// The level changed on the pad; the worker saves `config.ini`.
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
/// player index 0.
struct Pilot {
    session: Session,
    combo: Combo,
    armed: Option<Native>,
    old_rank: i32,
    stream: Stream,
    /// The 13.0.5 level ladder, checked at boot.
    ranks: [u8; 9],
}

static PILOT: Owned<Pilot> = Owned(UnsafeCell::new(Pilot {
    session: Session::new(),
    combo: Combo::new(),
    armed: None,
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
}

unsafe fn observe() -> Seen {
    let none = |gate| Seen {
        native: None,
        in_battle: false,
        can_think: false,
        gate,
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
    }
}

/// One P1 frame: the L+R+X combo, then the session's action.
unsafe fn pilot_frame(out: *mut MappedInputs, raw_buttons: u32) {
    let p = &mut *PILOT.0.get();
    let driving = p.session.armed().is_some();
    if p.combo.step(buttons::combo_held(raw_buttons), driving) {
        let now = SETTINGS.update(|s| Settings {
            level: combo::next_level(s.level),
            ..s
        });
        DIRTY.store(true, Ordering::Release);
        diag_event(format_args!("combo level={}", now.level));
    }
    let wanted = SETTINGS.load();
    let seen = observe();
    let same = match (p.armed, seen.native) {
        (Some(a), Some(n)) => a.ctrl == n.ctrl && a.ai == n.ai,
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
            release(p, seen.native);
            false
        }
        Action::Arm(level) => arm(p, seen.native, level),
        Action::Rearm(level) => {
            release(p, seen.native);
            arm(p, seen.native, level)
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

unsafe fn arm(p: &mut Pilot, seen: Option<Native>, level: u8) -> bool {
    let Some(n) = seen else {
        return false;
    };
    let rank = p.ranks[usize::from(settings::clamp_level(level) - 1)];
    let (old, stream) = native::arm(base(), n, rank);
    p.armed = Some(n);
    p.old_rank = old;
    p.stream = stream;
    ARMED_LEVEL.store(level, Ordering::Release);
    diag_event(format_args!("arm level={level} rank={rank} old_rank={old}"));
    true
}

unsafe fn release(p: &mut Pilot, seen: Option<Native>) {
    ARMED_LEVEL.store(0, Ordering::Release);
    let Some(armed) = p.armed.take() else {
        return;
    };
    // Restore only on the same live objects; after a scene change they are gone.
    let restored = seen.is_some_and(|n| n == armed);
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

/// `0x1751ce8`: `*out` is complete. P1 (index 0) only.
unsafe extern "C" fn fim_exit(ctx: &mut InlineCtx) {
    let marker = FIM_CALL.swap(0, Ordering::AcqRel);
    if marker == 0 || !ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let out = ctx.registers[19].x() as usize;
    if ctx.registers[23].x() != 0 || out == 0 || out % 4 != 0 {
        return;
    }
    let controller = if marker == 2 { ctx.registers[20].x() as usize } else { 0 };
    let raw = if controller != 0 && controller % 8 == 0 {
        read_u32(controller + 8)
    } else {
        0
    };
    diag_count();
    pilot_frame(out as *mut MappedInputs, raw);
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
    ) -> OptionSpec {
        OptionSpec {
            id,
            label,
            choices,
            config_key: None,
            values: Some(values),
            no_persist: false,
            tones: None,
            read_only: false,
            layout: RowLayout::Value,
        }
    }

    static OPTIONS: [OptionSpec; 3] = [
        row(settings::KEY_PILOT, "P1", &["Player", "CPU"], &["player", "cpu"]),
        row(settings::KEY_REMATCH, "Auto rematch", &["Off", "On"], &["off", "on"]),
        row(settings::KEY_SAVE, "Auto save replay", &["Off", "On"], &["off", "on"]),
    ];

    static SPEC: PluginSpec = PluginSpec {
        id: "ssbu_cpu_pilot",
        label: "CPU Pilot",
        options: &OPTIONS,
        config_path: Some(model::CONFIG_INI),
        own_page: false,
    };

    struct Handler;

    impl PluginHandler for Handler {
        fn current(&self, option: usize) -> Option<usize> {
            let s = SETTINGS.load();
            match option {
                0 => Some(usize::from(s.pilot)),
                1 => Some(usize::from(s.auto_rematch)),
                2 => Some(usize::from(s.auto_save_replay)),
                _ => None,
            }
        }

        fn request(&self, option: usize, choice: usize) -> RequestStatus {
            if option > 2 || choice > 1 || !ACTIVE.load(Ordering::Acquire) {
                return RequestStatus::Rejected;
            }
            let on = choice == 1;
            SETTINGS.update(|s| match option {
                0 => Settings { pilot: on, ..s },
                1 => Settings { auto_rematch: on, ..s },
                _ => Settings { auto_save_replay: on, ..s },
            });
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
fn diag_event(_args: core::fmt::Arguments) {
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

// The worker thread: saves config.ini after a pad level change and (diag)
// appends the event lines. Its object and stack are static.
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

unsafe fn site_address(site: &Site) -> Option<u64> {
    let start = getRegionAddress(Region::Text) as u64;
    let end = getRegionAddress(Region::Rodata) as u64;
    let address = start + site.offset as u64;
    (address % 4 == 0 && address + 64 <= end).then_some(address)
}

unsafe fn hook(site: &Site, callback: unsafe extern "C" fn(&mut InlineCtx), allow_chain: bool) -> (Entry, Outcome) {
    let Some(address) = site_address(site) else {
        return (Entry::Foreign, Outcome::Skipped);
    };
    let stock = site.stock();
    let entry = chain::entry(&stock, &chain::hook::words_at(address), address);
    // `chain` has one stub; only the first hooked site may chain through it.
    if matches!(entry, Entry::Chain(_)) && !allow_chain {
        return (entry, Outcome::Skipped);
    }
    chain::hook::inline(address, &stock, callback)
}

fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    core::str::from_utf8(&version.name[..end]).is_ok_and(model::version_supported)
}

fn report_install(report: &mut Line<384>) {
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
    let mut report = Line::<384>::new();
    let _ = write!(report, "{} plugin={}/{}", tick(), model::PLUGIN, model::VERSION);
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    TEXT_BASE.store(unsafe { getRegionAddress(Region::Text) as usize }, Ordering::Relaxed);
    let result = unsafe { install(&mut report) };
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
        if !start_worker() {
            let _ = write!(report, " worker=not_started");
        }
    }
    report_install(&mut report);
}

unsafe fn install(report: &mut Line<384>) -> &'static str {
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
    for site in model::SITES.iter().filter(|s| s.kind == Kind::Called) {
        let usable = site_address(site).is_some_and(|a| site.usable(&chain::hook::words_at(a), a));
        if !usable {
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
    let (entry, outcome) = hook(&model::FIM_VALID, fim_valid, true);
    let _ = write!(report, " fim_valid={}/{}", entry.name(), outcome.name());
    if !outcome.hooked() {
        return "not_installed";
    }
    let (entry, outcome) = hook(&model::FIM_EXIT, fim_exit, false);
    let _ = write!(report, " fim_exit={}/{}", entry.name(), outcome.name());
    if !outcome.hooked() {
        // The exit hook never ran, so the marker hook only sets a flag.
        return "not_installed";
    }
    ACTIVE.store(true, Ordering::Release);
    "installed"
}
