# Online gameplay capture — 2026-10-31

`online-pilot.gif` is the complete, uncropped Surface recording from Eden
run `o03`, started when the pilot logged Ready and armed. It shows the opening
approach and attacks; the match continues beyond the clip. The peer receives
only the neutral-position keepalive. See [the run evidence](../EDEN-RESULTS.md).

Raw source SHA-256:
`e6ffeb010ee497799bfdbdfb801e45b57c101474058316b4300265247c8d12da`.
Raw capture and first/middle/last review images are retained under
`/home/intpa/ssbu-cpu-pilot/eden/runs/o03/`.

Encode using playbook `69a08c0` (all original frame durations retained):

```text
python3 -I /home/intpa/ssbu-mod-playbook/tools/eden-solo/encode.py /home/intpa/ssbu-cpu-pilot/eden/runs/o03/s-online-pilot.gif docs/media/online-pilot.gif 0,0,1280,720 0 128
```
