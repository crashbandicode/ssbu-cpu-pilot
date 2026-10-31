# Results automation evidence — 2026-10-31

Implementation on `results`, based on `81a07fa`. No Eden, Switch, remote changes,
pushes, or delegation were performed. This is implemented code and static RE,
not a native acceptance result. Exact build identities and final checks are in
`/home/intpa/ssbu-cpu-pilot-work/research/results-impl.md` and
`results-compliance.md` in the same research directory.

## Architecture and sites

`core/src/results.rs` owns the fixed-size policy. `src/results.rs` observes
native objects and overlays only one of ten UI input records. `runtime.rs`
forwards the completed-array callback and installs the adapter. No native game
functions are called by the adapter, popup fields are never written, and no
heap allocation is introduced in a hook. Event formatting and enqueueing are
compiled only with `diag`; the release path adds no per-frame file writes.

All offsets below are main-relative, SSBU 13.0.5. Every row has its own 64-byte
`Site` in `src/model.rs`, extracted from the supplied `main-text.elf` with
`tools/site_bytes.py`. Read dependencies require all 16 instructions unchanged.
Hooks use the existing guarded stock/direct-branch/far-chain recognition.

| Offset | Site / purpose | Card scan / sharing |
| --- | --- | --- |
| `376309c` | RESULTS_INPUT: builder exit, x22 minus 1f0 is array | No reference; allow_chain=false |
| `1d59560` | RESULTS_CARDS: post-prologue x19 snapshot | No reference; allow_chain=false |
| `147fb64` | RESULTS_REMATCH: x20 record, x19 popup wrapper | elite_rematch; inline sharing v1 |
| `150ea38` | RESULTS_LIFETIME: results destruction/null | No reference; read only |
| `1d59dc4` | RESULTS_PORT: fighter-to-port table | No reference; read only |
| `1d5b5a8` | RESULTS_Y: modal gate, record stride, Y | No reference; read only |
| `1d5b650` | RESULTS_SAVE_OPEN: descriptor/status 1 | No reference; read only |
| `1d5a590` | RESULTS_SAVE_ACCEPT: button 2/status 2 | No reference; read only |
| `1d5a840` | RESULTS_SAVE_SUCCESS: success/status 3 | No reference; read only |
| `32f22c4` | RESULTS_POPUP_FIELDS: default/cancel/input mode | No reference; read only |
| `376db18` | RESULTS_CURSOR: selected button at +250 | No reference; read only |
| `32f1630` | RESULTS_POPUP_DECISION: native decision/result | No reference; read only |
| `1671650` | RESULTS_MODE: mode-service initialization guard | Five references; read only |

Card: `/home/intpa/gsp-visualizer-work/card-20261031`; raw results:
`/home/intpa/ssbu-cpu-pilot-work/research/results-card-scan.txt`.
The five mode references are elite_stages_counterpick, smash_gamepad_overlay,
smash_replay_to_mp4, ssbu_online_deluxe_rewrite, and training_modpack.
A constant reference is not proof of a hook. The builder entry `3762ec0` was
also scanned and had no references. Neither `143e2a0` nor `150e9d0` is hooked.

Rematch looks up `ssbu_inline_site_147fb64_v1` before inspecting the site. It
registers a read-only guest when another owner exists, otherwise installs an
owner and dispatches guests. A dedicated far-chain stub prevents overwriting
the CPU pilot's existing FIM stub. Failed dependencies disable input while
leaving an installed shared owner available to guests. Unknown instructions
are refused. Both load orders and stand-ins still require Eden tests.

## Signals and bounded policy

An epoch is `[[main+52c35d0]+8]`; teardown at `150ea38` destroys and nulls its
wrapper. A sampled zero separates reused addresses. Cards are borrowed only
at `1d59560`, never dereferenced later; cached readiness expires after one UI
callback. Offline P1 uses fighter-entry 0 through `[main+52c50f0]+298`, matching
the mapping code at `1d59dc4`; it is sampled only with the cards, so before them
(the winner splash) there is no offline port. Online uses only first local UI port 0.

Each frame means one completed UI-builder invocation; seconds below assume
60 Hz and are not measured. Settings bind per epoch; disabling, scene/port
changes, missing/incorrect signals, failure status, or deadline ends that
epoch without retry. A late signal cannot restart it.

0. Local winner splash (Eden c17, c14): offline results open on a splash that
   never advances alone (c17 sat 28 s with no cards sample; in c14 the cards
   and the Y prompt appeared only after manual A presses). While no cards
   sample of this epoch exists, press A on the splash: the first 90 frames
   after epoch begin, then another every 60 frames (A pulse as below, 17
   frames long), at most 3 per epoch. Each is logged as
   `save=press_a_splash n=<k>`. The moment any cards sample for the epoch
   arrives, ready or not (including one whose port could not be read), no A is
   ever pressed again in that epoch, and an A still held is released at once
   on its own port. Never on Training, replay playback, any unsupported scene,
   online, or with `auto_save_replay` off. **Assumption Eden must confirm:**
   no fighter-entry mapping exists before the cards, so the splash A goes to
   UI port 0 (P1's controller) and to no other port; the later Y/Right/A
   still use the cards' port (`cards_port`), which may differ.
1. Local cards: wait under 1200 frames (20 s) for phase/pending 2, finished=0,
   save_offered=1, popup idle, modal mask <=1, and save status 0. Then Y once.
   The 1200-frame deadline is counted from the last splash A (or from epoch
   begin if none was pressed), so the splash presses never shorten it: the
   worst case from begin is 90+60+60+1200 frames. Once any cards sample exists
   the flow only waits for these conditions; it never presses A for the splash.
2. Wait under 180 frames (3 s) for status 1 and ready Save popup record
   `58ffff145ea6f7be`, default=1, cancel=0, input mode=2, cursor=1.
3. Right bit21 plus stick +X for at most 11 frames, stopping when cursor=2;
   ten neutral frames, then A only if cursor remains 2. This step also has
   a 180-frame deadline. A/Y pulses have one edge, five further holds, one
   release and ten neutral frames. Lost readiness cancels remaining holds.
4. Status 2 must follow this A within 180 frames. Wait under 900 frames (15 s)
   for status 3 and ready success record `58ffff1e1eed5764`; press A once.
5. Status 5 within 480 frames (8 s) confirms the UI chain completed. Status 4
   is failure. Completion requires the entire owned 0->1->2->3->5 chain;
   no replay-commit hook is needed, and byte-level replay persistence is not
   independently observed by this plugin.
6. Online saving logs `save=skipped_online`. Ordinary rematch requires the
   shared parse to capture record `58ffff1cb2b11288` in this epoch. Within
   480 frames, require phase/pending 3, age >=36, result=0, not opening or
   decided, default/cursor=2, cancel=0, input mode=2. Press A once. Within
   360 frames require native phase3/pending6/decided/result2. Forced phase5,
   closing phase6/7, and retained results are never accepted as success.

Popup fields come from the supplied Training Modpack `automation_helper`
(native/popup/adapter/pulse/flow); cursor binding was additionally traced:
`32f20bc` -> `3771a80` -> `376d990`, with current ID stored at `376db18`.
The default is passed without decrement. Native button2 acceptance and success
status are visible at `1d5a594`, `1d5a5c8`, `1d5a840`, and `1d5a904`.
The native UI decision is copied at `32f1918`. Save status is
`[[main+52c35c8]+8]+10`; popup wrapper is `[main+532f820]`.

## Validation boundary and diagnostic acceptance

Host tests cover complete saves, missing/late signals, timeout terminality,
no blind confirmation, failure/unowned status, epoch reuse, settings/port
changes, single-port overlays, rematch normal/forced decisions, and online
save priority. These prove policy behavior, not that the native callback
observes each transient signal.

Mode reads follow the supplied game_mode guard. The local `0101xxxx` allowlist
is inferred from the replay plugin's `guards.rs` live-mode fixture; online is
`07xxxxxx`. Their values during results still need runtime qualification.
Other modes, including training, playback and local wireless, do not save.
The splash A (core tests `splash_a_*`, `no_splash_a_*`, `full_save_sequence_still_*`)
is host-tested only: whether UI port 0 reaches the splash, whether 1 to 3 As
advance it, and whether a cards sample is taken on the first cards frame are
all unverified. Online UI port acceptance, local P1 mapping, effective Right input, one
callback per frame, transient decision visibility, timing, storage failure,
actual replay creation, and rematch countdown completion remain unverified.
Offline rematch is omitted because CSS/stage/start signals were not proven.

Diagnostics emit transition lines with this literal format (numbers vary):
`results epoch=0x... port=N TOKEN step=STAGE status=N popup=0x... phase=N cursor=N mode=0x...`.
Offline, zero to twelve `save=press_a_splash n=1..12` lines (port=0) may precede the
save tokens, and none follows `save=press_y`. For a save, require in order `save=press_y`, `save=select_yes`, `save=confirm`,
`save=writing`, `save=close_success`, `save=completed` with final status=5.
Online require `save=skipped_online` when enabled, then `rematch=press_a`
and `rematch=accepted`. Install must show `results_cards=stock/near` (or `/far`),
`results_input=stock/near` (or `/far`), and `rematch_ready=1`; shared-site role is
`results_rematch=guest` or `results_rematch=owner:stock/near` (or a recognized chain/branch and near/far).
For failure tests, require `flow=timeout step=...`, `save=failed_status_4`,
`save=unexpected_status`, `flow=lost_popup`, or `flow=wrong_answer`, and no
subsequent presses in that epoch. Epoch logs also expose mode and selected port.
No gameplay log, GIF, desync result, or hardware claim is supplied.
