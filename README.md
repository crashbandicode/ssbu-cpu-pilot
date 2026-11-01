# ssbu-cpu-pilot

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

A Skyline plugin for Super Smash Bros. Ultimate 13.0.5. It hands P1 to the
game's own CPU and back, at any time before, during or after a match, so a
match can go on when you have to step away, or a mod can be tested against a
CPU on both sides. With Online Deluxe installed, it adds a "CPU Pilot" group
to Deluxe's Plugins page:

| Row | Choices | What it does |
| --- | --- | --- |
| P1 | Player / CPU | The CPU drives P1's pad, and P1's HUD tag reads "CPU Lv<n>". While it does, **L+R+X** on P1's controller cycles the CPU level 1–9 (it starts at 3 and is saved). |
| Auto rematch | Off / On | Online, answers Yes on the ordinary Quickplay/Elite "Rematch?" pop-up. |
| Auto save replay | Off / On | Offline, saves the replay from the results screen through the game's own Save Replay dialog. |

The CPU's decisions come from P1's own native AI object, run on a private
copy of the AI random stream and written to P1's mapped input; the shared
random streams and P1's controller fields are restored after every decision.
P1 always starts as the player after a boot. Offline battles need a native
CPU present (Training, local Smash). Online, in a 1v1 Quickplay match, the
CPU drives the local player: the plugin starts the AI service's stage
analysis on this console under a private copy of the AI random stream, so
the opponent's console only ever receives ordinary inputs. Asymmetric Eden
matches (plugin console against a vanilla one), including switching on and
off mid-match, passed the desync detector's end check; a replay A/B and
broader simulation-state checks are still to be tested
([docs/ONLINE.md](docs/ONLINE.md), [docs/EDEN-RESULTS.md](docs/EDEN-RESULTS.md)).

Results automation presses buttons for P1's port only, each step gated on
the game's own state, and stops for that results screen when a step does not
happen in time ([docs/RESULTS.md](docs/RESULTS.md)).

*Status (2026-11-01): v0.1.0, experimental. Tested in Eden with the diag
build of the same code: P1 as CPU in Training (on and off mid-fight, L+R+X, the HUD tag), the
Deluxe rows, the card's other 16 plugins alongside, auto save replay in a
local match, and the CPU driving the local player online, switched on and
off mid-match, against a vanilla console with the desync detector on both
(`desync_seen=0` at the same frame), and auto rematch answering Yes on the
online pop-up, with the rematch played and both sessions checked by the
detector. The v0.1.0 release build also ran in Eden with the full card set
it was installed with (16 plugins). Evidence:
[docs/EDEN-RESULTS.md](docs/EDEN-RESULTS.md). Not yet run on a Switch.*

![Online Deluxe's Plugins page: P1 switched from Player to CPU, then the CPU fights on its own](docs/media/menu-takeover.gif)

*Online Deluxe's Plugins page (top right): ZR+Right turns P1 from Player to
CPU, the tag becomes "CPU Lv9" (level set with L+R+X beforehand), the menu is
hidden and the CPU fights on its own. Eden `m01` (Windows), 2026-11-01, v0.1.0
release NRO `24184fe3`; 10 fps.*

## Library

`core/` is the `ssbu-cpu-pilot-core` crate: settings and their INI form, the
L+R+X level cycle, the level-to-rank ladder, the AI-to-pad encoding, the
per-frame pilot session, online admission, the results policy and the HUD
label, plus (feature `native`) the raw AI calls. It has no Skyline
dependency, so another plugin, such as the Training Modpack, can use it with
its own hooks and Skyline version ([docs/LIBRARY.md](docs/LIBRARY.md)).

## Install

Copy `libssbu_cpu_pilot.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. Its data
folder is `sd:/ultimate/ssbu-cpu-pilot/` (`events.log`, `config.ini`).
Online Deluxe 0.5.3 or later shows the rows; without it the plugin has no
menu. If Deluxe's Plugins page is hidden (`hide_pages=plugins` in its
`config.ini`), open Window Options with L+R+Down and set PluginsPage to Shown
with ZR+Right.

[smash-gamepad-overlay](https://git.syntaxbyexample.com/intpa/smash-gamepad-overlay)
v0.2.0 or later shows the CPU's inputs while it drives P1. Some Training
Modpack builds carry their own P1-as-CPU option; leave it off while this
plugin drives P1 (the modpack branch `cpu-pilot-lib` defers to this plugin
on its own).

## Build

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --no-default-features
pwsh -File tools/build.ps1 -Kind release   # cargo skyline build --release -> dist/release
pwsh -File tools/build.ps1 -Kind diag      # --features diag, the Eden build -> dist/diag
```

Toolchain 1.98.1 (`rust-toolchain.toml`), `skyline = "=0.6.0"`.

## License

MIT, see [LICENSE](LICENSE).
