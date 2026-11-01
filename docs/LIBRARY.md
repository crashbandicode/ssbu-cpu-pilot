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
| `settings` | yes | `Settings` (P1 CPU, level 1–9, auto rematch, auto save replay), `SharedSettings` (lock-free), `parse_ini` / `write_ini` (no heap); the P1 row (0.1.1): `P1_CHOICES` (`Player`, `CPU (L1)` … `CPU (L9)`), `p1_choice`, `p1_request` (a neighbouring choice toggles Player/CPU keeping the level, the current one steps the level) |
| `combo` | yes | `Combo::step(held, enabled)` (one level step per L+R+X press), `next_level` |
| `level` | yes | the 13.0.5 rank ladder (`ranks`, `rank_for`); refuses a table that differs |
| `pad` | yes | `MappedInputs` (8 bytes, the final-input-mapping output), `from_ai`, `pack` / `unpack` |
| `session` | yes | `Session::step(Frame) -> Action` (`Arm`, `Rearm`, `Think`, `Release`, `None`) |
| `hud` | yes | the "CPU Lv<n>" label in UTF-16 and ASCII |
| `results` | yes | the per-results-screen policy for auto save replay and auto rematch: steps, frame deadlines, one-port press sequences, no retry after a late signal |
| `online` | yes | exact online scope, single local controller-slot election, two-entry binding and bounded stage-initialization lease |
| `native` | Switch only | `controller`, `resolve`, `ready`, `arm`, `think`, `release`, `rank_table`, `scope`, `single_human`, `stage_state`, `stage_begin` / `stage_end`, `stream_copy` (13.0.5 offsets) |

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

Online human-only battles additionally need stage analysis before `ready`
succeeds. `src/online_runtime.rs` shows the experimental standalone adapter:
exact mode admission, controller-slot binding independent of FighterEntry
index, then a manager-wide lease. `stage_begin` must have a matching
`stage_end` at the same native call's exit. They restore the shared RNG and
d0, but not the native phase transition or stage caches. After Ready, keep
isolating manager calls even with the pilot off; stop at the owning scene's
end, not a stock transition. This requires its own guards and runtime proof;
see [ONLINE.md](ONLINE.md). Do not use the raw helpers as a readiness bypass.

## With the standalone plugin installed

`libssbu_cpu_pilot.nro` exports `ssbu_cpu_pilot_state_v1` (settings, the
armed level, the last CPU input) and `ssbu_cpu_pilot_set_v1` (replace the
settings). A host that finds them with `nn::ro::LookupSymbol` should use them
instead of driving P1 itself, so P1 is never driven twice.
