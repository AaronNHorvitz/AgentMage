"""Read inherited Linux lane quotas and counters without changing any limit.

The scope is the whole current cgroup, including the trusted builder and setup,
not a measurement of candidate processes alone. No delegation is required.
"""
from __future__ import annotations

from contextlib import contextmanager
import os
from pathlib import Path
import stat


VERSION = "coding-inherited-lane-resources-v1"
MEMORY_MAX = 16 * 1024**3
SWAP_MAX = 1024**3
CPU_MAX = 4
PIDS_MAX = 1024


class ResourceRefusal(ValueError):
    """Fixed diagnostic; private cgroup paths never enter published reports."""


def bounded_text(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ResourceRefusal("resources.file-refused")
        data = stream.read(16385)
    if len(data) > 16384:
        raise ResourceRefusal("resources.file-refused")
    return data.decode("ascii")


def current_group():
    memberships = [line[3:] for line in bounded_text(Path('/proc/self/cgroup')).splitlines()
                   if line.startswith('0::')]
    if len(memberships) != 1:
        raise ResourceRefusal("resources.membership-refused")
    relative = memberships[0]
    if (not relative.startswith('/') or any(part in ('.', '..') for part in relative.split('/'))):
        raise ResourceRefusal("resources.membership-refused")
    root = Path('/sys/fs/cgroup').resolve(strict=True)
    group = (root / relative.lstrip('/')).resolve(strict=True)
    if not group.is_relative_to(root) or not os.statvfs(group).f_flag & os.ST_RDONLY:
        raise ResourceRefusal("resources.mount-refused")
    return group


def natural(value):
    if not value.isascii() or not value.isdecimal() or len(value) > 20:
        raise ResourceRefusal("resources.number-refused")
    return int(value)


def counters(text, required):
    fields = {}
    for line in text.splitlines():
        pair = line.split()
        if len(pair) != 2 or pair[0] in fields:
            raise ResourceRefusal("resources.counters-refused")
        fields[pair[0]] = natural(pair[1])
    if not set(required).issubset(fields):
        raise ResourceRefusal("resources.counters-refused")
    return {key: fields[key] for key in required}


def snapshot():
    group = current_group()
    info = group.stat()
    limits = {name: natural(bounded_text(group / name).strip())
              for name in ('memory.max', 'memory.swap.max', 'pids.max', 'cpu.max.burst')}
    cpu = bounded_text(group / 'cpu.max').split()
    if len(cpu) != 2:
        raise ResourceRefusal("resources.cpu-refused")
    quota, period = map(natural, cpu)
    if (not 0 < limits['memory.max'] <= MEMORY_MAX or limits['memory.swap.max'] > SWAP_MAX
            or not 0 < limits['pids.max'] <= PIDS_MAX or limits['cpu.max.burst'] != 0
            or period == 0 or not 0 < quota <= CPU_MAX * period):
        raise ResourceRefusal("resources.quota-refused")
    limits.update(cpu_quota_usec=quota, cpu_period_usec=period)
    return {
        # Kept in memory only; no private cgroup identity is serialized.
        'identity': (info.st_dev, info.st_ino, group),
        'limits': limits,
        'cpu': counters(bounded_text(group / 'cpu.stat'), ('usage_usec', 'user_usec', 'system_usec')),
        'events': counters(bounded_text(group / 'memory.events'), ('max', 'oom', 'oom_kill')),
        'memory_current_bytes': natural(bounded_text(group / 'memory.current').strip()),
        'memory_peak_lifetime_bytes': natural(bounded_text(group / 'memory.peak').strip()),
    }


def difference(before, after):
    if before['identity'] != after['identity'] or before['limits'] != after['limits']:
        raise ResourceRefusal("resources.scope-or-quota-changed")
    values = {}
    for field in ('cpu', 'events'):
        values[field] = {key: after[field][key] - value for key, value in before[field].items()}
        if any(value < 0 for value in values[field].values()):
            raise ResourceRefusal("resources.counter-regressed")
    if after['memory_peak_lifetime_bytes'] < before['memory_peak_lifetime_bytes']:
        raise ResourceRefusal("resources.counter-regressed")
    return {**values, 'limits': before['limits'],
            'memory_current_before_bytes': before['memory_current_bytes'],
            'memory_current_after_bytes': after['memory_current_bytes'],
            'memory_peak_lifetime_bytes': after['memory_peak_lifetime_bytes'],
            'memory_peak_is_task_peak': False}


@contextmanager
def measurement(*, required=False):
    """Observe setup, execution and cleanup; interruption always propagates.

    Historical controls may report unavailable accounting. Generated admission
    must require verification; this module grants no admission on its own.
    """
    report = {'version': VERSION, 'scope': 'entire-inherited-lane',
              'includes_trusted_setup_and_other_lane_processes': True,
              'verified': False, 'error': None}
    before = None
    try:
        before = snapshot()
    except (OSError, ValueError):
        report['error'] = 'resources.unavailable-or-unbounded'
        if required:
            raise ResourceRefusal(report['error']) from None
    failed = False
    try:
        yield report
    except BaseException:
        failed = True
        raise
    finally:
        if before is not None:
            try:
                report.update(difference(before, snapshot()), verified=True)
            except (OSError, ValueError):
                report['error'] = 'resources.final-observation-refused'
                if required and not failed:
                    raise ResourceRefusal(report['error']) from None
