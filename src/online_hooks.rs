//! Manager hooks have independent far-chain stubs and sharing-v1 providers.
use super::*;
use crate::share::{self, Callbacks, Role, SiteShareV1};

macro_rules! shared_site {
    ($module:ident, $site:ident, $export:ident, $symbol:literal, $stub:ident, $callback:path) => {
        mod $module {
            use super::*;
            static GUESTS: Callbacks = Callbacks::new();
            // 0 = before main; 1 = installed; 2 = refused. After main, never
            // re-guard our own patch when a later-loaded guest registers.
            static STATE: AtomicU8 = AtomicU8::new(0);
            static TARGET: AtomicU64 = AtomicU64::new(0);
            static API: SiteShareV1 = SiteShareV1 {
                abi_version: share::ABI_VERSION,
                offset: model::$site.offset as u64,
                register,
            };

            unsafe extern "C" fn register(callback: usize) -> bool {
                let usable = match STATE.load(Ordering::Acquire) {
                    1 => true,
                    2 => false,
                    _ => version_is_supported() && site_address(&model::$site).is_some_and(|a| {
                        let entry = chain::entry(&model::$site.stock(), &chain::hook::words_at(a), a);
                        match entry {
                            Entry::Foreign => false,
                            Entry::Chain(to) => chain::hook::executable(to, 4),
                            _ => true,
                        }
                    }),
                };
                usable && GUESTS.push(callback)
            }

            #[no_mangle]
            pub extern "C" fn $export() -> *const SiteShareV1 { &API }

            unsafe extern "C" fn dispatch(ctx: &mut InlineCtx) {
                $callback(ctx);
                GUESTS.for_each(|address| {
                    let f: unsafe extern "C" fn(*mut InlineCtx) = core::mem::transmute(address);
                    f(ctx);
                });
            }

            extern "C" { fn $stub(); }
            core::arch::global_asm!(
                concat!(".pushsection .text.", stringify!($stub), ", \"ax\", %progbits"),
                ".balign 16",
                concat!(".hidden ", stringify!($stub)),
                concat!(".global ", stringify!($stub)),
                concat!(stringify!($stub), ":"),
                "nop", "nop", "nop", "nop", "nop",
                "adrp x17, {target}",
                "ldr x17, [x17, #:lo12:{target}]",
                "br x17",
                ".popsection",
                target = sym TARGET,
            );

            pub(super) unsafe fn install(report: &mut Line<768>, enabled: bool) -> bool {
                let export = lookup($symbol);
                let found = if export == 0 { 0 } else {
                    let f: extern "C" fn() -> *const SiteShareV1 = core::mem::transmute(export);
                    f() as usize
                };
                // Election precedes the guard: the owner may already have patched it.
                match share::role(found, &API as *const _ as usize) {
                    Role::Guest(owner) => {
                        let api = &*(owner as *const SiteShareV1);
                        let ok = enabled && share::compatible(api, model::$site.offset)
                            && (api.register)($callback as *const () as usize);
                        STATE.store(2, Ordering::Release);
                        let _ = write!(report, " {}=guest/{}", model::$site.name, if ok { "registered" } else { "refused" });
                        ok
                    }
                    Role::Owner => {
                        let Some(address) = site_address(&model::$site) else {
                            STATE.store(2, Ordering::Release);
                            let _ = write!(report, " {}=foreign/skipped", model::$site.name);
                            return false;
                        };
                        let (entry, outcome) = chain::hook::inline_with_stub(
                            address, &model::$site.stock(), dispatch,
                            $stub as *const () as usize as u64, &TARGET,
                        );
                        STATE.store(if outcome.hooked() { 1 } else { 2 }, Ordering::Release);
                        let _ = write!(report, " {}={}/{} guests={}", model::$site.name, entry.name(), outcome.name(), GUESTS.len());
                        outcome.hooked()
                    }
                }
            }
        }
    };
}

shared_site!(begin, AI_PROCESS_BEGIN, ssbu_inline_site_30c770_v1,
    b"ssbu_inline_site_30c770_v1\0", ssbu_pilot_begin_stub, online_runtime::begin);
shared_site!(end, AI_PROCESS_END, ssbu_inline_site_3119d0_v1,
    b"ssbu_inline_site_3119d0_v1\0", ssbu_pilot_end_stub, online_runtime::end);

pub(super) unsafe fn install(report: &mut Line<768>, enabled: bool) -> bool {
    // Both are evaluated even on failure: keep an owner's site for its guests.
    begin::install(report, enabled) & end::install(report, enabled)
}
