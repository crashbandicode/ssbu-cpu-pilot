"""The per-update count is per window, per slot, and does not depend on the host's frame rate."""
import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('trace_rate', Path(__file__).with_name('trace_rate.py'))
rate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rate)


def hit(t, point, place, x23=None, value=None):
    record = {'event': 'entry', 'point_id': point, 'wall_time_ns': str(t), 'locations': {'0x1': place}}
    if x23 is not None:
        record['registers'] = {'x23': hex(x23)}
    if value is not None:
        record['captures'] = [{'expression': 'x19', 'type': 'u64', 'value': str(value)}]
    return json.dumps(record)


def window(start, updates, entries_per_update, update_point, entry_point, step_ns, active_every=2):
    lines = []
    for i in range(updates):
        t = start + i * step_ns
        lines.append(hit(t, update_point, rate.UPDATE))
        for k in range(entries_per_update):
            lines.append(hit(t + 1 + k, entry_point, rate.ENTRY, x23=0))
            value = 0x000000c400008000 if i % active_every == 0 else 0x8000
            lines.append(hit(t + 1 + k, 'm' + entry_point, rate.MAPPED, value=value))
    return lines


class TraceRateTests(unittest.TestCase):
    def test_once_per_update_at_any_frame_rate(self):
        slow = window(0, 100, 1, 'u1', 'e1', 50_000_000)       # 20 updates/s
        fast = window(10**10, 300, 1, 'u2', 'e2', 16_666_667)  # 60 updates/s
        result = rate.windows(slow + fast)
        self.assertEqual([w['slot0_per_update'] for w in result], [1.0, 1.0])
        self.assertEqual([w['updates'] for w in result], [99, 299])
        self.assertEqual(result[0]['mapped_active'], 50)

    def test_a_doubled_entry_shows(self):
        result = rate.windows(window(0, 100, 2, 'u1', 'e1', 16_666_667))
        self.assertEqual(result[0]['slot0_per_update'], 2.0)

    def test_hits_outside_a_window_and_other_slots_are_apart(self):
        lines = window(0, 10, 1, 'u1', 'e1', 16_666_667)
        lines.append(hit(10**12, 'e9', rate.ENTRY, x23=0))
        lines.append(hit(5, 'e1', rate.ENTRY, x23=1))
        result = rate.windows(lines)
        self.assertEqual(result[0]['entry_by_slot'], {'0': 9, '1': 1})
        self.assertEqual(result[0]['slot0_per_update'], 1.0)

    def test_tap_jump_flag_alone_is_not_active_input(self):
        result = rate.windows(window(0, 10, 1, 'u1', 'e1', 16_666_667, active_every=10**6))
        self.assertEqual(result[0]['mapped_active'], 1)


if __name__ == '__main__':
    unittest.main()
