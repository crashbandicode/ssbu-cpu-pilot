# README media — 2026-11-01

All three come from Eden run `m02` on the Ally (Training, Sephiroth vs a CPU
Kirby): Online Deluxe 0.10.0 `a070b1a7…` and the pilot release build
`49efcc06…` (commit `be1838f`; the page and rows are those of 0.2.0). Run
evidence: `/home/intpa/ssbu-cpu-pilot/eden/runs/m02/` and
[EDEN-RESULTS.md](../EDEN-RESULTS.md).

| File | SHA-256 | Source |
| --- | --- | --- |
| `cpu-pilot-page.gif` | `caa6a8556e727367bfd91480a5dc757d9cbdad39d88bdacbe2b0136b7d010cb2` | Eden's own recording, unchanged: 1280×720, 76 frames, 9.5 s, 10 fps, Eden's frame times. Re-encoding with the playbook's `encode.py` (128 colours) made it larger (12.9 MB), so it was kept as is. |
| `cpu-pilot-page-crop.png` | `71dca8a6884bb2f86f6d1ff09f00ea1008bab299dbb12942fc2b71469f432f67` | Frame 20 of the GIF (2.96–3.07 s), box `1012,0,1274,66`, enlarged 3× (Lanczos). |
| `window-options-crop.png` | `6319f15e73f5fcb9b38ae329db2cd33125e3d0debc1db2edc389c70ac8d28460` | The Window Options still (`31b1ab03…`, last frame of a 300 ms clip), box `1012,0,1274,96`, enlarged 3×. |

Crops: `eden/m02/crops.py <gif> 3.0 <still> <out_dir>` in the work folder
(Pillow).
