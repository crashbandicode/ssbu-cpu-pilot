# ssbu-cpu-pilot

A Super Smash Bros. Ultimate 13.0.5 Skyline plugin that lets the native CPU drive P1, with settings for offline replay saving and ordinary Quickplay/Elite rematch prompts. Results automation is implemented and host-tested; it has **not been validated in Eden or on a Switch**. It saves only from ready local Smash result cards, confirms the native UI status chain, and stops for the epoch if a required signal times out. Online replay saving and offline rematch are omitted. See [results evidence](docs/RESULTS.md).

Install path (installation is not part of this task):
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_cpu_pilot.nro`.
Settings: `sd:/ultimate/ssbu-cpu-pilot/config.ini`, or Online Deluxe's CPU Pilot rows.
The settings are `auto_save_replay=on` and `auto_rematch=on`; both default off.

Build with the pinned toolchain:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --no-default-features
pwsh -File tools/build.ps1 -Kind release
pwsh -File tools/build.ps1 -Kind diag
```

The build script runs `cargo skyline build --release` (with `--features diag` for diagnostics).
No demonstration GIF is supplied: emulator use was not authorized.
License: MIT (Cargo package metadata).
Developed with the assistance of AI.
