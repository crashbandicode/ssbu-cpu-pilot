# Online adapter compliance — 2026-10-31

Note, 2026-11-13: a snapshot of branch `online` before its commit. Since then
`origin` is Forgejo, tags `v0.1.0` to `v0.2.2` exist, the version is 0.2.4,
`online-pilot.gif` was removed (`3e60ff5`; the README GIF is now
`cpu-pilot-page.gif`), and the
operator-local `HANDOFF.md` and `research/` files named below are on the Yoga.

Completed **pre-commit** checklist, playbook `cd9972d`. Implementation and Eden
runs followed `69a08c0`; the refreshed playbook adds copy/cache/disk obligations.
`fail` includes an unverified requirement. This is experimental implementation
evidence, not a release acceptance. No unresolved reviewer rows remain.

Machine command (exit 1: no origin and pre-commit dirty tree):

```text
python3 -I /home/intpa/ssbu-mod-playbook/tools/check/compliance.py /home/intpa/ssbu-cpu-pilot-online --nro /home/intpa/ssbu-cpu-pilot-online/dist/release/libssbu_cpu_pilot.nro --out /home/intpa/ssbu-cpu-pilot-work/research/online-compliance.machine.md
```

Paths below: `research/` means `/home/intpa/ssbu-cpu-pilot-work/research/`;
`runs/` means `/home/intpa/ssbu-cpu-pilot/eden/runs/`. Surface receipts are
collected under each run's `surface/`; Yoga receipts are at its root.
Post-commit identity and clean-tree verification belong to the final
operator-local `research/online-compliance.md`, which supersedes only the
snapshot's Git rows. [Run evidence](EDEN-RESULTS.md), [state audit](ONLINE.md).

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | `Cargo.toml`; machine checker. |
| release profile opt-level "s", lto, codegen-units 1, panic abort | M | pass | `Cargo.toml`; machine checker. |
| skyline pinned to =0.6.0 | M | pass | `Cargo.toml`. |
| diag cargo feature declared | M | pass | `Cargo.toml`. |
| toolchain 1.98.1 | M | pass | `rust-toolchain.toml`. |
| runtime only under cfg(target_os = "switch") | M | pass | `src/lib.rs`; host-tested pure `model` and `core::online`. |
| supported version 13.0.5 in model | M | pass | `src/model.rs`; runtime rejects unsupported versions with `result=unsupported_version`. |
| every site guard is 64 bytes | M | pass | Eight declared guards; machine checker and `src/model.rs`; original five retain order. |
| counting allocator reports heap_allocations | M | pass | `src/runtime.rs::CountingAllocator`; both pilot install logs report zero. |
| no heap in hooks; fixed buffers, static writer stack, stack-built file paths | M | pass | New hooks use fixed arrays/static state; `Line`, `Path`, and `WORKER_STACK_MEMORY` in runtime; install allocation evidence above. |
| host tests in model (cargo test) | M | pass | `research/online-test.log`: 19 plugin and 27 core tests passed. |
| README carries the AI-assistance disclaimer | M | pass | `README.md`; machine checker. |
| README has a GIF, install path, build commands and a license line | M | pass | `README.md`; machine checker. |
| GIF online-pilot.gif: full screen, 1280 wide, under 10 s | M | pass | Machine checker: 1280 px, 8.0 s; uncropped provenance in `docs/media/README.md`. |
| dated evidence doc present | M | pass | `docs/EDEN-RESULTS.md`, `docs/ONLINE.md`. |
| numbers in the evidence come from tested scripts over the logs | C | pass | `tools/online_report.py`; six tests in `tools/test_online_report.py`; both commands next to evidence. |
| handoff file present | M | pass | Operator-local `HANDOFF.md`, deliberately excluded by the existing shared Git exclude. |
| remote is Forgejo, not GitHub; Git messages from a UTF-8 file | M | fail | No origin (`git remote -v` empty). No publication; UTF-8-file commit is the next checkpoint. |
| working tree clean | M | fail | Pre-commit `git status --short` contains the intended implementation/docs/tools/media; `git diff --check` passes. Final Git receipt is separate. |
| build identity | M | pass | Branch `online`, base `3cbdb44`; precise built artifact hashes in `EDEN-RESULTS.md` and the five build logs. No tag. |
| shipped NRO identity | M | pass | Release candidate only, not shipped: SHA-256 `73c2980ee55bb78dcfbcd2efa55157a3d249884fa799b97b7a887abd2631ee99`, 98304 B resident (96 KiB). |
| no diag strings in the shipped NRO | M | pass | Machine checker finds no `diag.txt`/`standin` markers in the release candidate. |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | pass | `research/online-{test,clippy,no-default,release,diag}.log`, zero warnings; no later Rust changes. |
| Card scan for the site (and vtable slot) over a dated card copy | M | pass | `research/online-card-scan.txt`, `card-20261031`; selected direct sites do not use a vtable slot. Training body-check conflict avoided; scan limitations in `ONLINE.md`. |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | fail | v1 election/cached state and separate far stubs implemented; live inline/far and both-load-order matrix unverified. Runs only show stock/near installs. |
| Hook after the prologue or the function called instead | M | pass | Manager begin +0x60 (`0x30c770`), common epilogue (`0x3119d0`); native think/rank/reset called; sites documented in `ONLINE.md`. |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | pass | Release event path is install-only; `diag` ring/drain compiled out. Settings persistence writes config, not per-frame diagnostics. |
| Product log files for a logging plugin | C | N/A: input adapter, not a logging product | No product data-log exception is used. |
| Online: read-only rules reviewed hook by hook | C | fail | Review in `ONLINE.md`; phase/caches and native AI state persist. Complete indirect-call effects, checked RNG coverage and simulation noninterference remain unproved. |
| Online: replay A/B with the desync detector identical | C | fail | Not performed; asymmetric tests do not substitute for it. |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | pass | Two-host Eden LAN Online: o02 both normal ends frame 1493; o03 frame 744. Full lines in `EDEN-RESULTS.md`; o03 explicitly input-unsampled. Not a physical Switch test. |
| Eden: claim written and released, also after a failed run | M | pass | `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md`, owner `f9d4e7a2-online`, o01/o02/o03 claim/release pairs at lines 609–619. |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | M | pass | Availability/preflight receipts and claims; Surface :76, Yoga :77; fresh oNN-s/oNN-y copies of g/b. |
| Eden: profile copy with hard-linked game content; only the copy changed | M | pass | Reused `gv.py prep` links read-only content; adapter's inode/link check and private profile paths; config/save/plugin restoration receipts. |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | fail | Game content linked and mutable inputs independently copied, but reused prep copies byte-identical small inputs too; refreshed all-copy rule was not met. |
| Disk: one build cache per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | fail | Task used private `target/`; no complete before-every-build/launch disk receipts. Final disk check and owned-cache removal recorded in `research/online-cleanup.md`; historical gap remains. |
| Eden: waits gated on a log line or screenshot check | M | pass | Bounded loading/Ready/end polls and checked menu shots in o01–o03; `/tmp/opencode/online-eden.py` and run screenshots. |
| Eden: failed precondition aborted and retried under the next label | M | pass | o01 seed/report failure stopped/restored; o02 fresh retry without seeds; details in `EDEN-RESULTS.md`. |
| Eden: shortest rules covering the test path | M | pass | One stock, items/FS off, Omega; Quickplay UI minimum 3:00 instead of 1:00. Rules shots and normal end frames retained. |
| Eden: profile restored, temp files removed, host released | M | pass | o02/o03 `RESTORE-s.json`/`RESTORE-y.json`: all checks true; o01 aborted/restored. Injected inputs/configs/plugins restored, owned processes stopped; evidence/private profiles retained. |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | pass | Both regenerated reports have empty console findings; restore receipts confirm run receipts retained. Deliberate shutdown has `interrupted`, exit 1. |
| Every local command under the memory and time guard | M | pass | Local command wrappers use systemd memory/swap bounds and timeout; PowerShell bodies. Cleanup wrapper parsing failure and bounded script retry recorded verbatim in cleanup manifest. |
| Each run's outputs kept under its label | M | pass | `runs/o01`, `runs/o02`, `runs/o03`; Surface regular-file evidence collected separately from Yoga. |
| Evidence doc states run labels, build, what was done, what the log showed | M | pass | `docs/EDEN-RESULTS.md`, full build hashes and both end-line pairs. |
| GIF caption gives date and build; clip shows start, action and result | M | pass | o03 shows starting positions, sword contact and opponent below ledge; first/middle/last raw+encoded frames reviewed. Caption claims opening attacks only, not match completion. o02 results-only clip rejected. |
| No operator quotes, no claims beyond the tests | M | pass | Docs distinguish detector evidence, unsampled input and unproved full-state safety. |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | pass | `HANDOFF.md` identifies base, pending commit, evidence, released resources, gaps and final commands. |
| Release: version bump, annotated tag, assets verified by anonymous download | C | not authorized | No release order; version remains 0.1.0. |
| Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | No tag or released artifact. |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | not authorized | No physical card installation order or write. |
| Nothing pushed, tagged or written to the card without the operator's order | M | pass | Local implementation/testing only; no publishing, tag or MTP operation. |
| Model/effort gate and watcher per the routing authority | M | fail | Harness reports `nvidia-b/openai/openai/gpt-6-astra`; `MY_MODEL.md` offers no current-thread OpenCode live effort/watcher method. Unverified, not inferred from another session. |
