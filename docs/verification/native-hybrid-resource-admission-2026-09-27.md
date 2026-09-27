# Native Hybrid Resource Admission — 2026-09-27

Status: verified bounded admission-parser correction; the subsequent source-pinned
Muse campaign passed eight development cases, as recorded below. Production model
admission and independent qualification remain open. Scope: AMR-01 and AMR-02.4 prerequisites.
Engineering decision status: Accepted under owner delegation, 2026-09-20.
Authority: Decision 0054. No independent review or acceptance is asserted.

## Retained live failure and diagnosis

Campaign 14's first Muse repair attempt used clean source
`5974f33aa986b3742c452517039ba9d0c8feaf47`. It exited 5 after 11.835863 seconds
with `coding.development.candidate.load.model.runtime.failed`, before any prompt,
response or tool event. The outer supervised campaign exited 1, not its unresolved
GPU-cleanup status. The resource guard reported no error and sampled 1,876 MiB at
both baseline and peak. No served-context observation was produced. The remaining
cases and second repetition were not launched.

The failed campaign log is retained with SHA-256
`8a5e7bdaf1157ac9a1466a30700e704f3696f22e6caf25ebb38143cf9de3ac41`.
This is not a model-capability, codec or generation-budget failure.

The native resource parser required exactly one membership line. The actual
kernel view contained both a legacy `net_cls` membership and a unified v2
membership. Linux explicitly permits multiple membership records on a hybrid
host and identifies v2 with `0::`. See the
[kernel cgroup-v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html#processes).
The old parser therefore rejected the scope before reading its actual controls.

A CPU-only diagnostic verified all pinned packaged runtime files and links and
the exact Muse artifact hash. Its separate diagnostic parser also failed on the
hybrid layout; that attempt remains a failure, not successful admission. Its
whole-stat comparison included access time and is not evidence of model-content
drift. Diagnostic log SHA-256:
`a1458643d9648af988be4d298cdb0732158354b12bb6d3fabc95a4fc072ee9df`.

## Correction and unchanged restrictions

The parser now selects exactly one valid unified membership while checking the
shape of legacy records. Missing or duplicate hierarchies, duplicate controllers,
invalid fields, root/unavailable unified scope, traversal, aliases, deleted paths,
control bytes and oversized observations refuse. Legacy paths never substitute
for the v2 resource scope. Missing actual CPU/memory controls still refuse.

The prepared manifest, exact model/runtime, native lease, confinement and all
existing CPU, RAM, memory, swap and GPU thresholds are unchanged. The 32K context,
4K output reserve, four threads and single slot are unchanged; the 8K demo is
untouched. This introduces no dependency, execution loop, store or authority.

The wider check also found three corpus references to two Rust tests renamed in
`f8dd4e79`. Their existing assertions and history were inspected before updating
the names. Case IDs, expected outcomes and prerequisites remain unchanged. The
corpus check retains its exact function-presence regex but reports a bounded
missing-test message instead of dumping every repository source. Name presence
is not new semantic, executable or model coverage.

## Inspected verification

The hybrid regression compiled and failed with `model.native-resource.scope-unavailable`
before the fix (exit 101). Its retained log SHA-256 is
`3d3c07840480cba20b2c42eb0265134400d5f727a7d241ada89288f8093d3929`.

After correction:

- Six focused resource tests and strict inference Clippy passed. Log SHA-256:
  `99ecd8e4313e0d5f8f0cf7d948b4054392a38a4e91527a0bc874fe48af99e9bb`.
- The actual CPU-only native preflight passed inside the unchanged 5/6 GiB,
  512 MiB swap, 200% CPU scope. It launches no GPU observer, model or tool worker.
- 121 inference unit tests, ten inference integration tests and 322 host tests
  passed; strict host Clippy and the actual CLI/host/worker build passed. Ordinary
  ignored live/model tests are not counted as passes. The wider Python stage
  retained its corpus-reference failure: 76 passed, one failed. Whole-job exit 1;
  log `326cbaa82f8e4ef84dd80b6bafdac0c15e7f612eddb2ff85d8dd38da307d5616`.
- The first remainder exposed the next stale reference, again 76 passed and one
  failed. Log `88e2d340ac148b4a16f8301439b56865dc1203b0715de45723e9590aae536a04`.
  All missing names were then inventoried. The three corpus tests, source/effect
  audits, formatting and 22 actual-CLI scripted cases passed in the final
  remainder, exit 0. Log
  `77f3b2671ad509af29d6806f86c88ebfaad1f6a27f45db3c8180840deea4438a`.

Every scripted assertion, all 44 raw-log hashes and all 22 implementation identities
were inspected before documentation or staging changed. Matrix SHA-256:
`631f43246127b7d06021c04a7fbe36edd08f0a9a6836ea1c3236e4024f0e4f0c`.
Read-only inspection SHA-256:
`9e59b98e183a152ed2bb9f8f8a644d6bd8ea5c52120cdbe12673711233362f23`.
These are scripted executable results, not real-model success.

The three-file tested source/fixture/test diff against `5974f33a` has SHA-256
`96bc3fb95eed97f58b956da4a21775b5875c3af145bb185b249920fd5d9e0b00`.
Corrected Rust file SHA-256:
`3f56221af184ef330e7de202e2685c6c3eb5b2351e52b2bbeb65bfde170c012a`.
Executed CLI, host and read-worker SHA-256 values, respectively:

- `f074bf9a20fca6217febfa1d390ae4323f51fb5b9074c83211fa861e5152a389`
- `db36c5739d2daeb9328ef237885f6fd63db1160d8cdf77724b97f810f1204fa3`
- `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077`

## Freshness and remaining gates

The coherent correction was checkpointed before a changed, clean-pin native
campaign. At `b4417f47`, its work unit stayed open for that feedback, with the
single SBOM/evidence regeneration deferred until source corrections finished.
That checkpoint's SBOM was historical and stale for the changed inference source;
it did not claim current freshness or G-DOD-11 acceptance. This avoided repeating
a historical cascade per discovered defect without weakening any binding.
The subsequent campaign record below carries the unit's renewal disposition;
this debt must not be silently carried into unrelated implementation. Historical
reports are not rewritten as new proof.

Muse's earlier eight cases remain bounded evidence at their original source pin.
GPT-OSS remains separately unqualified; its unchanged malformed-response campaign
is not repeated. New native runs must retain exact identities, failed tests,
repairs, full artifacts, verifier outcomes and cleanup under the scheduled guards.
Independent review, human acceptance, command/Git lifecycle work, connected
research, supported-platform and release gates remain open. No AMR row or
`M-HARNESS-DAILY` milestone is closed here.

## Subsequent exact-source model check

[Campaign 15](coding-harness-campaign15-results-2026-09-27.md) subsequently passed
all eight Muse cases at clean `b4417f474a1ab4406de18c3e45a9ca61692a7743`, using the
unchanged profile, generation budget and limits. Both supervised passes exited 0
after their cleanup checks. The retained failure above remains a failure; this
new result is bounded development evidence, not production activation or review.
The linked record carries the correction unit's evidence-renewal disposition.
