# Online input adapter — 2026-10-31

Implementation under validation. Asymmetric Eden runs `o02` and `o03` passed
the RNG-detector end check ([evidence](EDEN-RESULTS.md)); this does not prove
complete simulation noninterference.
The output is the local controller's eight-byte mapped input at `0x1751ce8`,
before normal packing/transmission. There are no fighter status, damage,
position, velocity or post-unpack input writes in the adapter.

## Strategy and observations

Strategy B follows native stage initialization. A direct think call with the
phase check removed is not justified: think reads the analysis object through
`service+0xc8` (`0x2db81c`, `0x2dc68c`, `0x2dc6d8`), whose geometry query reads
the count at `stage+0x2c` and data at `+0x38` (`0x302a10`). Manager code checks
d1/phase and supplies d0 to analysis (`0x30e694..0x30e6b4`); native completion
writes phase 2 at `0x3119cc`. A deep private-copy strategy A has not been
proved feasible or impossible.

Admission requires initialized game mode and Quickplay's battle: melee
`0x10`, mode `0x07010400` on a Switch (read on a console 2026-11-02; admitted
from 0.2.2), or melee `0x0e`, mode `0x07010101` in Eden; the mode's top bit is
ignored, and the waiting practice (melee `0x0d`) is never admitted. Then
exactly two loaded human controllers and exactly one observed FIM slot.
0.2.1 admitted only Eden's tuple, so on a Switch it never armed online. The
rest of this online path has run only in Eden.
Controller slots are resolved to entries, never assumed to equal entry IDs.
Native controller back-links, scripts, operation flags and slot mirrors are
validated. Secondary AI (`ctrl+0x60`, `ai+0x980`) is refused.

## State accounting

| State | Lifetime / restoration | Evidence and limits |
| --- | --- | --- |
| Mapped input | Normal pack/send path | Asymmetric runs reached normal matching-frame detector ends; complete simulation state is not checked by that detector. |
| AI RNG stream 4, all 20 bytes | Private across decisions and manager calls; shared bytes restored at each return | Includes draw counter. Other eight shared streams are not rewritten. Complete indirect-call draw coverage remains unverified. |
| Controller input scratch `+0xc..+0x58` | Restored after think | Output is copied into mapped input, not left in the simulation controller. |
| Service d0 and slot-kind byte | d0 restored at manager epilogue; slot-kind restored after reset | Native human selector `0x6b7b9c..0x6b7bb0` takes the human branch while d0/slot-kind are clear, bypassing native think `0x6b8014`. |
| AI rank, derived rank values, scripts/decision state | Local AI state persists; saved rank restored when the same identity is released | `0x6b845c/60` write AI+c0c/c20; reset/think update its analysis. No proof yet of every consumer/indirect-call side effect. |
| Service phase, stage geometry, observation caches | Native phase 1→2 and warmed analysis persist to scene end | Intended as human-unused analysis. Full simulation noninterference is not established by the scoped disassembly or an RNG-only detector. |

After Ready the manager RNG lease survives pilot-off and stock transitions:
phase 2 cannot be undone by disabling input emission. A pending attempt stops
after 120 manager calls or lost intent/ownership. Online emission requires an
owned Ready lease. Results, changed singleton/manager/stage, or native CPU
presence invalidate admission. No copied AI/service is allocated.

## Sites and compatibility

The existing five Sites retain their order. Appended 64-byte guards:

- `0x30c770`: manager begin, +0x60; x19 is owner, x29 identifies the frame.
- `0x3119d0`: common manager epilogue, restoration before return.
- `0x1671650`: relied-on mode predicate validating initialized/melee globals.

The dated `card-20261031` scan finds Training Modpack's reference to
`0x30c740`. Source identifies it as a 16-byte body check, plus a replace hook
at `0x30c710`; both precede the chosen begin site. No exact new-hook reference
was reported. This bounded constant scan does not replace live guards.

Both manager sites export sharing-v1 providers, elect before guarding, cache
installed/refused state for later guests, and use independent far-chain stubs.
Their hooks remain available to registered guests when this pilot cannot run.
Both load orders and inline/far stand-ins remain runtime validation work.

## Evidence location

Operator-local disassembly, scan and gate receipts are in
`/home/intpa/ssbu-cpu-pilot-work/research/online-*`. Host tests exercise scope,
all slot masks, entry/slot distinctions, bounded attempts and isolation after
pilot-off. They do not establish Switch memory layouts or online safety.
