//! Switch results adapter. Three post-prologue inline sites: completed UI
//! records, a borrowed cards snapshot, and the shared online-rematch parse.
//! All policy advances on the UI-builder callback. No native calls, popup
//! writes, replay-commit hooks, or retained/dereferenced cards pointers.
//! Online the save uses the same native chain as offline: the cards' update
//! checks Y on each card's mapped UI port (`1d59d8c` → `1d5b590`) while the
//! save is offered (`+0x930`), then the dialog, the write and the success
//! pop-up publish statuses 1, 2, 3 and 5 as offline.
use crate::{chain, model, runtime, share};
use core::cell::UnsafeCell;
use core::fmt::Write;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use ssbu_cpu_pilot_core::{results as policy, settings::Settings};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static BUSY: AtomicBool = AtomicBool::new(false);
struct Owned<T>(UnsafeCell<T>);
// All accesses take BUSY; callbacks never block the UI thread.
unsafe impl<T> Sync for Owned<T> {}
struct State {
    flow: policy::Results,
    frame: u64,
    cards_epoch: usize,
    cards_frame: u64,
    cards_ready: bool,
    cards_port: Option<u8>,
    rematch_epoch: usize,
    last_epoch: usize,
    #[cfg(feature = "diag")]
    cards: CardsDiag,
}
/// The cards' fields behind the readiness test, logged whenever one changes
/// (at most `CARDS_LINES` lines per epoch) so an Eden run shows when the
/// online results offered the save and which field held it back: `+0x928`
/// route, `+0x930` save offered, `+0x910` no-save mode, `+0x924` finished,
/// the cards' phase and pending, the first two cards' states, the modal mask,
/// the pop-up idle test, the save status and the first two entries' UI ports.
#[cfg(feature = "diag")]
#[derive(Clone, Copy, PartialEq, Eq)]
struct CardsSig {
    route: u32,
    offered: u8,
    no_save: u8,
    finished: u8,
    phase: i32,
    pending: i32,
    state: [i32; 2],
    modal: u32,
    idle: u8,
    ready: u8,
    status: i32,
    ports: [i32; 2],
}
#[cfg(feature = "diag")]
const CARDS_SIG: CardsSig = CardsSig {
    route: 0,
    offered: 0,
    no_save: 0,
    finished: 0,
    phase: 0,
    pending: 0,
    state: [0; 2],
    modal: 0,
    idle: 0,
    ready: 0,
    status: 0,
    ports: [0; 2],
};
#[cfg(feature = "diag")]
const CARDS_LINES: u8 = 32;
#[cfg(feature = "diag")]
struct CardsDiag {
    /// The latest sample, written by the cards hook.
    now: CardsSig,
    /// The last logged sample of `logged_epoch`, `None` before the first.
    last: Option<CardsSig>,
    lines: u8,
    logged_epoch: usize,
}
static STATE: Owned<State> = Owned(UnsafeCell::new(State {
    flow: policy::Results::new(),
    frame: 0,
    cards_epoch: 0,
    cards_frame: 0,
    cards_ready: false,
    cards_port: None,
    rematch_epoch: 0,
    last_epoch: 0,
    #[cfg(feature = "diag")]
    cards: CardsDiag {
        now: CARDS_SIG,
        last: None,
        lines: 0,
        logged_epoch: 0,
    },
}));

fn with_state(f: impl FnOnce(&mut State)) {
    if BUSY
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    f(unsafe { &mut *STATE.0.get() });
    BUSY.store(false, Ordering::Release);
}

fn base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}
unsafe fn read<T: Copy>(at: usize) -> T {
    (at as *const T).read_volatile()
}
unsafe fn pointer(at: usize) -> Option<usize> {
    if at == 0 || at % 8 != 0 {
        return None;
    }
    let p = read::<usize>(at);
    (p != 0 && p % 8 == 0).then_some(p)
}
unsafe fn singleton(offset: usize) -> Option<usize> {
    pointer(pointer(base() + offset)? + 8)
}
unsafe fn pending(obj: usize) -> Option<i32> {
    let index = read::<i32>(obj + 0xf4);
    (0..19)
        .contains(&index)
        .then(|| read::<i32>(obj + 0xa8 + index as usize * 4))
}
unsafe fn epoch() -> usize {
    singleton(0x52c35d0).unwrap_or(0)
}
unsafe fn status() -> Option<i32> {
    singleton(0x52c35c8).map(|p| read(p + 0x10))
}

/// The UI port of fighter entry `entry` (`[[main+52c50f0]+0x298+4*entry]`,
/// -1 when none), the table the cards' save loop reads (`1d59dc4`).
unsafe fn entry_port(entry: usize) -> Option<u8> {
    let table = pointer(base() + 0x52c50f0)? + 0x298;
    let port = read::<i32>(table + entry * 4);
    (0..10).contains(&port).then_some(port as u8)
}

/// Online only the local fighter entry maps to a UI port; the game checks Y
/// on each card's own port (`1d59dc4` → `1d5b590`), so the save's input must
/// ride that port, whichever entry the local player is (slot 1 in Eden r02).
unsafe fn mapped_port() -> Option<u8> {
    (0..8).find_map(|entry| entry_port(entry))
}

unsafe fn popup() -> Option<policy::Popup> {
    let wrapper = pointer(base() + 0x532f820)?;
    let p = pointer(wrapper + 8)?;
    let d = pointer(wrapper + 0x10)?;
    let ui = pointer(pointer(p + 0x110)? + 0x80)?;
    let selection = pointer(ui + 0xd8)?;
    Some(policy::Popup {
        record: read(d + 0x198),
        phase: read(p + 0xf8),
        pending: pending(p)?,
        age: read::<i32>(p + 0xfc).max(0) as u32,
        result: read(p + 0x118),
        decided: read::<u8>(ui + 0x14d) != 0,
        opening: read::<u8>(p + 0x142) != 0,
        default_button: read(d + 0x160),
        cancel: read(d + 0x16c),
        input_mode: read(d + 0x170),
        cursor: read(selection + 0x250),
    })
}

/// x19 is borrowed only here, while the native cards callback owns it.
unsafe extern "C" fn cards(ctx: &mut InlineCtx) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    let c = ctx.registers[19].x() as usize;
    if c == 0 || c % 8 != 0 {
        return;
    }
    let e = epoch();
    if e == 0 {
        return;
    }
    let port = entry_port(0); // offline P1: fighter-entry 0 -> UI port
    let idle = singleton(0x532f820)
        .is_some_and(|p| read::<i32>(p + 0xf8) == 0 && read::<u8>(p + 0x142) == 0);
    let ready = read::<i32>(c + 0xf8) == 2
        && pending(c) == Some(2)
        && read::<u8>(c + 0x924) == 0
        && read::<u8>(c + 0x930) == 1
        && status() == Some(0)
        && idle
        && singleton(0x593b988).is_some_and(|p| read::<u32>(p) <= 1);
    with_state(|s| {
        s.cards_epoch = e;
        s.cards_frame = s.frame;
        s.cards_ready = ready;
        s.cards_port = port;
        #[cfg(feature = "diag")]
        {
            s.cards.now = CardsSig {
                route: read(c + 0x928),
                offered: read(c + 0x930),
                no_save: read(c + 0x910),
                finished: read(c + 0x924),
                phase: read(c + 0xf8),
                pending: pending(c).unwrap_or(-1),
                state: [read(c + 0x118), read(c + 0x218)],
                modal: singleton(0x593b988).map_or(u32::MAX, |p| read::<u32>(p)),
                idle: u8::from(idle),
                ready: u8::from(ready),
                status: status().unwrap_or(-1),
                ports: [raw_entry_port(0), raw_entry_port(1)],
            };
        }
    });
}

/// The port table's raw value for `entry` (-1 unmapped, -2 no table), diag only.
#[cfg(feature = "diag")]
unsafe fn raw_entry_port(entry: usize) -> i32 {
    pointer(base() + 0x52c50f0).map_or(-2, |t| read::<i32>(t + 0x298 + entry * 4))
}

/// Shared callback is an observer. It never reparses, writes registers, or
/// treats elite-rematch's rewritten Elite notice as the ordinary record.
unsafe extern "C" fn parsed(ctx: &mut InlineCtx) {
    let wrapper = ctx.registers[19].x() as usize;
    if pointer(base() + 0x532f820) != Some(wrapper) {
        return;
    }
    let e = epoch();
    let qualified = ctx.registers[20].x() == policy::REMATCH_RECORD;
    with_state(|s| {
        s.rematch_epoch = if qualified { e } else { 0 };
    });
}

/// Called from runtime's UI exit hook, after all ten records were rebuilt.
pub unsafe fn frame(records: *mut [policy::Record; 10], settings: Settings) {
    if !ENABLED.load(Ordering::Acquire) || records.is_null() || records as usize % 4 != 0 {
        return;
    }
    with_state(|s| {
        s.frame = s.frame.wrapping_add(1);
        let e = epoch();
        if e == 0 {
            s.rematch_epoch = 0;
            s.cards_epoch = 0;
        }
        let scene = policy::scene(
            read::<u8>(base() + ssbu_cpu_pilot_core::native::offsets::MODE_INITIALIZED) & 1 != 0,
            read::<u8>(base() + ssbu_cpu_pilot_core::native::offsets::MELEE_MODE),
            read(base() + ssbu_cpu_pilot_core::native::offsets::GAME_MODE),
        );
        // Any cards sample taken in this epoch, ready or not, however stale.
        let cards_seen = e != 0 && s.cards_epoch == e;
        let fresh_cards = cards_seen && s.frame.wrapping_sub(s.cards_frame) <= 1;
        // Local Smash maps fighter-entry 0 through the game's port table, which
        // is only sampled with the cards: before them (the winner splash) the
        // port is None and the core presses its splash A on UI port 0 itself.
        // Online the local entry's mapped port carries Y, Right and A; UI port
        // 0 when the table maps none (the port the rematch used in Eden r02).
        let port = if e == 0 {
            None
        } else if scene == policy::Scene::Online {
            mapped_port().or(Some(0))
        } else if s.cards_epoch == e {
            s.cards_port
        } else {
            None
        };
        #[cfg(feature = "diag")]
        if cards_seen {
            if s.cards.logged_epoch != e {
                s.cards.logged_epoch = e;
                s.cards.last = None;
                s.cards.lines = 0;
            }
            let c = s.cards.now;
            if s.cards.last != Some(c) && s.cards.lines < CARDS_LINES {
                s.cards.last = Some(c);
                s.cards.lines += 1;
                runtime::diag_event(format_args!(
                    "results epoch={:#x} port={} cards=sample n={} route={} offered={} no_save={} finished={} phase={} pending={} state0={} state1={} modal={} idle={} ready={} fresh={} status={} ports={},{} mode={:#x}",
                    e,
                    port.unwrap_or(255),
                    s.cards.lines,
                    c.route,
                    c.offered,
                    c.no_save,
                    c.finished,
                    c.phase,
                    c.pending,
                    c.state[0],
                    c.state[1],
                    c.modal,
                    c.idle,
                    c.ready,
                    u8::from(fresh_cards),
                    c.status,
                    c.ports[0],
                    c.ports[1],
                    read::<u32>(base() + 0x53050f0)
                ));
            }
        }
        let seen = policy::Frame {
            epoch: e,
            scene,
            port,
            cards_seen,
            cards_ready: fresh_cards && s.cards_ready,
            save_status: if e != 0 { status() } else { None },
            popup: if e != 0 { popup() } else { None },
            rematch_epoch: s.rematch_epoch,
        };
        #[cfg(feature = "diag")]
        let step = s.flow.stage();
        let out = s.flow.step(seen, settings);
        if let Some(port) = out.port {
            out.input.apply(&mut *records, port);
        }
        #[cfg(feature = "diag")]
        if let Some(event) = out.event {
            // The splash A rides UI port 0 before any port is mapped.
            let shown = if matches!(event, policy::Event::SplashA(_)) {
                out.port
            } else {
                port
            };
            runtime::diag_event(format_args!(
                "results epoch={:#x} port={} {} step={:?} status={} popup={:#x} phase={} cursor={} mode={:#x}",
                if e == 0 { s.last_epoch } else { e },
                shown.unwrap_or(255),
                event_name(event),
                step,
                seen.save_status.unwrap_or(-1),
                seen.popup.map_or(0, |p| p.record),
                seen.popup.map_or(-1, |p| p.phase),
                seen.popup.map_or(-1, |p| p.cursor),
                read::<u32>(base() + 0x53050f0)
            ));
        }
        s.last_epoch = e;
    });
}

#[cfg(feature = "diag")]
fn event_name(event: policy::Event) -> &'static str {
    use policy::Event::*;
    match event {
        Begin => "epoch=begin",
        SaveLate => "save=late_rematch_popup",
        Unsupported => "flow=unsupported",
        // Static text keeps the diag path free of allocation; n is 1..=SPLASH_MAX.
        SplashA(n) => {
            const NAMES: [&str; 12] = [
                "save=press_a_splash n=1",
                "save=press_a_splash n=2",
                "save=press_a_splash n=3",
                "save=press_a_splash n=4",
                "save=press_a_splash n=5",
                "save=press_a_splash n=6",
                "save=press_a_splash n=7",
                "save=press_a_splash n=8",
                "save=press_a_splash n=9",
                "save=press_a_splash n=10",
                "save=press_a_splash n=11",
                "save=press_a_splash n=12",
            ];
            const _: () = assert!(NAMES.len() == policy::SPLASH_MAX as usize);
            NAMES[usize::from(n.clamp(1, policy::SPLASH_MAX)) - 1]
        }
        SaveY => "save=press_y",
        SaveRight => "save=select_yes",
        SaveConfirm => "save=confirm",
        Saving => "save=writing",
        SaveClose => "save=close_success",
        Saved => "save=completed",
        RematchA => "rematch=press_a",
        RematchYes => "rematch=accepted",
        Timeout => "flow=timeout",
        FailedStatus => "save=failed_status_4",
        UnexpectedStatus => "save=unexpected_status",
        LostPopup => "flow=lost_popup",
        WrongAnswer => "flow=wrong_answer",
        Cancelled => "flow=cancelled",
        SceneChanged => "flow=scene_changed",
        PortChanged => "flow=port_changed",
    }
}

// Shared rematch site owns its own far-chain stub. The existing pilot's sole
// chain::hook::inline stub must never be reused for a second site's far jump.
static REMATCH_TARGET: AtomicU64 = AtomicU64::new(0);
extern "C" {
    fn ssbu_results_rematch_stub();
}
core::arch::global_asm!(
    ".pushsection .text.ssbu_results_rematch_stub, \"ax\", %progbits",
    ".balign 16", ".hidden ssbu_results_rematch_stub",
    ".global ssbu_results_rematch_stub", "ssbu_results_rematch_stub:",
    "nop", "nop", "nop", "nop", "nop",
    "adrp x17, {target}", "ldr x17, [x17, #:lo12:{target}]", "br x17",
    ".popsection", target = sym REMATCH_TARGET,
);
static GUESTS: share::Callbacks = share::Callbacks::new();
static SHARE_STATE: AtomicU8 = AtomicU8::new(0);
static TABLE: share::SiteShareV1 = share::SiteShareV1 {
    abi_version: share::ABI_VERSION,
    offset: model::RESULTS_REMATCH.offset as u64,
    register,
};
#[no_mangle]
pub extern "C" fn ssbu_inline_site_147fb64_v1() -> *const share::SiteShareV1 {
    &TABLE
}

unsafe extern "C" fn register(callback: usize) -> bool {
    let usable = match SHARE_STATE.load(Ordering::Acquire) {
        0 => {
            runtime::version_is_supported()
                && runtime::site_address(&model::RESULTS_REMATCH).is_some_and(|a| {
                    chain::entry(
                        &model::RESULTS_REMATCH.stock(),
                        &chain::hook::words_at(a),
                        a,
                    ) != chain::Entry::Foreign
                })
        }
        state => state == 1,
    };
    usable && GUESTS.push(callback)
}
unsafe extern "C" fn owner(ctx: &mut InlineCtx) {
    parsed(ctx);
    GUESTS.for_each(|address| {
        let callback: unsafe extern "C" fn(*mut InlineCtx) = core::mem::transmute(address);
        callback(ctx);
    });
}
unsafe fn shared(report: &mut impl Write) -> bool {
    let mut export = 0usize;
    nn::ro::LookupSymbol(&mut export, b"ssbu_inline_site_147fb64_v1\0".as_ptr());
    let found = if export == 0 {
        0
    } else {
        let export: extern "C" fn() -> *const share::SiteShareV1 = core::mem::transmute(export);
        export() as usize
    };
    // Election precedes byte checking: the owner may already have patched it.
    match share::role(found, &TABLE as *const _ as usize) {
        share::Role::Guest(table) => {
            SHARE_STATE.store(2, Ordering::Release);
            let api = &*(table as *const share::SiteShareV1);
            let ok = share::compatible(api, model::RESULTS_REMATCH.offset)
                && (api.register)(parsed as *const () as usize);
            let _ = write!(
                report,
                " results_rematch={}",
                if ok { "guest" } else { "refused" }
            );
            ok
        }
        share::Role::Owner => {
            let (entry, outcome) = match runtime::site_address(&model::RESULTS_REMATCH) {
                Some(a) => chain::hook::inline_with_stub(
                    a,
                    &model::RESULTS_REMATCH.stock(),
                    owner,
                    ssbu_results_rematch_stub as *const () as usize as u64,
                    &REMATCH_TARGET,
                ),
                None => (chain::Entry::Foreign, chain::hook::Outcome::Skipped),
            };
            SHARE_STATE.store(if outcome.hooked() { 1 } else { 2 }, Ordering::Release);
            let _ = write!(
                report,
                " results_rematch=owner:{}/{} guests={}",
                entry.name(),
                outcome.name(),
                GUESTS.len()
            );
            outcome.hooked()
        }
    }
}

pub unsafe fn install(report: &mut impl Write, callback: unsafe extern "C" fn(&mut InlineCtx)) {
    // Keep the shared hook available to guests even when our input adapter
    // fails a dependency. parsed() only publishes a private identity.
    let rematch = shared(report);
    let mut dependencies = true;
    for site in model::SITES
        .iter()
        .filter(|s| s.kind == model::Kind::Relied)
    {
        let ok = runtime::site_address(site).is_some_and(|a| {
            chain::usable(&site.stock(), &chain::hook::words_at(a), a, chain::WORDS)
        });
        if !ok {
            dependencies = false;
            let _ = write!(report, " site={} guard=mismatch", site.name);
        }
    }
    if !dependencies {
        let _ = write!(report, " results=off");
        return;
    }
    let (ce, co) = runtime::hook(&model::RESULTS_CARDS, cards);
    let (ie, io) = runtime::hook(&model::RESULTS_INPUT, callback);
    let _ = write!(
        report,
        " results_cards={}/{} results_input={}/{} rematch_ready={}",
        ce.name(),
        co.name(),
        ie.name(),
        io.name(),
        u8::from(rematch)
    );
    // Offline saving still works if the optional rematch sharing site refuses.
    ENABLED.store(co.hooked() && io.hooked(), Ordering::Release);
}
