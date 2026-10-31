//! Online binding and the scene-wide native analysis lease. All state below
//! belongs to the game thread, just like PILOT. No fighter input is written here.
use super::*;
use ssbu_cpu_pilot_core::online::{self, Scope, Stage};

struct Lease {
    manager: usize,
    slot: u8,
    service: usize,
    random: usize,
    stage_id: i32,
    stream: Stream,
    progress: Stage,
}

static SCENE: Owned<Option<(Scope, usize)>> = Owned(UnsafeCell::new(None));
static MASK: Owned<u8> = Owned(UnsafeCell::new(0));
static LEASE: Owned<Option<Lease>> = Owned(UnsafeCell::new(None));
// x29 identifies the enclosing native frame, so a nested call cannot end it.
static CALL: Owned<Option<(u64, native::StageCall)>> = Owned(UnsafeCell::new(None));

/// Find the two human entries by their controller slots, not by entry index.
unsafe fn pair(manager: usize, mask: u8) -> Result<Native, &'static str> {
    let mut slots = [None; 8];
    let mut natives = [None; 8];
    for index in 0..8 {
        let entry = fighter_entry(manager, index as i32);
        if entry == 0 {
            continue;
        }
        if entry % 8 != 0 {
            return Err("online_entry_alignment");
        }
        let (ctrl, info) = native::controller(base(), entry)?;
        let slot = read_u32(ctrl + 8);
        if slot >= 8 {
            return Err("online_slot_bounds");
        }
        let n = native::resolve(base(), ctrl, info, slot as u8)?;
        if !native::single_human(n) {
            return Err("online_subfighter_ai");
        }
        slots[index] = Some(n.slot);
        natives[index] = Some(n);
    }
    let index = online::local_entry(mask, &slots).ok_or("online_pair_or_slot")?;
    let n = natives[index].ok_or("online_local_entry")?;
    if natives.iter().flatten().any(|peer| peer.service != n.service || peer.random != n.random) {
        return Err("online_pair_service");
    }
    Ok(n)
}

pub(super) unsafe fn observe(slot: u8) -> Seen {
    let none = |gate| Seen { native: None, in_battle: false, can_think: false, gate };
    let scope = native::scope(base());
    let current = manager().filter(|&m| !call_bool(&IS_RESULT_MODE, m));
    let scene = current.map(|m| (scope, m));
    if *SCENE.0.get() != scene {
        *SCENE.0.get() = scene;
        *MASK.0.get() = 0;
        *LEASE.0.get() = None;
    }
    let Some(manager) = current else {
        return none("online_no_battle");
    };
    if scope == Scope::Offline {
        return super::observe();
    }
    if scope != Scope::Online1v1 {
        return none("unsupported_scope");
    }
    *MASK.0.get() |= 1 << slot;
    let n = match pair(manager, *MASK.0.get()) {
        Ok(n) => n,
        Err(gate) => return none(gate),
    };
    let ready_go = call_bool(&IS_READY_GO, manager);
    if ready_go && (*LEASE.0.get()).is_none() {
        let (phase, d0, stage_id) = native::stage_state(n.service);
        if stage_id != -1 && stage_id != 0x138 {
            if let Some(progress) = Stage::start(SETTINGS.load().pilot, true, phase, d0) {
                *LEASE.0.get() = Some(Lease {
                    manager,
                    slot: n.slot,
                    service: n.service,
                    random: n.random,
                    stage_id,
                    stream: native::stream_copy(n.random),
                    progress,
                });
                diag_event(format_args!("online stage_start slot={} stage={stage_id} phase={phase}", n.slot));
            }
        }
    }
    let owned = (*LEASE.0.get()).as_ref().is_some_and(|lease| {
        lease.service == n.service && lease.random == n.random && lease.progress == Stage::Ready
    });
    let ready = if owned { native::ready(n) } else { Err("online_analysis_not_owned") };
    Seen {
        native: Some(n),
        in_battle: true,
        can_think: ready_go && ready.is_ok(),
        gate: if !ready_go { "not_ready_go" } else { ready.err().unwrap_or("online_ready") },
    }
}

/// process_begin +0x60, beyond Training Modpack's entry and body guard.
pub(super) unsafe extern "C" fn begin(ctx: &mut InlineCtx) {
    if !ACTIVE.load(Ordering::Acquire) || (*CALL.0.get()).is_some() {
        return;
    }
    let Some(lease) = &mut *LEASE.0.get() else { return };
    let owner = ctx.registers[19].x() as usize;
    // Validate every borrowed singleton before touching it. A lease survives
    // pilot Off and stock loss, but never the scene/service that owns its data.
    if native::scope(base()) != Scope::Online1v1
        || manager() != Some(lease.manager)
        || call_bool(&IS_RESULT_MODE, lease.manager)
        || read_usize(base() + native::offsets::AI_SERVICE) != owner
        || native::service(base()) != Some(lease.service)
        || native::ai_stream(base()) != Some(lease.random)
    {
        *LEASE.0.get() = None;
        return;
    }
    let (_, d0, stage_id) = native::stage_state(lease.service);
    if stage_id != lease.stage_id || d0 & 1 != 0 {
        *LEASE.0.get() = None;
        return;
    }
    // Once phase 2 was reached, a transient unloaded fighter/stock transition
    // must not expose the additional native manager draws. d0 above still
    // relinquishes the lease if a real CPU/subfighter takes over this service.
    // Pending initialization, unlike Ready isolation, needs a live local pair.
    if lease.progress != Stage::Ready {
        if online::local_slot(*MASK.0.get()) != Some(lease.slot)
            || pair(lease.manager, 1 << lease.slot).is_err()
        {
            lease.progress = Stage::Stopped;
            return;
        }
    }
    if !lease.progress.before(SETTINGS.load().pilot) {
        return;
    }
    let call = native::stage_begin(lease.service, lease.random, lease.stream);
    *CALL.0.get() = Some((ctx.registers[29].x(), call));
}

/// The single native epilogue, including every early exit of process_begin.
pub(super) unsafe extern "C" fn end(ctx: &mut InlineCtx) {
    if !(*CALL.0.get()).as_ref().is_some_and(|(frame, _)| *frame == ctx.registers[29].x()) {
        return;
    }
    let Some((_, call)) = (*CALL.0.get()).take() else { return };
    let Some((stream, phase)) = native::stage_end(base(), call) else {
        *LEASE.0.get() = None;
        return;
    };
    if let Some(lease) = &mut *LEASE.0.get() {
        let before = lease.progress;
        lease.stream = stream;
        lease.progress.after(phase);
        if before != lease.progress && !matches!(lease.progress, Stage::Pending(_)) {
            diag_event(format_args!("online stage={:?} phase={phase} rng_restored=1 d0_restored=1", lease.progress));
        }
    }
}
