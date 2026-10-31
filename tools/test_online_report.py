"""Negative evidence must never become an online acceptance pass."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('online_report', Path(__file__).with_name('online_report.py'))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

PILOT = '''1 plugin=ssbu-cpu-pilot/0.1.0 result=installed heap_allocations=0
3 online stage_start slot=1 stage=1 phase=1
4 online stage=Ready phase=2 rng_restored=1 d0_restored=1
5 arm level=9 rank=100 old_rank=100 slot=1
6 p1_inputs calls=600 last_input=0x0000008800008201
'''
SURFACE = '''2 session slot=0 kind=online heap_allocations=0
9 sync_end trigger=normal kind=1 frame=1493 local=1 desync_seen=0 already_ending=0 caller=0x1569718
'''
YOGA = SURFACE.replace('local=1', 'local=0')


class OnlineReportTests(unittest.TestCase):
    def test_complete_asymmetric_match(self):
        result = report.summarize(PILOT, SURFACE, YOGA)
        self.assertEqual((result['result'], result['frame'], result['pilot_slot']), ('pass', 1493, 1))

    def test_detector_failures(self):
        for bad in (YOGA.replace('1493', '1494'), YOGA.replace('desync_seen=0', 'desync_seen=1'),
                    YOGA.replace('normal', 'disconnect'), YOGA.replace('local=0', 'local=1'),
                    YOGA.replace('1493', '0'), YOGA.splitlines()[0], YOGA + YOGA,
                    YOGA.replace('kind=online', 'kind=replay')):
            with self.subTest(log=bad), self.assertRaises(ValueError):
                report.summarize(PILOT, SURFACE, bad)

    def test_install_only_or_incomplete_pilot_does_not_pass(self):
        for bad in (PILOT.splitlines()[0], PILOT.replace('phase=2', 'phase=1'),
                    PILOT.replace('rng_restored=1', 'rng_restored=0'),
                    PILOT.replace('0x0000008800008201', '0x0000000000000000'),
                    PILOT.replace('5 arm', '10 arm'), PILOT + '7 release restored=true\n',
                    PILOT.replace('result=installed', 'result=not_installed'),
                    PILOT.replace('heap_allocations=0', 'heap_allocations=1')):
            with self.subTest(log=bad), self.assertRaises(ValueError):
                report.summarize(bad, SURFACE, YOGA)

    def test_peer_end_is_not_a_sync_end(self):
        with self.assertRaises(ValueError):
            report.summarize(PILOT, SURFACE, YOGA.replace('sync_end', 'peer_end'))

    def test_release_after_end_is_allowed(self):
        self.assertEqual(report.summarize(PILOT + '10 release restored=false\n', SURFACE, YOGA)['result'], 'pass')

    def test_unsampled_short_match_is_explicitly_distinguished(self):
        short = '\n'.join(line for line in PILOT.splitlines() if 'p1_inputs' not in line)
        with self.assertRaises(ValueError):
            report.summarize(short, SURFACE, YOGA)
        result = report.summarize(short, SURFACE, YOGA, allow_unsampled_input=True)
        self.assertEqual(result['result'], 'detector_pass_input_unsampled')
        self.assertFalse(result['nonzero_input_sampled'])
        with self.assertRaises(ValueError):
            report.summarize(PILOT.replace('0x0000008800008201', '0x0'), SURFACE, YOGA,
                             allow_unsampled_input=True)


if __name__ == '__main__':
    unittest.main()
