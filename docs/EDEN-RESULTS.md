# Eden evidence — 2026-10-31

The merged-build runs (offline, the Deluxe rows, the card set, auto save and
the online mid-match switch) are at the end, under "Merged builds". The
sections before them are the first online runs, on branch `online`.

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

## Merged builds (main `41a497c` → `3267837`)

Eden on the Surface, display `:78`, a hard-linked copy of source profile `g`
(`/home/intpa/ssbu-cpu-pilot/eden/profiles/c-s`, +619 MB over `g`), Eden
commit `5a4ed955f1`. Each run: claim and release in the coordination file,
the profile restored from a baseline (five checks), `console.log` searched
(0 panic/abort/crash matches in every run). Diag NROs; ticks are the
19.2 MHz system tick from `events.log`. Raw folders:
`/home/intpa/ssbu-cpu-pilot/eden/runs/<label>/` (gitignored); per-run notes:
`/home/intpa/ssbu-cpu-pilot-work/research/eden-merged.md`.

| Run | Build (diag sha256) | What | Result |
| --- | --- | --- | --- |
| c11 | `41a497c` (`5d567cf1…`) | Training: on mid-fight, L+R+X, off, on, leave | (a)–(e) pass; (f) fail: armed ~150 s after leaving |
| c12 | `41a497c` | Deluxe rows | pass |
| c13 | `41a497c` | the card's 16 plugins + this one | pass |
| c14, c15 | `41a497c` | auto save, local Smash | fail: mode `0x02010000` refused |
| c16 | `ff01ab3` (`3a6f3ed4…`) | leave Training, re-enter | pass |
| c17 | `ff01ab3` | auto save | fail: `flow=timeout step=Cards` (splash needs A) |
| c18 | `0f4457c` (`76c018ac…`) | auto save, 3 splash presses | fail: presses during the intro |
| c19 | `3267837` (`b94d05b7…`) | auto save, up to 12 splash presses | pass (twice) |
| o14 | `ff01ab3` | online mid-match switching against vanilla | pass |

**c11 (Training, P1 vs a CPU).** Install: `result=installed
heap_allocations=0`. Mid-fight `p1=cpu level=9`: `arm level=9 … slot=0`,
growing `p1_inputs`, `hud set level=9 slot=0 panes=2`, P1 acting with no host
input and its tag reading "CPU Lv9". One L+R+X: `combo level=1`, release,
`arm level=1 rank=0`, tag "CPU Lv1". `p1=player`: `release restored=true`,
`hud restore`, the original tag back, the host stick moving P1. Armed again:
`arm level=9`. Leaving Training did not release (fixed in `ff01ab3`, c16).

**c12.** `menu=registered`; Deluxe's Plugins page shows the "CPU Pilot" group
(P1, Auto rematch, Auto save replay); ZR+Right on P1 in Training: `arm
level=9`, row "CPU", tag "CPU Lv9"; ZR+Left restored.

**c13.** Every site `stock/near`, `results_rematch=guest` (elite-rematch owns
`.147fb64`), the other plugins installed, the Training Modpack menu opens
(B + D-pad Up), `diag.txt` on/off works.

**c16.** `7262200328528 stale no_battle_frames armed_cleared=1`, 1.60 s
after the last battle frame; no armed lines through the menus; next Training:
`release restored=false old_rank=100`, `arm level=9 rank=100 old_rank=100
slot=0`, tag "CPU Lv9"; `p1=player`: `release restored=true`, `hud restore
panes=2`.

**c19 (local Smash, Final Destination, 1 stock / 1:00, P1 CPU level 9 vs a
CPU, `auto_save_replay=on`, nothing pressed at results).** All at port 0,
`mode=0x2010000`, after `epoch=begin` (7325600417180):
`press_a_splash n=1..6` at +2.95…+8.10 s, then `press_y` +8.15,
`select_yes` +8.47, `confirm` +8.66, `writing` +8.86, `close_success` +9.13,
`completed` +9.32 s; no press after `press_y`. New
`save_data/replay/replay20261031172348.bin` (41,016 B). The game rests on the
final results card. A second match in the same run gave the same sequence;
Eden's recorder caught "Replay saved. You can view it in the Vault." (frame
f0154) and `replay20261031172742.bin` (57,720 B). Both files were removed by
the restore.

**o14 (online, mid-match switching).** Surface: this plugin + detector + tile
cursor; Yoga: detector + tile cursor, no plugin (profile `c-y`). Quickplay
Solo, Sephiroth on both, 2:00 time rules (the four phases need about two
minutes). Real input on both consoles first (26 input calls each), then
mid-match `p1=cpu`: `online stage_start slot=1 stage=347 phase=1`, `online
stage=Ready phase=2 rng_restored=1 d0_restored=1`, `arm level=9 rank=100
old_rank=100 slot=1`; about 17 s later `p1=player`: `release restored=true`,
`hud restore panes=2`; then `p1=cpu`: `arm level=9 … slot=1` with no second
`stage_start`. Detectors: Surface `sync_end trigger=normal kind=1 frame=7521
local=1 desync_seen=0`, Yoga `sync_end … frame=7521 local=0 desync_seen=0`.


### Auto rematch (r01 fail, r02 pass)

Online pair as o14 (Surface: this plugin + detector + tile cursor; Yoga:
detector + tile cursor, no plugin), Quickplay 1 stock, Omega, items and FS
meter off, `p1=cpu level=9 auto_rematch=on`; the Yoga answered by hand.

- **r01, `3267837` (`b94d05b7…`):** both matches logged `flow=timeout
  step=Rematch` about 8.5 s after `epoch=begin`; the pop-up came about 15 s
  after the match, so it went unanswered. Detectors: `desync_seen=0` at the
  same frame on both (511, 988). Fixed in `582f5c2` (2400-frame wait).
- **r02, `582f5c2` (`fe5bf7f0…`), pass.** Match 1 (Yoga Yes): `rematch=press_a
  step=Rematch` at +22.34 s, `rematch=accepted step=Answer` 16 ms later; a new
  detector session started on both 39.3 s after match 1 and the rematch played
  (the pilot armed again, `arm level=9 … slot=1`). Match 2 (Yoga No): the
  plugin pressed Yes (`rematch=press_a`, `rematch=accepted` at +22.25 s); the
  Yoga returned to the character select, while the Surface stayed on the
  rematch "Waiting…" screen for the 3.5 min observed, with no error dialog. r01
  showed the same with the roles swapped (the vanilla Yoga, having said Yes,
  waited after the Surface had left), which points at Eden's LAN stand-in
  rather than the plugin; not proven. Detectors: match 1 `sync_end … frame=631
  … desync_seen=0` and match 2 `frame=1055 … desync_seen=0` on both consoles;
  no third session; 0 panic/abort/crash and 0 `2306-` on both hosts. Match 2's
  `epoch=begin` line still showed match 1's pop-up record; it did not cause a
  press (the step acts only on a pop-up parsed in its own epoch).

**Not yet run in Eden:** the smash-gamepad-overlay bridge (a branch of that
repo), the Training Modpack branch that uses this library, a replay A/B with
the detector.
