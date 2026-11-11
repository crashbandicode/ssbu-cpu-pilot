"""A waiting-room run passes only with the practice armed on slot 0, a clean handover and a synchronized match."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('lobby_report', Path(__file__).with_name('lobby_report.py'))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)

PILOT = '''100 plugin=ssbu-cpu-pilot/0.2.3 level=3 result=installed heap_allocations=0 menu=registered
200 scene scope=WaitingPractice slot=0
210 gate ready armed=0
300 arm level=3 rank=21 old_rank=100 slot=0
400 p1_inputs calls=600 last_input=0x000000da00008000
19200400 p1_inputs calls=1200 last_input=0x0000000000008000
19300000 stale no_battle_frames armed_cleared=1
19400000 scene scope=Online1v1 slot=0
19400001 handover from=WaitingPractice to=Online1v1
19400002 release restored=false old_rank=100
19500000 online stage_start slot=0 stage=347 phase=1
19500001 online stage=Ready phase=2 rng_restored=1 d0_restored=1
19500002 arm level=3 rank=21 old_rank=100 slot=0
19600000 p1_inputs calls=1800 last_input=0x00005e4b00008802
38800000 p1_inputs calls=2400 last_input=0x0000000000008000
40000000 stale no_battle_frames armed_cleared=1
40000001 release restored=false old_rank=100
41000000 scene scope=WaitingPractice slot=0
41000002 arm level=3 rank=21 old_rank=100 slot=0
'''
LOCAL = '''19450000 session slot=0 kind=online heap_allocations=0
39000000 sync_end trigger=normal kind=1 frame=7521 local=0 desync_seen=0 already_ending=0 caller=0x1569718
'''
PEER = '''5 session slot=1 kind=online heap_allocations=0
9 sync_end trigger=normal kind=1 frame=7521 local=1 desync_seen=0 already_ending=0 caller=0x1569718
'''


class LobbyReportTests(unittest.TestCase):
    def test_the_practice_drives_and_hands_over(self):
        result = report.summarize(PILOT, LOCAL, PEER)
        self.assertEqual(result['battle']['frame'], 7521)
        self.assertEqual(result['practice']['inputs']['nonzero_samples'], 1)
        self.assertEqual(result['practice']['inputs']['calls_per_s'], [600.0, 600.0])
        self.assertIn('handover from=WaitingPractice to=Online1v1', result['handover']['handover'])
        self.assertIn('restored=false', result['handover']['practice_release'])
        self.assertIn('slot=0', result['back']['re_arm'])

    def test_a_handover_line_is_not_required_when_the_practice_ended_first(self):
        early = PILOT.replace('19400001 handover from=WaitingPractice to=Online1v1\n', '').replace(
            '19400002 release restored=false', '19350000 release restored=false')
        self.assertIsNone(report.summarize(early, LOCAL, PEER)['handover']['handover'])

    def test_failures(self):
        bad_pilot = (
            PILOT.replace('result=installed', 'result=not_installed'),
            PILOT.replace('heap_allocations=0', 'heap_allocations=2'),
            PILOT.replace('300 arm level=3 rank=21 old_rank=100 slot=0', '300 arm level=3 rank=21 old_rank=100 slot=1'),
            PILOT.replace('300 arm', '300 gate'),
            PILOT.replace('19400002 release restored=false', '19400002 release restored=true'),
            PILOT.replace('19400002 release restored=false old_rank=100\n', ''),
            PILOT.replace('handover from=WaitingPractice', 'handover from=Offline'),
            PILOT.replace('19500002 arm', '19500002 gate'),
            PILOT.replace('0x00005e4b00008802', '0x0000000000008000'),
            PILOT.replace('0x000000da00008000', '0x0000000000008000'),
            PILOT + '30000000 release restored=false old_rank=100\n',
            PILOT.replace('19500001 online stage=Ready', '19500001 online stage=Pending'),
        )
        for bad in bad_pilot:
            with self.subTest(pilot=bad.splitlines()[:1]), self.assertRaises(ValueError):
                report.summarize(bad, LOCAL, PEER)
        for bad in (PEER.replace('7521', '7522'), PEER.replace('desync_seen=0', 'desync_seen=1'),
                    PEER.replace('local=1', 'local=0'), PEER.replace('normal', 'disconnect'), PEER + PEER):
            with self.subTest(peer=bad), self.assertRaises(ValueError):
                report.summarize(PILOT, LOCAL, bad)


if __name__ == '__main__':
    unittest.main()
