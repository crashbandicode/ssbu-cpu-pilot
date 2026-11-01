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

### Online auto save replay (s02 fail, s03 and s04 pass), 2026-11-01

Online pair as r02 (Surface `:78`: this plugin's diag build + detector `95e7def9…` +
tile cursor `b8313ca2…`, profile `c-s`; Yoga `:77`: detector + tile cursor, no plugin,
profile `c-y`), rating runtime, Quickplay Solo, Sephiroth on both, 1 stock, Omega,
items and FS meter off. Branch `online-save`. Timelines:
`python3 -I tools/results_timeline.py runs/<label>/surface/outputs/sdmc/ultimate/ssbu-cpu-pilot/events.log`.

- **s02, diag `eb34d9a2…` (`be1838f` + cards lines), fail.** P1 the player,
  Auto save on, Auto rematch off: `epoch=begin`, then `flow=timeout step=Cards` at
  +21.02 s and no `cards=sample` line, so the cards hook never ran. Frames: winner
  splash with a countdown (14 → 0), then "Rematch?" (first seen at about +22.6 s),
  then the character select, with no cards screen and no press. Replays 5 → 5. The second match never
  matched (the Surface sat on the character select after the repick), so the run
  was aborted and released. Detector: `sync_end … desync_seen=0` on both.
- **s03 and s04, diag `4a44ebf4…` (online splash A), pass.** One character-select
  start, then the rematch:
  - Match 1, P1 CPU, both rows on, Yoga Yes. `save=press_a_splash n=1..6` on port 0
    from +1.9 s, then `cards=sample n=1 route=1 offered=1 … phase=2 pending=2 …
    ready=1 … ports=-1,0` at +7.2 s (s03) / +7.1 s (s04). Then `press_y`,
    `select_yes`, `confirm`, `writing`, `close_success` and `save=completed …
    status=5` at +8.31 / +8.21 s, then `rematch=press_a` at +22.21 / +22.10 s and
    `rematch=accepted` 0.02 s later. The rematch played on both consoles.
  - Match 2 (the rematch), P1 the player with keepalive input, Auto rematch off,
    Yoga No: the same splash and save tokens, `save=completed … status=5` at
    +7.96 / +8.05 s, no rematch line, and no third session within 60 s.
  - Every epoch: `save=ok` from the timeline script. New replays on the Surface:
    s03 `replay202611185320.bin` (34,944 B) and `replay202611185420.bin`
    (30,224 B); s04 `replay20261119145.bin` (33,504 B) and
    `replay20261119250.bin` (33,464 B). Each run's count went 5 → 6 → 7; the
    copies are in `runs/<label>/surface/save-after/`. In s03 the Surface keepalive
    did not stop at its `sync_end` (a run-script error) and kept tapping through
    match 2's results. s04 reran both matches with that fixed: `{"steps": 35,
    "errors": 0, "reason": "stop"}`.
  - Frames (s03, `burst/*-s-1.png`): after the A, the online result cards
    ("Gold Earned 60", a 15 s countdown), "Save a replay of this battle?" with Yes
    selected, then the cards with the GSP change, and "Rematch?" when the cards'
    countdown ended.
  - Detectors: s03 `sync_end trigger=normal kind=1 frame=1514 … desync_seen=0`
    and `frame=1339 … desync_seen=0`; s04 `frame=1475` and `frame=1497`, all on
    both consoles at the same frame.

All three runs: claim and release in the coordination file; both profiles
restored (8/8 checks); 0 panic/abort/crash and 0 `2306-` in both `console.log`s;
`run.json` and `memory.peak` kept.

### Training Modpack branch using this library (k01, k02)

Fork branch `cpu-pilot-lib` at `c840b20` (`ssbu-cpu-pilot-core` rev `0162eed`,
`--features enhancements-alpha`, NRO sha256 `18db25e2…596e`), on the Surface
with `libnro_hook.nro`, `libparam_hook.nro` and the tile cursor; the fork's
`[CpuPilot]` lines are in `sd:/ultimate/TrainingModpack/practice-diagnostics.log`.

- **k01, the fork alone, pass.** `[CpuPilot] offline_probe_ready` at boot; the
  modpack menu's Misc tab, "P1 CPU Control" On and level 9: `menu_control
  event=armed level=9 rank=100 … slot: 0 …`; P1 acted with no host input (the
  opponent from 67.4% to 86.3% in 5 s); Off: `menu_control event=menu_off …
  rank_restore_attempted=Some(true)`, and the host stick reached P1 again.
- **k02, the fork with this plugin (`582f5c2`), pass.** The fork logged
  `[CpuPilot] standalone_pilot_present ssbu_cpu_pilot_state_v1=0x871398450;
  fork pilot hooks not installed` and no `offline_probe_ready`; this plugin:
  `result=installed heap_allocations=0 menu=host_missing`. The fork's menu On
  drove nothing (P1 idle, no fork line in 10 s); `p1=cpu` here: `arm level=9
  rank=100 old_rank=100 slot=0`, "CPU Lv9" on the tag, P1 fighting; `p1=player`:
  `release restored=true`, the host stick moving P1.

Both runs: 0 panic/abort/crash in `console.log`.

### smash-gamepad-overlay reading this plugin's state (g01–g04)

The overlay's bridge reads `ssbu_cpu_pilot_state_v1` and, while `armed_level`
is non-zero, draws the CPU's last input instead of the physical pad. Surface
`:78`, Training, this plugin's diag `582f5c2` (`fe5bf7f0…`), ARCropolis, the
overlay's `layout.arc` and default `config.json`, the host pad idle while the
CPU drove P1.

- **g01, overlay `232006b`, its own draw hook, pass:** host input lit ZR, A
  and the stick; `p1=cpu`: `arm level=9 … slot=0`, "CPU Lv9", the overlay's
  stick and A/B followed the CPU over a 29-frame burst; `p1=player`:
  `release restored=true`, the overlay followed the host pad again.
- **g02, the same with the Training Modpack (its non-draw path), pass**, with
  one defect in both: X lit in every CPU frame, because the pilot's input
  carries the tap-jump flag at rest and the bridge mapped it to X.
- **g03, the overlay's text fallback (no layout), partial:** the text line
  follows the CPU, but that fallback writes P1's `set_txt_00`, the pane that
  carries this plugin's "CPU Lv9" tag, so the tag does not show in that mode.
- **g04, overlay `e4bd87d` (tap-jump flag no longer mapped), pass:** X lit in
  2 of 29 CPU frames, when the CPU jumped.

All four: 0 panic/abort/crash in `console.log`. The overlay's `v0.2.0`
(`a00d41b`) differs from `e4bd87d` only in README, version and `Cargo.lock`.

### v0.1.0 release on the 2026-11-01 card set (v03) and the README GIF (m01)

2026-11-01, the Ally (Windows 11, AMD Radeon iGPU), Eden-GIF `b3e672c4`
(`5a4ed955f1`), `eden.exe` sha256 `d455ae54…936a`, an isolated portable run
folder per label (the shared Eden user folder unchanged), the v0.1.0 release
NRO `24184fe3…`. Release builds write only their install line, so the checks
below come from game frames (Eden's recorder; `ui.screenshot` returns black for
the Vulkan surface on that host).

- **v03, pass:** the 16 NROs installed on the Switch (hashes equal to the
  install receipt), the card's Deluxe `config.ini`, plus the tile cursor. Every
  plugin installed: this plugin `plugin=ssbu-cpu-pilot/0.1.0 level=3 …
  results_rematch=guest … result=installed heap_allocations=0 menu=registered
  menu_allocations=38`, elite-rematch, quiet-elite-switch and gsp-visualizer
  `result=installed heap_allocations=0`, the desync detector, Deluxe
  `build=0.9.0-…`, the overlay `c38-b11-a00d41b`, ARCropolis 4.1.1, the
  Training Modpack. The card's Deluxe INI hides the Plugins page
  (`hide_pages=plugins,own_pages,performance`); Window Options, PluginsPage →
  Shown, then the Plugins page lists `CPU Pilot`. P1 → CPU: row `CPU`, tag
  `CPU Lv3` (the default level), P1 fought with no host input (Lucina 0 →
  20.8%), the overlay (v0.2.0) lit the CPU's buttons. P1 → Player: tag back,
  the host stick moved P1. L+R+X: `CPU Lv4` after one press, `CPU Lv9` after
  six; the level stayed over Player → CPU. Quit Training clean. `eden_log.txt`
  15,159 lines, no panic/abort/crash event (one match is the setting name
  `cpuopt_ignore_memory_aborts`); memory peak 6,208,217,088 B.
- **m01, the README GIF:** this plugin, Deluxe and imgui_smash from the card,
  the tile cursor. From the recorder's start: L+R+Down at 0.5 s and 1.3 s (the
  Plugins page, P1 `Player`), ZR+Right at 2.5 s (P1 `CPU`, tag `CPU Lv9` from
  about 2.6 s; the level was set with L+R+X before recording), L+R+Down at 4.3 s
  and 4.9 s (menu hidden), then P1 fought alone (Jigglypuff 0 → 19.2%). The
  Deluxe chord also reaches the game, so P1 dodges while the menu opens. Eden's
  own GIF (take 2), 1280×720, 95 frames at 10 fps with none dropped, 13.71 MB,
  sha256 `70f7bb4d…894f`; a 960-wide take of the same schedule was 8.03 MB but
  the menu text was too small to read.

### 0.2.0: the CPU Pilot page with Online Deluxe 0.10.0 (w01, p01, m02, m03)

2026-11-01, Online Deluxe 0.10.0 `a070b1a7…` (one Window Options row per plugin
page). Surface :78 for w01, the Ally (as above) for the rest. Evidence in
`/home/intpa/ssbu-cpu-pilot/eden/runs/{w01,p01,m02,m03}/`.

- **w01, pass (release `0830f3fc…`, `17f7033`):** the card set with both new
  builds. The card's Deluxe INI (`hide_pages=plugins,own_pages,performance`)
  showed one Hidden row per page: `Stages + Counterpick` (the card's
  elite_stages_counterpick, id `stages_counterpick`) and `CPU Pilot`. CPU Pilot
  → Shown: Deluxe wrote `hide_pages=plugins,performance` and
  `hide_plugin_pages=stages_counterpick`; the cycle became Settings → CPU Pilot
  → Window Options. ZR+Right: tag `CPU Lv3`, Ness 10.0 → 138.4% with the host
  idle; ZR+Left: the player again.
- **p01, pass (diag `6e381a59…`, `17f7033`, and a test plugin with two
  pages):** each page shown and hidden on its own, hidden pages skipped by the
  cycle, Up/Down wrapping through the rows, the setting kept after a restart.
  Mid-match, P1 handed over and back from the page (`arm level=3 … slot=0`,
  `release restored=true`). Auto save with P1 the player ran through
  `save=completed step=Close status=5` and a new replay file. With P1 set to CPU
  the host drove the menus (tile cursor `step screen=css`); no `arm` outside
  battle.
- **m02, the README media (release `49efcc06…`, `be1838f`):** see
  [media/README.md](media/README.md).
- **m03, pass (release `e996e923…`, the 0.2.0 code):** the card's 16 NROs as
  installed at 13:57Z (Deluxe 0.10.0) with this build in place of 0.1.0, the
  tile cursor, and the card's Deluxe INI with the two lines above. Install
  `plugin=ssbu-cpu-pilot/0.2.0 … result=installed heap_allocations=0
  menu=registered`, Deluxe `build=0.10.0-89e9b473db22-1793530551`. In Training:
  Settings → CPU Pilot (`P1 Player`); ZR+Right: `P1 CPU`, tag `CPU Lv9`;
  Window Options listed `CPU Pilot Shown` and `Stages + Count…` Hidden (the
  long label is clipped by the row's label column); P1 fought alone (Ness 0 →
  5.6%). `eden_log.txt` 5,464 lines, no panic/abort/crash event (the one match
  is the setting name).

**Not yet run in Eden:** a replay A/B with the detector.
