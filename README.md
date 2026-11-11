# ssbu-cpu-pilot

A Skyline plugin for Super Smash Bros. Ultimate 13.0.5 that hands P1 to the
game's own CPU and back, at any time: step away mid-match, or let a CPU play
your side while you test a mod. It can also answer the online "Rematch?"
pop-up and save replays for you, offline and online.

![Online Deluxe's CPU Pilot page: P1 switched from Player to CPU (L5), L+R+X steps it to L6 and L7, then the CPU fights on its own](docs/media/cpu-pilot-page.gif)

*Eden, 2026-11-01, Training: L+R+Down to the CPU Pilot page, ZR+Right sets P1
to `CPU (L5)`, L+R+X twice steps it to `CPU (L7)` (HUD tag `CPU Lv7`), Window
Options, then the window closes and the CPU fights with the controller idle.
Online Deluxe 0.10.1, ssbu-cpu-pilot 0.2.1, 10 fps.*

## Use

Press **L+R+Down** until Online Deluxe shows the **CPU Pilot** page, then
**ZR+Right/Left** on a row:

![The CPU Pilot page: P1 CPU (L6), Auto rematch Off, Auto save replay Off](docs/media/cpu-pilot-page-crop.png)

| Row | |
| --- | --- |
| P1 | `Player`, or `CPU (L1)` … `CPU (L9)`: the CPU plays P1 in matches and Training, with the HUD tag `CPU Lv<n>`, and (0.2.3) in the Quickplay waiting-room practice, which shows no tag; menus stay yours. When a match is found the CPU keeps P1 in the match. ZR+Right/Left switches Player ↔ CPU in one press and keeps the level. **L+R+X** steps the level 1 → 9 → 1 (starts at 3, saved): on this row (Deluxe 0.10.1 or later), or on P1's controller during a match. P1 is the player again after a reboot. |
| Auto rematch | Answers Yes to the online "Rematch?" pop-up. |
| Auto save replay | Saves the replay at the results screen, offline and online (then answers "Rematch?" if Auto rematch is on). |

Both Auto rows work whether P1 is you or the CPU. If the page is not in the
cycle, show it on Deluxe's last page, Window Options (Deluxe 0.10.0 or later):

![Window Options: the CPU Pilot row set to Shown](docs/media/window-options-crop.png)

## Install

Copy `libssbu_cpu_pilot.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/` (data:
`sd:/ultimate/ssbu-cpu-pilot/`). The page needs
[Online Deluxe](https://git.syntaxbyexample.com/intpa/ssbu-online-deluxe-rewrite)
0.5.3 or later. If your Training Modpack build has its own P1-as-CPU option,
leave it off while this plugin drives P1.

## More

- Online play and what was checked against a vanilla console: [docs/ONLINE.md](docs/ONLINE.md)
- Rematch and replay automation: [docs/RESULTS.md](docs/RESULTS.md)
- The `ssbu-cpu-pilot-core` library, for plugins such as the Training Modpack: [docs/LIBRARY.md](docs/LIBRARY.md)
- [smash-gamepad-overlay](https://git.syntaxbyexample.com/intpa/smash-gamepad-overlay) 0.3.0 shows the CPU's inputs on P1's own layout
- Test evidence (Eden, and a Switch session on 2026-11-02): [docs/EDEN-RESULTS.md](docs/EDEN-RESULTS.md)

## Build

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --no-default-features
pwsh -File tools/build.ps1 -Kind release   # cargo skyline build --release -> dist/release
pwsh -File tools/build.ps1 -Kind diag      # Eden build -> dist/diag
```

License: MIT, see [LICENSE](LICENSE).

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited above.
