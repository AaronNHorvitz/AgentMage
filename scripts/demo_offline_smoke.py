#!/usr/bin/env python3
"""Real inference through Application in a scoped, networkless backend namespace."""
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
BASE = Path(os.environ.get('XDG_STATE_HOME', str(Path.home() / '.local/state'))).resolve() / 'agentmage-demo'


def main():
    os.umask(0o077)
    if '--inside' not in sys.argv:
        BASE.mkdir(parents=True, exist_ok=True, mode=0o700)
        scope = BASE / 'offline-scope'
        scope.mkdir(exist_ok=True, mode=0o700)
        env = dict(os.environ, XDG_STATE_HOME=str(scope))
        cmd = ['/usr/bin/bwrap', '--unshare-net', '--die-with-parent', '--ro-bind', '/', '/',
               '--bind', str(BASE), str(BASE), '--dev-bind', '/dev', '/dev', '--proc', '/proc',
               sys.executable, str(Path(__file__).resolve()), '--inside']
        p = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=180)
        if p.returncode != 0:
            print(p.stdout + p.stderr, file=sys.stderr)
            raise SystemExit(p.returncode)
        report = json.loads(p.stdout)
        files = ['Cargo.toml', 'Cargo.lock', 'shells/host/Cargo.toml', 'shells/host/src/lib.rs', 'supply-chain/sbom.cdx.json', 'supply-chain/dependency-provenance.json', 'supply-chain/dependency-hashes.sha256', 'scripts/demo.py', 'scripts/demo_offline_smoke.py', 'demo/model.json', 'shells/host/src/demo_documents.rs']
        report['source_sha256'] = {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in files}
        (ROOT / 'docs/verification/demo-offline-acceptance.json').write_text(json.dumps(report, indent=2) + '\n')
        print('PASS: backend and real model inference without external networking')
        return
    import demo
    demo.private_state()
    app = demo.Application(json.loads(demo.CONFIG.read_text()))
    start = time.monotonic()
    try:
        network = {'interfaces': socket.if_nameindex(), 'namespace': os.readlink('/proc/self/ns/net')}
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.settimeout(1)
        try:
            probe.connect(('1.1.1.1', 443))
        except OSError as e:
            network['external_connect_blocked'] = True
            network['external_connect_error'] = str(e)
        else:
            raise RuntimeError('Scoped namespace unexpectedly permits external networking')
        finally:
            probe.close()
        app.model.start()
        until = time.monotonic() + 120
        while not app.model.ready():
            if not app.model.starting:
                raise RuntimeError(app.model.error)
            if time.monotonic() > until:
                raise RuntimeError('Isolated model startup timed out')
            time.sleep(.25)
        admission = app.operation('ingest', {'folder': str(ROOT / 'demo/fixtures/aurora')})
        job = app.operation('ask', {'question': 'When does Aurora launch, and who leads it?'})
        until = time.monotonic() + 90
        while True:
            result = app.operation('result', {'job': job['job']})
            if result['done']:
                break
            if time.monotonic() > until:
                raise RuntimeError('Isolated inference timed out')
            time.sleep(.1)
        if result.get('error'):
            raise RuntimeError(result['error'])
        assert '18 October 2026' in result['answer'] and 'Mira Chen' in result['answer'], result
        assert result['citations'], result
        print(json.dumps({'record_type': 'agentmage_scoped_offline_demo_acceptance', 'passed': True,
                          'network': network, 'admitted_files': admission['accepted_count'],
                          'real_application_result': result, 'total_seconds': round(time.monotonic()-start, 3),
                          'limits': ['Scoped backend/model network namespaces only; host network configuration untouched.',
                                     'Uses same Application operation and Rust document runtime as browser backend.']}))
    finally:
        app.close()


if __name__ == '__main__':
    main()
