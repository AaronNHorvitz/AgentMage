# T-303/T-306: exact source pairs for original-owner candidate review

Status: independent core review required. Both parent rows stay open, without
new rows or sub-rows. Existing ancestry remains immutable.

The explicit trusted candidate checker previously received source digests without
source content. It now receives complete bounded before/after source pairs from
the original two-root read scope, correlated with the exact packet and retained
native write history. Every packet file is included, even when the candidate
selects only one logical commit group. Immutable-base absence differs from an
existing empty file; separately approved new-file modes remain visible.

The existing offline immutable-base collector retains each already verified blob
alongside its digest and Git mode. Its object-identity, path, file-count and
128 KiB per-blob bounds remain. Candidate postimages use the existing confined
source reader. Both byte sets must match the exact packet hashes before checker
invocation. Complete pre/post Git observations also compare base bytes. Successful
review still requires all original source, mode, validation and ownership checks.
The base observation's debug output omits source bytes; source-pair values have
no serialization, content-bearing debug representation or public constructor.

The bytes are untrusted local data. Reading them does not authorize disclosure
to a model or service. This increment installs no default reviewer and establishes
no artifact capture/scanner provenance, person consent or execution authority.
Native admission, actual independent checker/artifact integration, complete
live-owner approval composition, recovery and model measurements remain open.
No API, MCP or terminal commit/push route is enabled.

## Validation

All 32 original-owner review methods pass in the focused run: 20.67 seconds
compilation and 11.72 seconds tests, after the shared reservation. This includes
exact before/after bytes, approved regular/executable creations, complete packet
material for a selected subset, and refusal of hash-only base observations and
stale/missing/extra/absent source content. Existing offline blob tests compare
the retained bytes as well as hashes and modes. The new debug test verifies that
source bytes are omitted. These are component observations with synthetic native
creation/Git observations, not an admitted native workflow or model result.

The complete affected gate remains **FAIL**: **1,430 passed, 72 failed,
64 ignored**, zero measured or filtered, across **26 unfiltered targets** of the
three affected crates. Every per-target failure and ignore identity matches the
preceding complete baseline exactly. This batch adds three Linux passes.
The unchanged kernel's twenty targets are outside this batch's affected gate;
they were included in the preceding complete-packet baseline. No affected target
is excluded. All three new methods and 54 existing methods in changed sources,
formatting and Clippy with warnings denied pass. All six source hashes remain
fixed, with no later source edit.

The full run takes 694.40 seconds including shared reservations: formatting
70.54, Clippy 26.79 and Rust tests 595.60 seconds. No timeout occurs. The raw log
SHA-256 is
`4e02fd3096626959957f9364f2083bccf986d405ff3aaff3ee03072ded8f1a3c`.
The data-comparison script exits zero; this is not a gate PASS.

Focused and complete affected commands, raw logs and source freeze are retained
in the private reproduction bundle `/lane/state/tmp/t306-review-source`.
The runner reserves heavy commands internally. Never wrap that runner in another
build-slot invocation. Final documentation, public-content, unchanged-row and
staged checks pass. The preceding review report arrived during the frozen run;
only documentation and its byte-identical retained report were added afterwards.

No evaluator, hidden/held-out task, scorer, prompt, threshold, dependency, SBOM,
lifecycle, scoreboard, release or readiness changes. No GPU, host key, download
or push. Existing low findings remain same-row backlog.

The preceding complete-packet commit received independent **Verdict: PASS** during
this run, with no high or medium finding. Its report is retained byte-for-byte.
Low L-1 (construction-time approval clock and dispatch-site coverage) and L-2
(raw post-effect reconciliation visibility) remain same-row backlog for product
execution composition. Neither parent row closes.
