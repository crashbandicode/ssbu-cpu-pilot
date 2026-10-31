# Using ssbu-cpu-pilot-core from another plugin

2026-10-31. For a plugin that wants P1-as-CPU with its own hooks and menu,
such as the Training Modpack.

## Dependency

```toml
[dependencies]
ssbu-cpu-pilot-core = { git = "https://git.syntaxbyexample.com/intpa/ssbu-cpu-pilot", rev = "<commit>" }

[target.'cfg(target_os = "switch")'.dependencies]
ssbu-cpu-pilot-core = { git = "https://git.syntaxbyexample.com/intpa/ssbu-cpu-pilot", rev = "<commit>", features = ["native"] }
```

Or a `path` to a local checkout. The crate has no dependencies and no
Skyline dependency, so it links into a plugin built with any Skyline
version. Pin a `rev`: the API is versioned by commit until a release.

## Modules

| Module | Host-tested | What it gives |
| --- | --- | --- |
| `settings` | yes | `Settings` (P1 CPU, level 1–9, auto rematch, auto save replay), `SharedSettings` (lock-free), `parse_ini` / `write_ini` (no heap) |
| `combo` | yes | `Combo::step(held, enabled)` (one level step per L+R+X press), `next_level` |
| `level` | yes | the 13.0.5 rank ladder (`ranks`, `rank_for`); refuses a table that differs |
| `pad` | yes | `MappedInputs` (8 bytes, the final-input-mapping output), `from_ai`, `pack` / `unpack` |
| `session` | yes | `Session::step(Frame) -> Action` (`Arm`, `Rearm`, `Think`, `Release`, `None`) |
| `hud` | yes | the "CPU Lv<n>" label in UTF-16 and ASCII |
| `native` | Switch only | `controller`, `resolve`, `ready`, `arm`, `think`, `release`, `rank_table` (13.0.5 offsets) |

## The host's part

Per battle frame, for P1's controller (as `src/runtime.rs` does):

1. Find P1's `FighterEntry`, then `native::controller` and `native::resolve`
   (it refuses a real CPU and anything not fully loaded).
2. Build a `session::Frame` from the setting, the battle state,
   `native::ready` and ready-go, and call `Session::step`.
3. `Arm(l)`: `native::arm(base, n, rank_for(l))`, keep the old rank and the
   private stream. `Rearm(l)`: `native::release` with the kept rank, then arm.
   `Release`: `native::release` only if the objects are still the armed ones.
4. On `Arm`, `Rearm` and `Think`, write `native::think(...)` over P1's
   `MappedInputs` after the game's final input mapping (`0x1750dd0`) ran.

Every `native` call is meant for the game thread during a battle, after the
host checked the 13.0.5 version and the 64-byte guards of `0x2da180`,
`0x6b8400` and `0x2d8d40` (`src/model.rs`). The AI draws only on a private
copy of the AI random stream; the shared stream is restored before each
call returns.

## With the standalone plugin installed

`libssbu_cpu_pilot.nro` exports `ssbu_cpu_pilot_state_v1` (settings, the
armed level, the last CPU input) and `ssbu_cpu_pilot_set_v1` (replace the
settings). A host that finds them with `nn::ro::LookupSymbol` should use them
instead of driving P1 itself, so P1 is never driven twice.
