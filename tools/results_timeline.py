"""Per-epoch timeline of the diag build's `results` lines, as offsets from `epoch=begin`.

Usage: python3 -I tools/results_timeline.py <events.log>

Ticks are the 19.2 MHz system tick that starts each line. For each results epoch it prints
every event as `+S.SS s <event> step=<stage> status=<n>`, then one verdict line:
`save=ok` only when save=press_y, select_yes, confirm, writing, close_success and completed
appear once each, in that order, the last with status=5, and no splash A follows the Y;
`rematch=` the rematch outcome (accepted, a flow= token logged in the Rematch/Answer
stages, or none).
"""
import re
import sys

TICK_HZ = 19_200_000
LINE = re.compile(r'(\d+) results epoch=(0x[0-9a-f]+) port=(\d+) (\S+)(?: (n=\d+))?(.*)')
CHAIN = ('save=press_y', 'save=select_yes', 'save=confirm', 'save=writing', 'save=close_success',
         'save=completed')


def parse(text):
    """Epochs in order of `epoch=begin`: [(epoch, begin_tick, [(tick, event, fields), ...])]."""
    epochs, current = [], {}
    for line in text.splitlines():
        m = LINE.fullmatch(line.strip())
        if not m:
            continue
        tick, epoch, event = int(m[1]), m[2], m[4] + (f' {m[5]}' if m[5] else '')
        fields = dict(re.findall(r'(\w+)=(\S+)', m[6]))
        if m[4] == 'epoch=begin':
            current[epoch] = (epoch, tick, [])
            epochs.append(current[epoch])
        if epoch in current:
            current[epoch][2].append((tick, event, fields))
    return epochs


def verdict(events):
    names = [e for _, e, _ in events]
    chain = [n for n in names if n in CHAIN]
    ok = chain == list(CHAIN) and events[names.index(CHAIN[-1])][2].get('status') == '5'
    if ok:
        y = names.index(CHAIN[0])
        ok = not any(n.startswith('save=press_a_splash') for n in names[y:])
    rematch = 'none'
    for _, e, f in events:
        if e == 'rematch=accepted':
            rematch = 'accepted'
        elif e.startswith('flow=') and f.get('step') in ('Rematch', 'Answer'):
            rematch = e
    return ok, rematch


def report(text):
    out = []
    for epoch, begin, events in parse(text):
        out.append(f'epoch={epoch} begin={begin}')
        for tick, event, fields in events:
            out.append(f'  +{(tick - begin) / TICK_HZ:6.2f} s {event} step={fields.get("step", "-")}'
                       f' status={fields.get("status", "-")}')
        ok, rematch = verdict(events)
        out.append(f'  save={"ok" if ok else "no"} rematch={rematch}')
    return '\n'.join(out)


if __name__ == '__main__':
    with open(sys.argv[1], encoding='utf-8', errors='replace') as f:
        print(report(f.read()))
