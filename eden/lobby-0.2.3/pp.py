"""R15 rows (tr-opus-pilot-patch, 2026-11-11, TASK-pilot-lobby-patch.md): ssbu-cpu-pilot 0.2.3 (diag, branch
`patch/lobby-practice-drive`) drives P1 in Quickplay's waiting-room practice, beside TrainingRooms' lobby tools, and
hands over to the Quickplay match.

b: TrainingRooms (diag) + Online Deluxe 0.10.4 (the card's) + the pilot (diag) + the detector + the tile cursor;
a: vanilla (detector + tile cursor). Built on QCF's R2 pair rows (`steps.py`, `r2.py`) and worker B's R14 hooks
(`r14bc.py`: the pilot entry's per-frame trace window).

Every row passes on the pilot's own diag lines (`sd:/ultimate/ssbu-cpu-pilot/events.log`):
- `scene scope=WaitingPractice slot=0`, `gate <reason> armed=<n>`, `arm level=<l> rank=<r> old_rank=<o> slot=<s>`,
  `p1_inputs calls=<n> last_input=<hex>` (every 600 of P1's input mappings), `release restored=<b> old_rank=<o>`,
  `handover from=<scope> to=<scope>`, `stale no_battle_frames armed_cleared=1`, `online stage_start …`;
- TrainingRooms' diag log for Record (`[PracticeRecording] record_refused`, `screen=recording … recording=1`) and the
  speeds (`[GameSpeed] lobby …`);
- the detectors' `sync_end … desync_seen=0` on both consoles.
"""
import json
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import r2  # noqa: E402
import r14bc  # noqa: E402
import steps  # noqa: E402

PILOT_LOG = 'user/sdmc/ultimate/ssbu-cpu-pilot/events.log'
PILOT_INI = 'user/sdmc/ultimate/ssbu-cpu-pilot/config.ini'
STATE = 'pp-state.json'
ARM = re.compile(r'arm level=(\d) rank=(\d+) old_rank=(-?\d+) slot=(\d)')
P1_INPUTS = re.compile(r'p1_inputs calls=(\d+) last_input=(0x[0-9a-f]+)')
RELEASE = re.compile(r'release restored=(\w+) old_rank=(-?\d+)')


def _profile(ctx, on=None):
    return Path(ctx.runner.instances[on or ctx.on]['profile'])


def _plog(ctx, on=None):
    return _profile(ctx, on) / PILOT_LOG


def _size(path):
    try:
        return path.stat().st_size
    except OSError:
        return 0


def _plines(ctx, start, on=None):
    p = _plog(ctx, on)
    try:
        with p.open('rb') as f:
            f.seek(start)
            return [l.strip() for l in f.read().decode('utf-8', 'replace').splitlines() if l.strip()]
    except OSError:
        return []


def _wait_plog(ctx, start, pattern, cap_s):
    end = time.monotonic() + cap_s
    while time.monotonic() < end:
        ctx.check_stop()
        hits = [l for l in _plines(ctx, start) if re.search(pattern, l)]
        if hits:
            return hits[0]
        time.sleep(0.5)
    return None


def _ini(ctx):
    p = _profile(ctx) / PILOT_INI
    return p.read_text(errors='replace') if p.is_file() else ''


def _state(ctx):
    p = Path(ctx.runner.out) / STATE
    return json.loads(p.read_text()) if p.is_file() else {}


def _remember(ctx, **kw):
    p = Path(ctx.runner.out) / STATE
    s = _state(ctx)
    s.update(kw)
    p.write_text(json.dumps(s, indent=1) + '\n')


def _play(ctx, name):
    seg = ctx.runner.plan['segments'][name]
    path = Path(seg['file'])
    ctx.call('macro.play', {'path': str(path if path.is_absolute() else Path(ctx.runner.base) / path)})
    ctx.runner.wait_macro(ctx.on, seg.get('cap', 20))
    ctx.call('input.reset')


def _tap(ctx, button):
    ctx.call('input.button', {'button': button, 'state': 'tap', 'duration_ms': 100, 'lease_ms': 500})
    time.sleep(0.3)
    ctx.call('input.reset')


def _tick_s(line):
    m = re.match(r'(\d+) ', line)
    return int(m.group(1)) / 19_200_000 if m else None


def installs(ctx):
    """b: the pilot's install line (0.2.3, every site installed, no hook heap, the Deluxe rows registered)."""
    lines = [l for l in _plines(ctx, 0) if 'plugin=ssbu-cpu-pilot/' in l]
    out = {'install': lines[-1:]}
    steps._save(ctx, 'pp-install', out)
    if not lines or not re.search(r'plugin=ssbu-cpu-pilot/0\.2\.3 .*result=installed heap_allocations=0 .*menu=registered',
                                  lines[-1]):
        raise RuntimeError('pilot install line: %s' % lines[-1:])
    return {'install': lines[-1][:300]}


def practice_ready(ctx, cap_s=120.0):
    """b: TrainingRooms' waiting-room practice is up (`[LobbyPractice] active=true`, the last such line), then a
    settle."""
    end = time.monotonic() + cap_s
    active = None
    while time.monotonic() < end:
        ctx.check_stop()
        states = r2._lines_since(ctx, 0, r'\[LobbyPractice\] active=')
        if states and 'active=true' in states[-1]:
            active = states[-1]
            break
        time.sleep(1.0)
    time.sleep(5.0)
    out = {'active': active, 'shot': str(ctx.shot('pp-practice-ready'))}
    steps._save(ctx, 'pp-practice-ready', out)
    if not active:
        raise RuntimeError('b: the practice is not up within %s s' % cap_s)
    return {'active': active[:160]}


def arm_in_practice(ctx, presses=3, close=2):
    """b in the waiting practice, the pad otherwise idle: Online Deluxe's overlay (L + R + Down `presses` times to its CPU
    Pilot page), ZR + Right sets P1 to CPU, L + R + Down `close` times closes it. Passes when `config.ini` says `p1=cpu`
    and the pilot logs `arm … slot=0` after `scene scope=WaitingPractice` (MACROS rule 15: the behaviour, not the
    setting)."""
    start = _size(_plog(ctx))
    tr_mark = r2._mark(ctx)
    shots = {'before': str(ctx.shot('pp-arm-before'))}
    for i in range(presses):
        _play(ctx, 'pilot-page-step')
        time.sleep(1.3)
        shots['page%d' % i] = str(ctx.shot('pp-arm-page%d' % i))
    _play(ctx, 'pilot-arm')
    time.sleep(1.3)
    shots['set'] = str(ctx.shot('pp-arm-set'))
    for i in range(close):
        _play(ctx, 'pilot-page-step')
        time.sleep(1.3)
    shots['closed'] = str(ctx.shot('pp-arm-closed'))
    # ZR + Right is also TrainingRooms' recording pop-up chord; B closes it (r7.py's _overlay_pilot does the same).
    _tap(ctx, 'b')
    time.sleep(1.0)
    shots['popup_closed'] = str(ctx.shot('pp-arm-popup-closed'))
    ini, end = '', time.monotonic() + 10
    while time.monotonic() < end and not re.search(r'^p1\s*=\s*cpu', ini, re.M):
        ini = _ini(ctx)
        time.sleep(0.5)
    arm = _wait_plog(ctx, start, ARM.pattern, 20)
    time.sleep(3.0)
    shots['driving'] = str(ctx.shot('pp-arm-driving'))
    lines = _plines(ctx, start)
    tr = [l[:200] for l in r2._lines_since(ctx, tr_mark, r'PracticeRecording|notice|GameSpeed|LobbyPractice')][-12:]
    out = {'ini': ini[:200], 'arm': arm, 'pilot_lines': lines[-20:], 'tr_lines': tr, 'shots': shots}
    steps._save(ctx, 'pp-arm', out)
    scene = [l for l in _plines(ctx, 0) if 'scene scope=' in l]
    if not re.search(r'^p1\s*=\s*cpu', ini, re.M):
        raise RuntimeError('the CPU Pilot page did not set p1=cpu: %r' % ini[:120])
    if not arm or ARM.search(arm).group(4) != '0':
        raise RuntimeError('no arm on slot 0 in the practice: %s' % lines[-8:])
    if not scene or 'scope=WaitingPractice' not in scene[-1]:
        raise RuntimeError('the last scene line is not the waiting practice: %s' % scene[-2:])
    _remember(ctx, armed_at=start)
    return {'arm': arm, 'scene': scene[-1]}


def drives(ctx, seconds=24.0, tag='pp-drive-x1'):
    """b, the pad idle: the pilot keeps driving P1 (no release; `p1_inputs` rising with non-zero CPU input), its entry
    once per vsync on slot 0 (r14bc.window), and frames over the span (P1 moving, the partner taking hits)."""
    start = _size(_plog(ctx))
    began = time.monotonic()
    shots = {'t0': str(ctx.shot(tag + '-t0'))}
    w = r14bc.window(ctx, tag, seconds=6.0)
    while time.monotonic() - began < seconds:
        ctx.check_stop()
        time.sleep(1.0)
        if 't8' not in shots and time.monotonic() - began >= 8:
            shots['t8'] = str(ctx.shot(tag + '-t8'))
    shots['end'] = str(ctx.shot(tag + '-end'))
    lines = _plines(ctx, start)
    inputs = [P1_INPUTS.search(l) for l in lines if P1_INPUTS.search(l)]
    released = [l for l in lines if RELEASE.search(l) or 'stale ' in l]
    slot0 = (w.get('per_slot') or {}).get('0') or {}
    out = {'p1_inputs': [(m.group(1), m.group(2)) for m in inputs], 'released': released, 'window_slot0': slot0,
           'window_slots': sorted((w.get('per_slot') or {}).keys()), 'pilot_lines': lines[-12:], 'shots': shots}
    steps._save(ctx, tag, out)
    problems = []
    if released:
        problems.append('released while driving: %s' % released[:2])
    if not inputs or all(int(m.group(2), 16) == 0 for m in inputs):
        problems.append('no non-zero CPU input logged (p1_inputs): %s' % lines[-6:])
    rate = slot0.get('calls_per_vsync')
    if rate is None or not 0.8 <= rate <= 1.2:
        problems.append('slot 0 entry %s per vsync (1 expected)' % rate)
    if problems:
        raise RuntimeError('; '.join(problems))
    return {'p1_inputs': out['p1_inputs'][-3:], 'calls_per_vsync': rate}


def record_refused(ctx):
    """b, the pilot driving: TrainingRooms' Record chord (ZR + D-pad Down) is refused (`record_refused`, its notice),
    and no capture starts (`recording=1`). The R7 refusal, in the lobby."""
    mark = r2._mark(ctx)
    start = _size(_plog(ctx))
    _play(ctx, 'chord-record')
    found = r2._wait_line(ctx, mark, r'record_refused', 8)
    time.sleep(0.6)
    shot = str(ctx.shot('pp-record-refused'))
    time.sleep(2.0)
    lines = [l[:220] for l in r2._lines_since(ctx, mark, r'PracticeRecording|notice_draw')]
    started = [l for l in lines if re.search(r'screen=recording role=\w+ recording=1', l)]
    plines = _plines(ctx, start)
    out = {'refused': found, 'started': started, 'tr_lines': lines[-12:], 'pilot_lines': plines[-6:], 'shot': shot}
    steps._save(ctx, 'pp-record-refused', out)
    if not found or started:
        raise RuntimeError('Record not refused while the pilot drives: %s' % lines[-6:])
    if any(RELEASE.search(l) for l in plines):
        raise RuntimeError('the pilot released during the Record chord: %s' % plines[-4:])
    return {'refused': found[:200]}


LOBBY_LINE = r'\[GameSpeed\] lobby '


def speeds(ctx, plan=None, tag='pp-speeds'):
    """b, the pilot driving: each lobby speed the build offers, reached with the speed hold's segments from x1 and
    back. Per speed: the plugin's line, r14bc.window (the pilot's entry per vsync on slot 0: no doubling), the pilot
    still armed (no release) and `p1_inputs` rising. `plan`: [[label, [segments]], ...] from x1, then back to x1."""
    plan = plan or [['1/2', ['speed-left1']], ['1/4', ['speed-left1']], ['x1', ['speed-right1', 'speed-right1']]]
    start = _size(_plog(ctx))
    results = [dict(r14bc.window(ctx, tag + '-start'), speed='start')]
    for label, segs in plan:
        mark = r2._mark(ctx)
        for name in segs:
            _play(ctx, name)
            time.sleep(1.0)
        line = r2._wait_line(ctx, mark, LOBBY_LINE, 10)
        time.sleep(1.5)
        w = r14bc.window(ctx, '%s-%d-%s' % (tag, len(results), label.replace('/', '-').replace('.', '_')), seconds=6.0)
        w['speed'] = label
        w['line'] = line
        w['lines'] = [l[:200] for l in r2._lines_since(ctx, mark, LOBBY_LINE)][-6:]
        results.append(w)
    time.sleep(10.0)
    plines = _plines(ctx, start)
    inputs = [P1_INPUTS.search(l).groups() for l in plines if P1_INPUTS.search(l)]
    released = [l for l in plines if RELEASE.search(l) or 'stale ' in l]
    out = {'speeds': [{'speed': w['speed'], 'line': w.get('line'), 'lines': w.get('lines'),
                       'slot0': (w.get('per_slot') or {}).get('0')} for w in results],
           'p1_inputs': inputs, 'released': released}
    steps._save(ctx, tag, out)
    problems = []
    for w in results:
        rate = ((w.get('per_slot') or {}).get('0') or {}).get('calls_per_vsync')
        if rate is None or not 0.8 <= rate <= 1.2:
            problems.append('%s: slot 0 entry %s per vsync' % (w['speed'], rate))
        if w['speed'] not in ('x1', 'start') and not w.get('line'):
            problems.append('%s: no [GameSpeed] lobby line' % w['speed'])
    if released:
        problems.append('released during the speeds: %s' % released[:2])
    if len(plan) > 1 and len(inputs) < 2:
        problems.append('p1_inputs did not keep rising: %s' % inputs)
    if problems:
        raise RuntimeError('; '.join(problems))
    return {'speeds': [(s['speed'], (s['slot0'] or {}).get('calls_per_vsync'), (s['line'] or '')[-80:])
                       for s in out['speeds']]}


def mark(ctx, key='match'):
    """b: remember the pilot log's size (the handover's start), the detectors' logs on both."""
    det = {k: steps._size(ctx.runner.instances[k]['detector_log']) for k in ctx.runner.instances}
    _remember(ctx, **{key: {'pilot': _size(_plog(ctx)), 'detectors': det, 'at': time.time()}})
    return {key: _state(ctx)[key]}


def handover(ctx, key='match', cap_s=180.0):
    """b, while a joins: the match is found with the pilot driving. Passes on the pilot's own lines since `mark`: the
    practice session dropped without touching its objects (`handover from=WaitingPractice to=Online1v1`, then
    `release restored=false`), the battle scene (`scene scope=Online1v1`), and the battle arming through its existing
    path (`online stage_start`, `online stage=Ready`, `arm …`), with both detectors' sessions open."""
    st = _state(ctx)[key]
    start, dets = st['pilot'], st['detectors']
    began, sessions, armed = time.monotonic(), {}, None
    while time.monotonic() - began < cap_s:
        ctx.check_stop()
        for k, p in ((k, ctx.runner.instances[k]['detector_log']) for k in ctx.runner.instances):
            if k not in sessions and steps.SESSION.search(steps._tail(p, dets[k])):
                sessions[k] = round(time.monotonic() - began, 1)
        lines = _plines(ctx, start)
        battle = [i for i, l in enumerate(lines) if 'scene scope=Online1v1' in l]
        if battle:
            arms = [l for l in lines[battle[0]:] if ARM.search(l)]
            if arms:
                armed = arms[0]
        if len(sessions) == len(ctx.runner.instances) and armed:
            break
        time.sleep(1.0)
    time.sleep(2.0)
    lines = _plines(ctx, start)
    shot = str(ctx.shot('pp-%s-handover' % key))
    hand = [l for l in lines if 'handover from=' in l]
    rel = [l for l in lines if RELEASE.search(l)]
    stage = [l for l in lines if 'online stage' in l]
    out = {'sessions_after_s': sessions, 'handover': hand, 'release': rel, 'stage': stage, 'armed': armed,
           'pilot_lines': lines[-40:], 'shot': shot}
    steps._save(ctx, 'pp-%s-handover' % key, out)
    problems = []
    if len(sessions) < len(ctx.runner.instances):
        problems.append('no online session on both detectors: %s' % sessions)
    battle = next((i for i, l in enumerate(lines) if 'scene scope=Online1v1' in l), None)
    arm_at = next((i for i, l in enumerate(lines) if battle is not None and i > battle and ARM.search(l)), None)
    if battle is None or arm_at is None:
        problems.append('the battle did not arm the pilot: %s' % lines[-8:])
    else:
        # The practice session ends before the battle arms: a release (the handover on the first battle frame, or
        # the practice's own end), and never a restore on objects once the battle scope is up.
        ended = [l for l in lines[:arm_at] if RELEASE.search(l)]
        if not ended:
            problems.append('the practice session was never released before the battle armed')
        restored_in_battle = [l for l in lines[battle:arm_at] if RELEASE.search(l) and 'restored=true' in l]
        if restored_in_battle:
            problems.append('a restore after the battle scope came up: %s' % restored_in_battle[:1])
        if hand and ('from=WaitingPractice' not in hand[0] or 'to=Online1v1' not in hand[0]):
            problems.append('unexpected handover: %s' % hand[:1])
    if problems:
        raise RuntimeError('; '.join(problems))
    return {'handover': hand[:1], 'release': rel[:2], 'stage': stage[:2], 'armed': armed}


def match_end(ctx, key='match'):
    """b after `steps.battle`: both detectors' `sync_end … desync_seen=0` at the same frame, and the pilot's lines over
    the match (still armed to the end; the stale clear after it)."""
    st = _state(ctx)[key]
    ends = {}
    for k in ctx.runner.instances:
        text = steps._tail(ctx.runner.instances[k]['detector_log'], st['detectors'][k])
        ends[k] = [m.group(0) for m in steps.SYNC_END.finditer(text)]
    frames = {k: re.findall(r'frame=(\d+)', ' '.join(v[-1:])) for k, v in ends.items()}
    lines = _plines(ctx, st['pilot'])
    inputs = [P1_INPUTS.search(l).groups() for l in lines if P1_INPUTS.search(l)]
    out = {'sync_end': ends, 'frames': frames, 'p1_inputs_in_match': inputs, 'pilot_lines': lines[-30:]}
    steps._save(ctx, 'pp-%s-end' % key, out)
    problems = []
    if any(not v for v in ends.values()):
        problems.append('no sync_end on both: %s' % ends)
    if any('desync_seen=0' not in v[-1] for v in ends.values() if v):
        problems.append('desync: %s' % ends)
    if len({tuple(v) for v in frames.values()}) != 1:
        problems.append('sync_end frames differ: %s' % frames)
    if problems:
        raise RuntimeError('; '.join(problems))
    return {'sync_end': {k: v[-1] for k, v in ends.items()}, 'p1_inputs_in_match': inputs[-2:]}


def rearmed(ctx, key='back', cap_s=60.0):
    """b back in the waiting practice after the match, the setting still CPU: the pilot arms again on slot 0 in the
    practice (`scene scope=WaitingPractice`, then `arm … slot=0`) and drives."""
    start = _state(ctx)[key]['pilot']
    began, armed = time.monotonic(), None
    while time.monotonic() - began < cap_s and not armed:
        ctx.check_stop()
        lines = _plines(ctx, start)
        practice = [i for i, l in enumerate(lines) if 'scene scope=WaitingPractice' in l]
        if practice:
            armed = next((l for l in lines[practice[-1]:] if ARM.search(l)), None)
        time.sleep(1.0)
    lines = _plines(ctx, start)
    out = {'armed': armed, 'pilot_lines': lines[-20:], 'shot': str(ctx.shot('pp-%s-rearmed' % key))}
    steps._save(ctx, 'pp-%s-rearmed' % key, out)
    if not armed or ARM.search(armed).group(4) != '0':
        raise RuntimeError('not re-armed on slot 0 in the practice: %s' % lines[-8:])
    return {'armed': armed}


def disarm_records(ctx, presses=3, close=2):
    """b: the P1 row back to Player (ZR + Right on `CPU (Ln)` toggles to Player; ZR + Left would also be TrainingRooms'
    Play on Player 1): the pilot releases on the live practice objects (`release restored=true`), and TrainingRooms'
    Record then starts a capture (`recording=1`); a second chord stops it."""
    start = _size(_plog(ctx))
    for i in range(presses):
        _play(ctx, 'pilot-page-step')
        time.sleep(1.3)
    _play(ctx, 'pilot-arm')
    time.sleep(1.3)
    ctx.shot('pp-disarm-set')
    for i in range(close):
        _play(ctx, 'pilot-page-step')
        time.sleep(1.3)
    _tap(ctx, 'b')
    time.sleep(1.0)
    rel = _wait_plog(ctx, start, RELEASE.pattern, 15)
    time.sleep(2.0)
    mark = r2._mark(ctx)
    _play(ctx, 'chord-record')
    began = r2._wait_line(ctx, mark, r'screen=recording role=\w+ recording=1', 10)
    time.sleep(2.0)
    _play(ctx, 'chord-record')
    stopped = r2._wait_line(ctx, mark, r'screen=recording role=\w+ recording=0', 10)
    out = {'release': rel, 'ini': _ini(ctx)[:120], 'record_started': began, 'record_stopped': stopped,
           'shot': str(ctx.shot('pp-disarm-records'))}
    steps._save(ctx, 'pp-disarm-records', out)
    if not rel or RELEASE.search(rel).group(1) != 'true':
        raise RuntimeError('no release on the live practice objects: %s' % rel)
    if not began:
        raise RuntimeError('Record did not start with the pilot off')
    return {'release': rel, 'record_started': began[:160], 'record_stopped': (stopped or '')[:160]}
