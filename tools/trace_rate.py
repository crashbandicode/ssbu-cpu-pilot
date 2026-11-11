"""The pilot's entry per game update, from an Eden guest-trace JSONL (eden/lobby-0.2.3/pp.py `per_update` windows).

Usage: python3 -I tools/trace_rate.py <instance trace.jsonl> [--out report.json]

A window is one armed point on the practice's own update (`main+0x151b150`, once per update of the waiting
practice). Over the intervals between its hits (`updates` = hits - 1) it counts the hits of the pilot's entry (final input mapping's exit,
`main+0x1751ce8`) per controller slot (x23), and P1's mapped input two instructions later (`main+0x1751cf0`, the
u64 at x19 after the pilot wrote it; active when anything but the tap-jump flag 0x8000 is set).
"""
import argparse
import json
from pathlib import Path

UPDATE = 'main+0x151b150'
ENTRY = 'main+0x1751ce8'
MAPPED = 'main+0x1751cf0'


def kind(record):
    places = set((record.get('locations') or {}).values())
    for name in (UPDATE, ENTRY, MAPPED):
        if name in places:
            return name
    return None


def number(value):
    try:
        return int(str(value), 0)
    except (TypeError, ValueError):
        return None


def windows(lines):
    hits = []
    for line in lines:
        if '"point_id"' not in line:
            continue
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get('event') != 'entry':
            continue
        name = kind(record)
        if name:
            hits.append((int(record['wall_time_ns']), name, str(record.get('point_id')), record))
    spans = {}
    for t, name, point, _ in hits:
        if name == UPDATE:
            first, last, count = spans.get(point, (t, t, 0))
            spans[point] = (min(first, t), max(last, t), count + 1)
    result = []
    for point, (first, last, updates) in sorted(spans.items(), key=lambda item: item[1][0]):
        # The updates-1 intervals between the window's update hits: each holds one update's input mapping.
        updates -= 1
        inside = [h for h in hits if first <= h[0] < last]
        slots = {}
        mapped = []
        for _, name, _, record in inside:
            if name == ENTRY:
                slot = number((record.get('registers') or {}).get('x23'))
                slots[slot] = slots.get(slot, 0) + 1
            elif name == MAPPED:
                captures = record.get('captures') or []
                value = number(captures[0].get('value')) if captures and isinstance(captures[0], dict) else None
                if value is not None:
                    mapped.append(value)
        result.append({'update_point': point, 'seconds': round((last - first) / 1e9, 2), 'updates': updates,
                       'entry_by_slot': {str(k): v for k, v in sorted(slots.items(), key=lambda i: str(i[0]))},
                       'slot0_per_update': round(slots.get(0, 0) / updates, 3) if updates else None,
                       'mapped_samples': len(mapped), 'mapped_active': sum(1 for v in mapped if v & ~0x8000)})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('trace', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    with args.trace.open(encoding='utf-8', errors='replace') as f:
        result = windows(f)
    text = json.dumps(result, indent=1) + '\n'
    if args.out:
        args.out.write_text(text, encoding='utf-8')
    print(text, end='')


if __name__ == '__main__':
    main()
