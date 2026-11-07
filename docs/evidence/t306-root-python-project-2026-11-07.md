# T-303/T-306: explicit root-level Python tests

Base: `7976c11955565f4b98f33df765384b22624b6117`. This same-row batch extends
the accepted project-test definition under Decision 0155. Both mission parents
stay open. Independent core review is required; no coding score or model
qualification is claimed. The preceding [recovery repair review](../reviews/7976c1195556-local-coding-recovery-lifecycle.txt)
is retained byte-for-byte with its exact-commit **Verdict: PASS** and no open
high/medium finding. Its two low coverage findings remain backlog, along with
the existing factory allocation lifetime issue.

## Behavior

`python-root-project` explicitly selects isolated verbose unittest discovery
from the worktree root through the existing native coordinator. The fixed argv
is `-I -B -m unittest discover -s . -t . -v`, with a distinct command identity.
No model or repository text can select arbitrary arguments or a layout fallback.
The separate-process service, owned attach route and existing terminal run driver
accept this closed selection. Existing `python-project` keeps its original
command unchanged.

The same one-to-sixteen exact existing `src/` Python files are writable. Files outside that exact sealed source list, descendant
paths and file creation remain refused. No shell, inline interpreter or network
authority is added.
Every command invocation still needs its existing exact kernel approval; limits
remain 30 seconds, 256 MiB and bounded output. The validation template binds the
selected command. A distinct sealed layout constraint rejects cross-layout resume,
missing markers, duplicate markers and conflicting markers. The positive-test
minimum and existing strict output parser remain unchanged.

Root discovery supports top-level `test_*.py` modules and importable nested
packages. It does not recurse into unpackaged directories. A preliminary actual
interpreter probe retained privately reproduced the package selection's import
refusal without `tests/__init__.py`, missing `src` imports when dropping the
explicit top level, and zero discovery when scanning a root with only an
unpackaged tests directory. These results are not relabeled as supported layouts.
A top-level test module is the new supported case. Root selection can discover
more tests than the package selection; the exact command is displayed for review.

## Verification

Locked/offline checks used two build jobs and four test threads after waiting
for the shared reservation. One verified outer build reservation covered focused
checks and the original fast-gate planner/executor. Only Cargo's executor prefix
was changed to avoid recursively acquiring that already held reservation; target
selection, supervisors and failure handling were unchanged.

| Focused check | Actual result, including compilation |
| --- | --- |
| Host project composition and interpreter fixtures | 6 passed / 587 filtered; 11.734511 seconds |
| API coding contracts | 20 passed / 5 filtered; 7.145290 seconds |
| Terminal driver approval/denial contracts | 1 passed / 14 filtered; 4.196786 seconds |
| Native launcher closed selections | 1 passed / 320 filtered; 1.505769 seconds |
| Gate formatting / Clippy with warnings denied | PASS; 1.15 / 8.77 seconds |

Every focused check has zero failures and zero ignores. The terminal method
exercises six synthetic HTTP/PTY cases: approval and denial for fixture,
package-project and root-project selections. API checks bind the selection to
startup and reject a prepare body that tries to change it.

The new root interpreter fixture executes a real failing assertion, source repair
with a passing test, genuine import error and zero discovered tests. The four-run
fixture took 244,640 microseconds. A worktree-local `unittest.py` that raises on
import did not replace the initially imported standard runner. The definition
check verifies distinct command/template identities, fixed offline limits,
positive-test minimum and missing/duplicate/conflicting/cross-layout marker
refusal. Existing exact source scope and creation refusals still pass.

These are component and synthetic-repository measurements, not native sandbox,
model or full workflow evidence. The selected Python interpreter executes
outside the native command sandbox only in the trusted fixture. The complete
Rust gate is **FAIL**: 918 passed, 72 failed, 63 existing ignores and 0 filtered
across 22 target summaries; 120.57 seconds including compilation. Every failure
identity matches the prior batch exactly: 53 host and 19 Linux failures under
deferred T-301.1. There are no new or removed failures. The two new host tests
and expanded terminal contracts pass in the complete run. The gate remains FAIL;
no skip, ignore, threshold or failure allowlist was added.

Final four-file Markdown, retained-review, whitespace and source-policy checks
cover the completed report. The private review request retains the raw logs,
complete policy-input manifest, eight-source freeze and commit-scope audit.
No SBOM or historical hash-bound evidence is regenerated.

## Remaining scope

No native positive create/run/recover/apply workflow, arbitrary configured coding
endpoint, durable ownership, new-file transfer, approved commit/push, MCP or
parent measurement is established. The deferred T-301.1 native trust blocker
still applies. No evaluation task, hidden test, held-out input, threshold,
score, dependency, SBOM, lifecycle status or release authority changes.
