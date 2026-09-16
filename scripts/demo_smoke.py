#!/usr/bin/env python3
"""Repeatable full demo acceptance; leaves a restarted demo running."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def main():
    run(sys.executable, 'scripts/demo.py', 'start', '--no-open')
    run('node', 'scripts/demo_browser_smoke.mjs')
    run(sys.executable, 'scripts/demo.py', 'stop')
    run(sys.executable, 'scripts/demo_offline_smoke.py')
    run(sys.executable, 'scripts/demo.py', 'start', '--no-open')
    # A second fresh browser run proves successful inference after complete stop/restart.
    run('node', 'scripts/demo_browser_smoke.mjs', '--restart-only')
    from demo import STATE
    launch = json.loads((STATE / 'launch.json').read_text())
    report = {'record_type': 'agentmage_demo_stop_restart_acceptance', 'passed': True,
              'commands': ['python3 scripts/demo.py start --no-open', 'node scripts/demo_browser_smoke.mjs',
                           'python3 scripts/demo.py stop', 'python3 scripts/demo_offline_smoke.py',
                           'python3 scripts/demo.py start --no-open', 'node scripts/demo_browser_smoke.mjs --restart-only'],
              'final_running_pid': launch['pid'], 'recorded_at_unix': time.time(),
              'source_sha256': {'scripts/demo_smoke.py': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
              'acceptance_reports': ['docs/verification/demo-browser-acceptance.json',
                                     'docs/verification/demo-offline-acceptance.json',
                                     'docs/verification/demo-restarted-browser-acceptance.json']}
    (ROOT / 'docs/verification/demo-restart-acceptance.json').write_text(json.dumps(report, indent=2)+'\n')
    print('PASS: full real-model browser, offline isolation, stop and restart. Demo remains running.')


if __name__ == '__main__':
    main()
