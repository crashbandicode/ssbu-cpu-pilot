"""Check one asymmetric Eden match, not complete simulation equivalence.

Usage: python3 -I tools/online_report.py /path/to/runs/oNN [--out report.json]
Surface files are collected under surface/; Yoga files stay at the run root.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re


def events(text, prefix):
    result = []
    for line in text.splitlines():
        match = re.fullmatch(r'(\d+) (.*)', line)
        if match and match[2].startswith(prefix):
            fields = dict(re.findall(r'(\w+)=([^\s]+)', match[2]))
            result.append((int(match[1]), fields, line))
    return result


def one(values, name):
    if len(values) != 1:
        raise ValueError(f'{name}: expected one event, got {len(values)}')
    return values[0]


def end(text, name):
    start = one(events(text, 'session '), name + ' session')
    finish = one(events(text, 'sync_end '), name + ' sync_end')
    if start[1].get('kind') != 'online' or start[0] >= finish[0]:
        raise ValueError(name + ': incomplete online session')
    required = {'trigger': 'normal', 'kind': '1', 'desync_seen': '0', 'already_ending': '0'}
    if any(finish[1].get(k) != v for k, v in required.items()):
        raise ValueError(name + ': abnormal or desynchronized end')
    frame = finish[1].get('frame', '')
    if not frame.isdecimal() or int(frame) <= 0 or finish[1].get('local') not in ('0', '1'):
        raise ValueError(name + ': invalid frame/local fields')
    return finish


def summarize(pilot, surface, yoga, allow_unsampled_input=False):
    s, y = end(surface, 'Surface'), end(yoga, 'Yoga')
    if s[1]['frame'] != y[1]['frame'] or s[1]['local'] == y[1]['local']:
        raise ValueError('end frames differ or both logs describe the same local peer')
    install = one(events(pilot, 'plugin=ssbu-cpu-pilot/'), 'pilot install')
    if install[1].get('result') != 'installed' or install[1].get('heap_allocations') != '0':
        raise ValueError('pilot did not install without allocations')
    begin = one(events(pilot, 'online stage_start '), 'stage_start')
    ready = one(events(pilot, 'online stage=Ready '), 'stage Ready')
    armed = one(events(pilot, 'arm '), 'pilot arm')
    if not install[0] < begin[0] < ready[0] < armed[0] < s[0]:
        raise ValueError('pilot lifecycle order is invalid')
    if begin[1].get('phase') != '1' or ready[1].get('phase') != '2':
        raise ValueError('native analysis did not initialize')
    if any(ready[1].get(k) != '1' for k in ('rng_restored', 'd0_restored')):
        raise ValueError('native restoration receipt missing')
    slot, level = armed[1].get('slot', ''), armed[1].get('level', '')
    if slot not in tuple(map(str, range(8))) or begin[1].get('slot') != slot:
        raise ValueError('local slot changed')
    if level not in tuple(map(str, range(1, 10))):
        raise ValueError('invalid pilot level')
    outputs = [e for e in events(pilot, 'p1_inputs ') if armed[0] < e[0] < s[0]]
    sampled = any(int(e[1].get('last_input', '0'), 16) != 0 for e in outputs)
    if not sampled and (outputs or not allow_unsampled_input):
        raise ValueError('no nonzero mapped input observed during the match')
    if any(armed[0] < e[0] < s[0] for e in events(pilot, 'release ')):
        raise ValueError('pilot released before the normal end')
    return {'result': 'pass' if sampled else 'detector_pass_input_unsampled',
            'scope': 'one asymmetric online RNG-detector match', 'nonzero_input_sampled': sampled,
            'frame': int(s[1]['frame']), 'pilot_slot': int(slot), 'level': int(level),
            'install': install[2], 'stage_start': begin[2], 'stage_ready': ready[2],
            'arm': armed[2], 'surface_sync_end': s[2], 'yoga_sync_end': y[2]}


def report(root, allow_unsampled_input=False):
    paths = {'pilot': root / 'surface/s-ssbu-cpu-pilot/events.log',
             'surface': root / 'surface/s-ssbu-desync-detector/events.log',
             'yoga': root / 'y-ssbu-desync-detector/events.log'}
    texts = {name: path.read_text(encoding='utf-8') for name, path in paths.items()}
    result = summarize(**texts, allow_unsampled_input=allow_unsampled_input)
    s = json.loads((root / 'surface' / f'PREP-{root.name}.json').read_text())['plugins']
    y = json.loads((root / f'PREP-{root.name}.json').read_text())['plugins']
    pilot_name = 'libssbu_cpu_pilot.nro'
    if pilot_name not in s or pilot_name in y:
        raise ValueError('plugin sets are not asymmetric')
    for name in ('libssbu_desync_detector.nro', 'libssbu_tile_cursor.nro'):
        if name not in s or name not in y or s[name] != y[name]:
            raise ValueError('test companion mismatch: ' + name)
    result.update(label=root.name, pilot_sha256=s[pilot_name],
                  log_sha256={name: hashlib.sha256(path.read_bytes()).hexdigest()
                              for name, path in paths.items()})
    result['console_findings'] = {}
    for name, folder in (('surface', root / 'surface/s'), ('yoga', root / 'y')):
        console = (folder / 'console.log').read_text(errors='replace')
        result['console_findings'][name] = [line for line in console.splitlines()
                                            if re.search('panic|abort|crash', line, re.I)]
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--allow-unsampled-input', action='store_true',
                        help='report detector evidence for a match shorter than the input sampling interval')
    args = parser.parse_args()
    try:
        result = report(args.run, args.allow_unsampled_input)
    except (OSError, ValueError, KeyError) as exc:
        parser.exit(1, f'FAIL: {exc}\n')
    text = json.dumps(result, indent=2) + '\n'
    if args.out:
        args.out.write_text(text, encoding='utf-8')
    print(text, end='')


if __name__ == '__main__':
    main()
