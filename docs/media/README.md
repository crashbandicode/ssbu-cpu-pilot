# README media — 2026-11-01

All three come from Eden run `m05` on the Ally (Training, Sephiroth vs a CPU
Steve, the operator's 16-plugin card set): Online Deluxe 0.10.1 `a8dcda24…`
and the ssbu-cpu-pilot 0.2.1 release build `e3033426…`. Run evidence:
`/home/intpa/ssbu-cpu-pilot/eden/runs/m05/` and
[EDEN-RESULTS.md](../EDEN-RESULTS.md).

| File | SHA-256 | Source |
| --- | --- | --- |
| `cpu-pilot-page.gif` | `085a6dd4a11434a4124497756bdc16102289958c6c0d37faf5651812461d1665` | Eden's own recording, unchanged: 1280×720, 69 frames, 9.5 s, 10 fps, Eden's frame times. |
| `cpu-pilot-page-crop.png` | `2f7a4dbbc91e337cbfbcc5b9653081fb96282bff4bf727d1e0ccf542ead2e76e` | The GIF at 3.6 s (frame 24, `CPU (L6)`), box `1014,0,1272,64`, enlarged 3× (Lanczos). |
| `window-options-crop.png` | `0a0ca2648b85bcceac4ec1778c3b6e0ebf963d777cc078f6912264c6965b9165` | The GIF at 5.9 s (frame 40), box `1014,0,1272,104`, enlarged 3×. |

Crops: `eden/m05/crops.py <gif> <out_dir>` in the work folder (Pillow).
