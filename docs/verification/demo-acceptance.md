# Fedora local demo acceptance

Verified on the owner's Fedora Kinoite computer on 2026-09-15 America/Chicago
(2026-09-16 UTC), on branch `demo/fedora-local-docs`. This is scoped demo
acceptance under Decision 0053; production preview/full-GA remains incomplete.

## Repeatable commands and results

```sh
python3 scripts/demo_smoke.py
python3 scripts/demo_evidence_check.py
```

Both exited **0**. The full smoke builds/launches the application, automates its
actual browser interface, stops it, exercises the same application/Rust document
runtime under network isolation, then restarts and repeats a successful browser
interaction. It leaves the restarted application running. The evidence checker
verifies complete recorded input-file SHA-256 bindings without modifying reports.

All **11 browser cases passed**: launch/model readiness; accepted/skipped inventory;
real-model answers and source inspection; follow-up; honest unanswerable response;
actual tokenizer context overflow; cancellation and recovery; unavailable model and
retry; six-turn limit and new conversation; folder boundaries, unsupported/non-UTF8
inputs and untrusted source instructions; missing-token/wrong-Origin/wrong-Host
endpoint refusals. Boundary tests use synthetic outside-folder secrets; no personal
files are read. Exact Host tests use Node HTTP to preserve the requested headers.

The known-fact assertions inspect generated answer text separately from quoted
source text. Answers correctly identify 18 October 2026 and Mira Chen from
`project.md`, Theo Park and rehearsal timing from `operations.txt`, and the budget
and venue. Citations contain actual supporting snapshot text, file digest and source
ranges, and are expanded through the interface. The absent favorite-ice-cream fact
produces an insufficient-evidence response with no citations.

## Retained machine-readable evidence

- `demo-browser-acceptance.json`: all browser outcomes and real application answers,
  supporting citations, prompt/output token counts and measured generation latency.
- `demo-offline-acceptance.json`: only loopback in the scoped application namespace,
  external connection refused with network unreachable, and successful real local
  inference through the same application/runtime path. The model has its own nested
  networkless namespace. No host network settings were changed.
- `demo-restarted-browser-acceptance.json`: fresh browser launch/admission and repeated
  successful real inference after complete application/model stop and restart.
- `demo-restart-acceptance.json`: successful command sequence and final running PID.
- `demo-resource-observations.json`: actual point observations, not certified peaks.
- `demo-regression-results.md`: relevant regressions and the five remaining unrelated
  scaffold fixture failures, recorded honestly.
- `demo-model-probe.md`: exact model/runtime provenance and investigation of rejected
  historical profiles and initial prompt/decoding failures.

The selected model is first-party Meta Muse Glimmer 30B Q4_K_M, SHA-256
`4cc57c0f51040a226e5a72cc47b7613f7772950e460a665f7083de89f183f60e`,
using the fully hash-pinned existing llama.cpp b10423 Vulkan package, commit
`a94d563ed801d1da1b8c2432946de07d0231bb3d`. Exact configuration is
`demo/model.json`: 8,192 context tokens, 2,048 maximum output tokens, one slot,
all GPU layers, temperature 0 and seed 42. No cloud fallback or downloads occur
at launch. Historical rejected production model profiles remain rejected.

Observed GPU use was 17,318 MiB including the desktop on the 24 GiB RTX 4090;
model process RSS was 1,501,760 KiB. Per-interaction timings are retained in the JSON;
final browser acceptance recorded 15 completed interactions at 4.446–19.650
seconds, including reasoning. Offline inference took 15.780 seconds; the fresh
post-restart answer took 15.598 seconds.
These are observations on this machine, not a performance guarantee.

## Limits

Text/Markdown UTF-8 only; lexical retrieval and explicit bounded context omissions;
six successful turns; no automatic compaction; incomplete output rejected; snapshots
and conversations memory-only. Citation identity/ranges are validated, but semantic
entailment for arbitrary documents is not formally proven. Storage is privately
permissioned, not claimed encrypted. This browser demo does not qualify installed
native packages, Windows, enterprise features, production models or release gates.

See [LOCAL-TESTING.md](../LOCAL-TESTING.md) for one-command launch/stop, limits,
setup, troubleshooting and the five-minute walkthrough.
