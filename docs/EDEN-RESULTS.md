# Online Eden evidence — 2026-10-31

Two asymmetric Quickplay matches reached normal matching-frame detector ends.
This is scoped RNG-detector evidence, not complete simulation-state safety.
The state/RE limits are in [ONLINE.md](ONLINE.md).

## Build and setup

- Branch `online`, implementation based on `3cbdb44`; tests followed playbook
  `69a08c0`, final checklist review follows `cd9972d`.
- Diag NRO SHA-256: `e63b3cc3c648650d46e9777653fc8812e4d4c65e98c8e7d80d6c8123dde00f00`.
- Release NRO SHA-256: `73c2980ee55bb78dcfbcd2efa55157a3d249884fa799b97b7a887abd2631ee99`.
- Surface :76: pilot diag, detector and tile cursor. Yoga :77: detector and
  tile cursor, no pilot. Both Eden AppImages: `7b4d18629c2964380d794ac8b3d119c222309b68d91e8e706b4755caa603b2ae`.
- Fresh `oNN-s`/`oNN-y` private profiles, game content hard-linked, small
  config/save snapshots; source profiles g/b unchanged. One instance per host,
  preflight memory passed. Both selected Sephiroth using tile cursor.
- Shortest native Quickplay stock rules: 1 stock, 3:00 (the UI minimum),
  Omega form, items/FS meter off. Screenshots retained under each run's label.
- Pilot enabled at level 9 before matchmaking; no Surface gameplay input from
  the driver. Yoga used crouch/shield keepalive. No forced gameplay state.

## Runs

`o01` failed its boot precondition: rating seed `43:1500` required an existing
fighter report for PID 1. Both instances stopped, profiles restored, claim
released; `o02` retried without seeds.

In `o02`, installation logged all four inline sites `stock/near`,
`result=installed heap_allocations=0`. The pilot resolved local slot 1,
initialized native analysis phase 1→2 on stage 1, logged both RNG/d0
restorations, armed level 9, and emitted nonzero mapped input. It stayed armed
through normal end. Exact detector lines:

```text
Surface: 7142643845342 sync_end trigger=normal kind=1 frame=1493 local=1 desync_seen=0 already_ending=0 caller=0x1569718
Yoga:    7110275712525 sync_end trigger=normal kind=1 frame=1493 local=0 desync_seen=0 already_ending=0 caller=0x1569718
```

`o02`'s recorder started too late: visual review showed only results. That clip
was rejected as a gameplay demo. `o03` repeated the asymmetric setup, triggering
recording from the pilot's Ready/arm log. Native analysis initialized on stage
300; slot 1 armed level 9 and remained armed through end:

```text
Surface: 7159938282893 sync_end trigger=normal kind=1 frame=744 local=1 desync_seen=0 already_ending=0 caller=0x1569718
Yoga:    7127573676753 sync_end trigger=normal kind=1 frame=744 local=0 desync_seen=0 already_ending=0 caller=0x1569718
```

This shorter match ended before the next periodic mapped-input sample. Its
report explicitly says `detector_pass_input_unsampled`; the GIF supplies the
visual gameplay evidence. Raw and encoded first/middle/last frames were viewed:
opening positions, sword contact, then the opponent below the ledge. The clip
shows the opening attacks, not the entire match or its final result.

## Reproduce the report

The tested parser checks plugin asymmetry, normal online ends, distinct local
peers, matching frames, phase/restoration/arm ordering, input samples and no
early release. It searches both complete console logs for panic/abort/crash;
neither run had a match. It does not infer full-state synchronization.

```text
python3 -I tools/test_online_report.py
python3 -I tools/online_report.py /home/intpa/ssbu-cpu-pilot/eden/runs/o02
python3 -I tools/online_report.py /home/intpa/ssbu-cpu-pilot/eden/runs/o03 --allow-unsampled-input
```

Six parser tests pass, including missing/truncated evidence, desync, unequal
frames, duplicate sessions, premature release and unsampled short matches.
All five Rust/build gates passed: 19 plugin + 27 core tests, clippy with
warnings denied, no-default-features, release and separate diag build.
Receipts: `/home/intpa/ssbu-cpu-pilot-work/research/online-*.log`.

## Evidence and restoration

Run roots: `/home/intpa/ssbu-cpu-pilot/eden/runs/{o01,o02,o03}/`. Collected
Surface files are under `surface/` in the Yoga copy; Yoga receipts/logs are at
the root. Keep PREP/LAUNCH, `run.json`, `memory.peak`, console/plugin logs,
screenshots, recorder metadata and RESTORE receipts. All owned processes,
displays and locks stopped. Config/save snapshots and original plugin/output
sets hash-verified restored on both hosts; claims released after each run.
The bounded launchers report `interrupted`, exit 1 from deliberate cleanup,
not an application crash.

Unverified: replay A/B, live sharing collision/load-order matrix, complete
indirect-call side effects and noninterference of persistent phase/caches.
No origin is configured; the live effort/watcher gate is unavailable in this
OpenCode session. No push, tag, release or physical card write was ordered or
performed. The [completed checklist](ONLINE-COMPLIANCE.md) also records the
copy/cache/disk evidence gaps. The final Git-status receipt is operator-local
`/home/intpa/ssbu-cpu-pilot-work/research/online-compliance.md`.
