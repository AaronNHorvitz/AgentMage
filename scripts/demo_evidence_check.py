#!/usr/bin/env python3
"""Verify current bindings and actual recorded acceptance outcomes; never writes reports."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ['demo-browser-acceptance.json', 'demo-offline-acceptance.json',
           'demo-restarted-browser-acceptance.json', 'demo-restart-acceptance.json']


def main():
    for name in REPORTS:
        report = json.loads((ROOT / 'docs/verification' / name).read_text())
        if 'tests' in report:
            assert report['tests'] and all(t['passed'] for t in report['tests']), name + ': acceptance failure'
            assert report['real_model_interactions'], name + ': no real model interactions'
        else:
            assert report['passed'], name + ': failed'
        for file, expected in report['source_sha256'].items():
            assert hashlib.sha256((ROOT / file).read_bytes()).hexdigest() == expected, name + ': stale input ' + file
        if 'network' in report:
            assert report['network']['external_connect_blocked'], 'Offline inference not proven'
            assert '18 October 2026' in report['real_application_result']['answer']
        print('PASS: current ' + name)


if __name__ == '__main__':
    main()
