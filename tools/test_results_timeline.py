"""Host tests for results_timeline.py: offsets, the save-chain verdict and the rematch outcome."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('results_timeline', Path(__file__).with_name('results_timeline.py'))
timeline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(timeline)

B = 1_000_000_000
ONLINE = '\n'.join(f'{B + int(s * 19_200_000)} results epoch=0x10 port=0 {e} status={st} popup=0x0 phase=0' for s, e, st in [
    (0, 'epoch=begin step=Idle', 0),
    (2.0, 'save=press_a_splash n=1 step=Cards', 0),
    (7.1, 'cards=sample n=1 route=1 offered=1', 0),
    (7.2, 'save=press_y step=Cards', 0),
    (7.5, 'save=select_yes step=SaveDialog', 1),
    (7.7, 'save=confirm step=SelectYes', 1),
    (7.9, 'save=writing step=Confirm', 2),
    (8.1, 'save=close_success step=Saving', 3),
    (8.3, 'save=completed step=Close', 5),
    (22.2, 'rematch=press_a step=Rematch', 5),
    (22.25, 'rematch=accepted step=Answer', 5),
]) + '\n1 plugin=ssbu-cpu-pilot/0.2.0 result=installed\n'


class TimelineTests(unittest.TestCase):
    def test_offsets_and_a_complete_online_epoch(self):
        out = timeline.report(ONLINE)
        self.assertIn('epoch=0x10 begin=1000000000', out)
        self.assertIn('+  2.00 s save=press_a_splash n=1 step=Cards status=0', out)
        self.assertIn('+  8.30 s save=completed step=Close status=5', out)
        self.assertTrue(out.endswith('save=ok rematch=accepted'))

    def test_missing_reordered_failed_or_late_splash_is_not_a_save(self):
        bad = (ONLINE.replace('save=writing', 'save=other'),
               ONLINE.replace('save=confirm step', 'save=x step').replace('save=select_yes', 'save=confirm'),
               ONLINE.replace('step=Close status=5', 'step=Close status=4'),
               ONLINE + f'\n{B + 9 * 19_200_000} results epoch=0x10 port=0 save=press_a_splash n=2 step=Cards status=0')
        for text in bad:
            with self.subTest(text=text[-80:]):
                self.assertIn('save=no', timeline.report(text))

    def test_rematch_outcomes_and_separate_epochs(self):
        timed_out = ONLINE.replace('rematch=accepted step=Answer', 'flow=timeout step=Rematch')
        self.assertTrue(timeline.report(timed_out).endswith('rematch=flow=timeout'))
        save_timeout = ONLINE.split('\n')[0] + f'\n{B + 21 * 19_200_000} results epoch=0x10 port=0 flow=timeout step=Cards status=0'
        self.assertTrue(timeline.report(save_timeout).endswith('save=no rematch=none'))
        two = ONLINE + '\n' + ONLINE.replace('epoch=0x10', 'epoch=0x20')
        self.assertEqual(timeline.report(two).count('save=ok rematch=accepted'), 2)
        # A line of an epoch whose begin was not logged is ignored.
        self.assertEqual(timeline.report(ONLINE.split('\n', 1)[1]), '')


if __name__ == '__main__':
    unittest.main()
