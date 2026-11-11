"""Check one Eden waiting-room run of the diag build (0.2.3): the pilot drives P1 in Quickplay's waiting practice,
then a match is found and the battle arms through its own online path.

Usage: python3 -I tools/lobby_report.py <pilot events.log> <detector events.log, this console>
                                        <detector events.log, the peer> [--out report.json]

Every value comes from the logged lines; it reads the pilot's diag events and both consoles' desync-detector events.
Ticks are nn::os system ticks (19.2 MHz) of the console that ran the pilot.
"""
import argparse
import json
from pathlib import Path
import re

TICKS_PER_S = 19_200_000


def events(text, prefix):
    result = []
    for line in text.splitlines():
        match = re.fullmatch(r'(\d+) (.*)', line.strip())
        if match and match[2].startswith(prefix):
            fields = dict(re.findall(r'(\w+)=([^\s]+)', match[2]))
            result.append((int(match[1]), fields, match[0]))
    return result


def first(values, name, after=-1, before=None):
    for value in values:
        if value[0] > after and (before is None or value[0] < before):
            return value
    raise ValueError(f'{name}: no event in range')


def inputs(pilot, start, end):
    lines = [e for e in events(pilot, 'p1_inputs ') if start < e[0] < end]
    nonzero = [e for e in lines if int(e[1].get('last_input', '0'), 16) & ~0x8000 != 0]
    rates = []
    for a, b in zip(lines, lines[1:]):
        calls = int(b[1]['calls']) - int(a[1]['calls'])
        if b[0] > a[0] and calls > 0:
            rates.append(calls * TICKS_PER_S / (b[0] - a[0]))
    return {'samples': len(lines), 'nonzero_samples': len(nonzero),
            'calls_per_s': [round(min(rates), 1), round(max(rates), 1)] if rates else None}


def sync_end(text, name):
    ends = events(text, 'sync_end ')
    if len(ends) != 1:
        raise ValueError(f'{name}: expected one sync_end, got {len(ends)}')
    end = ends[0]
    required = {'trigger': 'normal', 'kind': '1', 'desync_seen': '0', 'already_ending': '0'}
    if any(end[1].get(k) != v for k, v in required.items()):
        raise ValueError(f'{name}: abnormal or desynchronized end')
    return end


def summarize(pilot, local, peer):
    install = first(events(pilot, 'plugin=ssbu-cpu-pilot/'), 'install')
    if install[1].get('result') != 'installed' or install[1].get('heap_allocations') != '0':
        raise ValueError('pilot did not install without allocations')
    scenes = events(pilot, 'scene ')
    arms = events(pilot, 'arm ')
    releases = events(pilot, 'release ')
    practice = first([s for s in scenes if s[1].get('scope') == 'WaitingPractice'], 'practice scene')
    battle = first([s for s in scenes if s[1].get('scope') == 'Online1v1'], 'battle scene', practice[0])
    p_arm = first(arms, 'practice arm', practice[0], battle[0])
    if p_arm[1].get('slot') != '0':
        raise ValueError('the practice armed another slot than 0')
    if any(s[0] for s in scenes if practice[0] < s[0] < p_arm[0]):
        raise ValueError('the scene changed before the practice armed')
    b_arm = first(arms, 'battle arm', battle[0])
    stage = first(events(pilot, 'online stage=Ready '), 'stage Ready', battle[0], b_arm[0])
    ended = [r for r in releases if p_arm[0] < r[0] < b_arm[0]]
    if not ended:
        raise ValueError('the practice session was never released before the battle armed')
    if any(r[1].get('restored') == 'true' for r in ended if r[0] > battle[0]):
        raise ValueError('a rank was restored after the battle scope came up')
    handovers = [h for h in events(pilot, 'handover ') if battle[0] <= h[0] < b_arm[0]]
    if handovers and (handovers[0][1].get('from'), handovers[0][1].get('to')) != ('WaitingPractice', 'Online1v1'):
        raise ValueError('unexpected handover')
    mine, theirs = sync_end(local, 'local'), sync_end(peer, 'peer')
    if mine[1].get('frame') != theirs[1].get('frame') or mine[1].get('local') == theirs[1].get('local'):
        raise ValueError('end frames differ or both logs describe the same console')
    if not b_arm[0] < mine[0]:
        raise ValueError('the battle armed after the match ended')
    if any(b_arm[0] < r[0] < mine[0] for r in releases):
        raise ValueError('the pilot released during the match')
    in_practice = inputs(pilot, p_arm[0], battle[0])
    in_battle = inputs(pilot, b_arm[0], mine[0])
    if not in_practice['nonzero_samples'] or not in_battle['nonzero_samples']:
        raise ValueError('no non-zero CPU input sampled in the practice or the battle')
    back = [s for s in scenes if s[0] > mine[0] and s[1].get('scope') == 'WaitingPractice']
    rearm = first(arms, 'practice re-arm', back[0][0]) if back else None
    return {
        'install': install[2],
        'practice': {'scene': practice[2], 'arm': p_arm[2], 'inputs': in_practice},
        'handover': {'battle_scene': battle[2], 'handover': handovers[0][2] if handovers else None,
                     'practice_release': ended[0][2], 'stage_ready': stage[2], 'battle_arm': b_arm[2],
                     'seconds_scene_to_arm': round((b_arm[0] - battle[0]) / TICKS_PER_S, 2)},
        'battle': {'inputs': in_battle, 'local_sync_end': mine[2], 'peer_sync_end': theirs[2],
                   'frame': int(mine[1]['frame'])},
        'back': {'scene': back[0][2] if back else None, 're_arm': rearm[2] if rearm else None},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('pilot', type=Path)
    parser.add_argument('local', type=Path)
    parser.add_argument('peer', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    try:
        result = summarize(*(p.read_text(encoding='utf-8', errors='replace') for p in (args.pilot, args.local, args.peer)))
    except (OSError, ValueError, KeyError) as exc:
        parser.exit(1, f'FAIL: {exc}\n')
    text = json.dumps(result, indent=2) + '\n'
    if args.out:
        args.out.write_text(text, encoding='utf-8')
    print(text, end='')


if __name__ == '__main__':
    main()
