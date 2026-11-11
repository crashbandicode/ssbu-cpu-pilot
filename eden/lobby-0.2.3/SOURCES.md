# Sources of the hooks and segments these rows use (not copied)

The rows also load hooks and route segments from other lanes' Eden folders, unchanged:

| File | sha256 | Source |
| --- | --- | --- |
| `r2.py` | `e5e782b622cefcb16ed68d33753812165f22937d4c27b459317a255ca350898d` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `spd.py` | `60ffa60266f230ca72ff18945c5536763f9e8ab33227d2ad0517638fd26905ec` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `steps.py` | `38041948dc79eac35d159251ac479940e210dbdc67e5fe478e051b68545e44ea` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/to-online.edenmacro.json` | `d52590537267a9aa817e3d7e8969fde0d70e38ca93cf36a5ca616ba8d4678858` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/to-quickplay.edenmacro.json` | `4db24aee967ca0721918af0291deb53cbefdc07ce4485cf8d423401a5bc63acc` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/to-css.edenmacro.json` | `df589d123449a453b70525e393591b2a95249bc8d55470402985236df774a4d5` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/pick.edenmacro.json` | `aa7177b16f3c04163c79e3faac2cc3f5ba6ea52ab569c5a0d2d6c5e5acf7ca02` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/tile-up.edenmacro.json` | `281878f3b0c24b0a8703890518e1579cdc5aa1946320754da05605fbebe954eb` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/ready.edenmacro.json` | `a66184f087ac3841bbecd7198f2adbc20ffab73d18c1ba2a84edecc4027c7ede` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/speed-left1.edenmacro.json` | `2f50aa0293d96c23cf4388ed0b805e72a0ce526d5242ff1c3eab108c20031792` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/speed-left3.edenmacro.json` | `b4959e57ba4809ebfdb12667d1112dedbeb53519e56ca3091f5496ccb08a7caa` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `segments/speed-right1.edenmacro.json` | `296ed174a477649e703afc1b601a67048bbb7a1fff053d2fcd0879c73eee564a` | QCF's `/home/intpa/eden-runs/inputs-qcf/rows-r2b/` (the Yoga) |
| `r14bc.py` | `001f127725c4a0a9ed99ca1d19bc863698b87ee86b17671b90fb507c13ffb56c` | worker B's R14 hooks, `/tmp/opencode/r7/r14bc/rows/r14bc.py` (the Surface) |
| `segments/pilot-page-step.edenmacro.json` (copied) | `db109fa89164d216cf7516057f28626012db5f7015ee870691e47c194074c2ac` | TrainingRooms `eden/lr14t-shared/segments/` |
| `segments/pilot-arm.edenmacro.json` (copied) | `bd024e051944683b3ef4b73fc0da3821d03c3627d441eed05e0477531961e728` | TrainingRooms `eden/lr14t-shared/segments/` |
| `segments/pilot-release.edenmacro.json` (copied) | `48793ebf9713be0db63f2dfa3575e4ce0e84e68fa6e0c923c50a1b1c08780b92` | TrainingRooms `eden/lr14t-shared/segments/` |
| `segments/chord-record.edenmacro.json` (copied) | `f2fa1848d9988aea562b142421767175b60a3b836770c3f537b7473e9dd18e04` | worker B's `/home/intpa/eden-runs/r7-rows-stage/segments/` |

Runner: the playbook's `tools/eden-runner/runner.py` at `4ca80e1`, run through `tools/eden-test/eden_test.py`.
pp1 (`pp1-a725c27-y`) ran an earlier `pp.py` (sha256 `b6e47c7dfd32…`): the same functions, before `speeds` took a `tag`.
