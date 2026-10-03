"""Apply each mutation alone, run its package filter, record the failing tests, restore the file."""
import json, subprocess, sys
from pathlib import Path
ROOT = Path('<repo>')
spec = json.loads(Path(sys.argv[1]).read_text())
results = []
for m in spec:
    path = ROOT / m['file']
    original = path.read_text()
    text = original
    for old, new in m['replace']:
        assert text.count(old) == 1, (m['id'], old[:60], text.count(old))
        text = text.replace(old, new)
    path.write_text(text)
    try:
        features = ['--features', m['features']] if m.get('features') else []
        proc = subprocess.run(['cargo', 'test', '--locked', '--offline', '-p', m['package'], *features, '--lib', '--', *m['filters']],
                              cwd=ROOT, capture_output=True, text=True, timeout=3000)
        out = proc.stdout + proc.stderr
        (Path('<state>/batch31-mutants') / f"{m['id']}.log").write_text(out)
        failed = sorted(line[5:-len(' ... FAILED')] for line in out.splitlines() if line.startswith('test ') and line.endswith(' ... FAILED'))
        compile_error = 'error[' in out or 'could not compile' in out
        caught = m['expected'] in failed
        results.append({'id': m['id'], 'expected': m['expected'], 'caught': caught, 'failed': failed, 'compile_error': compile_error})
        print(m['id'], 'CAUGHT' if caught else ('COMPILE-ERROR' if compile_error else 'SURVIVED'), failed, flush=True)
    finally:
        path.write_text(original)
Path(sys.argv[2]).write_text(json.dumps(results, indent=2) + '\n')
