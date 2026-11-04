# T-311.18 protocol lifecycle and launcher arguments

Status: builder implementation checks passed; independent exact-commit core review
required. Implementation base: `9a21c0d57bd7111c2d65a7a8ae749128457cc802`.
This row and parent T-311 remain open.

## Scope

Address T-311.17 review lows L1/L2/L3 in oracle v15. Before importing candidate
code, duplicate the protocol writer to a non-inheritable descriptor and redirect
fd 1 to diagnostics. Ordinary output, original stdout objects, raw fd 1 writes
and exec children therefore cannot substitute a protocol reply through stdout.
After assertions, send a closed end-of-requests frame and require clean EOF.
Extra bytes remain a sticky failure even when the original assertions passed;
a held pipe remains bounded by the existing host execution deadline.

Use anonymous NUL-separated launcher option transport. Namespace PID 1's argv
contains only the launcher, descriptor number and fixed namespace-local worker
command. Close the argument descriptor in the launcher and host. Namespace,
mount, environment and resource options remain unchanged. This addresses argv
exposure, without claiming every source of host filesystem metadata is hidden.

Document that diagnostic frame bounds count JSON-encoded bytes, including
escaping and newline, rather than characters. The
[guide](../guides/coding-evaluation-oracle.md) also states the remaining
same-interpreter limit: deliberate descriptor discovery or worker patching can
still control candidate observations. Hidden assertions, counts and output
remain in the separate trusted process. Generated scoring stays disabled.

Existing builder-authored malformed-wire probes now inject through the private
descriptor to preserve their original security purpose. Separate new probes
exercise standard output isolation. No frozen hidden method, task pin,
selection or threshold changes. T-311.15/T-311.16 cleanup functions remain
unchanged; this batch does not restart their three-strike review cycle.

## Verification

All implementation checks below ran through the shared build reservation. Times
exclude reservation waits and include command overhead.

| Check | Result | Seconds |
| --- | --- | --- |
| Targeted security/lifecycle checks | PASS; 11 methods | 6.242811 |
| Exact transcript and direct-worker fixture checks | PASS; two methods | 8.984046 |
| Scoped fast gate | PASS; 115 unit/integration tests | 72.601648 |
| Legacy unit suites | PASS; 58 tests | 20.358967 |
| Independent verifier | PASS; 15 pairs, 18 original methods, 160 reference calls | 8.003399 |
| Unchanged legacy verifier | PASS; 30 pairs, 89 reference tests, no skips | 44.565864 |
| Persistent protocol probes | PASS; 12 controls across four methods | 4.571826 |
| Frozen-input and report audit | PASS; 102 reports and 246 log bindings | 1.498364 |
| Policy validator | PASS; 718 Markdown files and all policy invariants | 146.429072 |

All fifteen independent bases fail their exact expected profiles, and all fifteen
references pass: 30 control attempts, 15 passed and 15 failed. All thirty legacy
pairs discriminate. The collection digest remains
`0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570`.
The six legacy source/test files, all thirty frozen task pins, the old ledger
suffix, retained review bytes and deferred cleanup/mode functions are unchanged.
No cleanup recovery record was produced. Source/test bytes stayed unchanged
after the passing fast gate.

Initial targeted failures and the separate
launcher diagnostic are retained privately: the anonymous option transport
initially included the executable, so the launcher emitted usage and neither
worker started. The correction keeps the fixed command after the option
delimiter. No test or threshold was weakened to accept the failure.

The first full fast gate ran 115 tests and failed two builder test fixtures.
The launch-failure transcript now contains the trusted end marker; its assertion
now requires those exact bytes while retaining zero assertions/calls and a
counted infrastructure failure. The direct worker test now mocks duplication,
fd wrapping and redirection instead of modifying the test runner's own output
descriptors. It still requires the exact ready frame, protocol refusal and zero
helper calls. Real sandbox probes independently exercise actual descriptors.
The failed gate and its source digests are retained separately.

No model, GPU, held-out task, prompt tuning, model download, supply-chain cascade,
long acceptance gate, release or owner-readiness action occurred. Control
reports retain zero model attempts and GPU minutes; score and tokens stay null.

## Artifact identity

Raw logs and reports remain private. These neutral labels bind the measured bytes.

| File | SHA-256 |
| --- | --- |
| `scripts/coding_eval_oracle.py` | `d94f0834591bc2ee3cf5f57fbc67104905e587258c30491c48a67f90368b4a0b` |
| `scripts/coding_eval_oracle_worker.py` | `d2eae83204538d240ce38792a7c92e1d8981f19930db7d6ff94545b9366f03c5` |
| `tests/test_coding_eval_oracle.py` | `72167777c0282db13964de143725a434f60582045fde094bcd58ee54db967665` |
| `tests/test_coding_eval_oracle_worker.py` | `cb0bbaaf426e2102de1b39ebe4da5bb9e29c3ba5e8d2ea4790843e40b3ea1a9a` |
| `fast.log` | `7b13f2e78fd8cb61fe7e86be0678ed1bc1418de0c1614d161be1f6c7a42ba7c1` |
| `fast-result.json` | `589000cf4cab7ee031441af525a4c6cf70f92bbc76e69439e7e1b211680ddcfe` |
| `legacy-unit.log` | `4ca62b3dbfef44fcc87fc7f75a6e807ef1526c8e620c0b5f70636f41404c56d2` |
| `legacy-unit-result.json` | `c9e7e8e3759c3c0b0db64d7091e9207e32a9db35fc02598372f17f250ff90669` |
| `independent/summary.json` | `c655bd6abe8a020691c8edf0a17567c233dc93d98366e3bded30db53f9b70b0a` |
| `independent-result.json` | `6718b13706cea23c3474a6c57f4f2e6a0f625748462a3017e8e547941a7ccfe9` |
| `legacy/summary.json` | `df3644b49231f9877b19f801b5827b8f5c439cbe749e0484f9638e934d0c88a9` |
| `legacy-result.json` | `9bc3a8b921a31c85f4b4b8edc5ab90611f2dfc1f6827892c14880773d866ec9c` |
| `probes.json` | `adf1cfc66bdd71c4e61ca842c2c3f8d0e20d80db33d64eebc9efa4f967c5e4bf` |
| `probes-result.json` | `02b71186cef51aa168f201b1d0ef9d15db33e652212f45d756c32975b252fbde` |
| `audit.json` | `5cc55b9ea926ad2dd5c0fc34901290c7a6e8defce4a8abd91e6ae7a049c1570e` |
| `source-digests.json` | `072824f7338cfba6d524e35cd9f43b1573951886c222247e0ce58a82d2fc1bb6` |
| `targeted.log` | `016cd6d0b0b8013ccd37a5fc2e0921639d7304dd216a90c7816e15ca106cfd95` |
| `targeted-result.json` | `0fff5d30d8e74861b5d167bc9aa1532607fe1ab87bdda4f429e7d4f5e96bf40a` |
| `targeted-repeat-result.json` | `ab515652da70dbb62d9b035e1365d09e6c6ea88a77b71c8ec822fc3922e3c0ac` |
| `launch-probe.log` | `e514e546ec8d18857450986bb6e4cd2a5f0ff3ed6ae9859c8495d49cdd47aead` |
| `launch-probe-result.json` | `46d9c61fd240bc7dac545c42ecf65b39abecb618494b2c23341b610e66a49843` |
| `initial-fast.log` | `cfa3fe1c4af0aaba3a38e043b8587f22debb934e90b1fc933a8af45246edbf9e` |
| `initial-fast-result.json` | `6802385839b9461d900e4950f62f1edaea3128415645907e18568f0f244975d8` |
| `initial-source-digests.json` | `29ef761a5c06c4bd5fe51332ceabc99bf727081c72899bf53d793bb4603da927` |
| `fixture-repair-result.json` | `eb6ed8d919291cfd8d9a3baf90549f1856a70903e0e143c81cc1f09b1f149117` |
| `policy.log` | `80444c8ed0d77bc59b74351ca54e4d919e7f22e8d509d4aaea7788b26ed9acf6` |
| `policy-result.json` | `b9e93d72f84c1bf5254affd64c82445bbb062c5bb2d58f5412caa87992780806` |
