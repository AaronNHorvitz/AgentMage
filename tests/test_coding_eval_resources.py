"""Inherited lane accounting, counter integrity and refusal contracts."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import unittest
from unittest import mock

from scripts import coding_eval_resources as resources


class CodingResourceTests(unittest.TestCase):
    def before(self):
        return {'identity': (10, 20, Path('/private/synthetic-lane')),
                'limits': {'memory.max': resources.MEMORY_MAX},
                'cpu': {'usage_usec': 100, 'user_usec': 60, 'system_usec': 40},
                'events': {'max': 2, 'oom': 0, 'oom_kill': 0},
                'memory_current_bytes': 10000, 'memory_peak_lifetime_bytes': 30000}

    def test_measurement_accounts_children_without_publishing_the_private_identity(self):
        before = self.before()
        after = deepcopy(before)
        after['cpu'].update(usage_usec=150, user_usec=90, system_usec=60)
        after['memory_current_bytes'] = 9000
        after['memory_peak_lifetime_bytes'] = 40000
        with mock.patch.object(resources, 'snapshot', side_effect=(before, after)):
            with resources.measurement(required=True) as report:
                self.assertFalse(report['verified'])
        self.assertTrue(report['verified'])
        self.assertEqual(report['cpu'], {'usage_usec': 50, 'user_usec': 30, 'system_usec': 20})
        self.assertEqual(report['memory_peak_lifetime_bytes'], 40000)
        self.assertFalse(report['memory_peak_is_task_peak'])
        self.assertEqual(report['scope'], 'entire-inherited-lane')
        self.assertTrue(report['includes_trusted_setup_and_other_lane_processes'])
        self.assertNotIn('private', json.dumps(report))

    def test_required_measurement_refuses_before_execution_if_no_bounded_lane_exists(self):
        with mock.patch.object(resources, 'snapshot', side_effect=OSError('private path')):
            with self.assertRaisesRegex(resources.ResourceRefusal, '^resources.unavailable-or-unbounded$'):
                with resources.measurement(required=True):
                    self.fail('unbounded operation started')
            with resources.measurement() as report:
                pass
            self.assertFalse(report['verified'])
            self.assertNotIn('private path', json.dumps(report))

    def test_scope_limit_and_counter_changes_prevent_verified_publication(self):
        for field, change in (('identity', (10, 21, Path('/private/synthetic-lane'))),
                              ('limits', {'memory.max': 2 * resources.MEMORY_MAX}),
                              ('cpu', {'usage_usec': 90, 'user_usec': 60, 'system_usec': 40}),
                              ('events', {'max': 1, 'oom': 0, 'oom_kill': 0}),
                              ('memory_peak_lifetime_bytes', 20000)):
            before = self.before()
            after = {**deepcopy(before), field: change}
            with self.subTest(field=field), mock.patch.object(resources, 'snapshot', side_effect=(before, after)):
                with self.assertRaisesRegex(resources.ResourceRefusal, '^resources.final-observation-refused$'):
                    with resources.measurement(required=True) as report:
                        pass
                self.assertFalse(report['verified'])

    def test_interruption_survives_failed_final_observation(self):
        with mock.patch.object(resources, 'snapshot', side_effect=(self.before(), OSError('private path'))):
            with self.assertRaises(KeyboardInterrupt):
                with resources.measurement(required=True) as report:
                    raise KeyboardInterrupt
        self.assertFalse(report['verified'])
        self.assertEqual(report['error'], 'resources.final-observation-refused')

    def test_unbounded_or_over_allocation_limits_and_malformed_counters_are_refused(self):
        group = mock.MagicMock()
        group.stat.return_value.st_dev = 1
        group.stat.return_value.st_ino = 2
        files = {'memory.max': str(resources.MEMORY_MAX), 'memory.swap.max': str(resources.SWAP_MAX),
                 'pids.max': str(resources.PIDS_MAX), 'cpu.max.burst': '0', 'cpu.max': '400000 100000',
                 'cpu.stat': 'usage_usec 100\nuser_usec 60\nsystem_usec 40\n',
                 'memory.events': 'max 0\noom 0\noom_kill 0\n',
                 'memory.current': '100', 'memory.peak': '200'}
        group.__truediv__.side_effect = lambda key: key
        variants = (('memory.max', 'max'), ('memory.max', str(resources.MEMORY_MAX + 1)),
                    ('memory.swap.max', str(resources.SWAP_MAX + 1)), ('pids.max', 'max'),
                    ('pids.max', str(resources.PIDS_MAX + 1)), ('cpu.max', 'max 100000'),
                    ('cpu.max', '400001 100000'), ('cpu.max', '1 0'), ('cpu.max.burst', '1'),
                    ('cpu.stat', 'usage_usec 2\nusage_usec 1\n'),
                    ('memory.events', 'max 0\noom -1\noom_kill 0\n'))
        for key, value in variants:
            data = {**files, key: value}
            with self.subTest(field=key, value=value), \
                    mock.patch.object(resources, 'current_group', return_value=group), \
                    mock.patch.object(resources, 'bounded_text', side_effect=lambda path: data[path]):
                with self.assertRaises(resources.ResourceRefusal):
                    resources.snapshot()

    def test_real_inherited_read_only_limits_account_a_child_process(self):
        with resources.measurement(required=True) as report:
            subprocess.run(['/usr/bin/python3', '-I', '-B', '-c',
                            'data = bytearray(8 * 1024 * 1024)\n'
                            'for _ in range(1000000): x = 7 * 13\n'], check=True)
        self.assertTrue(report['verified'], report)
        self.assertGreater(report['cpu']['usage_usec'], 0)
        self.assertLessEqual(report['limits']['memory.max'], resources.MEMORY_MAX)
        self.assertLessEqual(report['limits']['cpu_quota_usec'], 4 * report['limits']['cpu_period_usec'])
        self.assertFalse(report['memory_peak_is_task_peak'])


if __name__ == '__main__':
    unittest.main()
