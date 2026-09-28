# Decision 0100: Native Model Socket Exchange Deadlines

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-28 |
| Authority | Decisions 0054, 0061, 0081, 0088 and 0095; current owner restart |
| Scope | AMR-01/AMR-03 and Tasks 48.2.4.2, 48.2.4.4 and 48.2.5.1 prerequisites |

## Finding

Native health, serving-property and token-count requests apply timeouts to each
blocking socket operation. A peer that continues sending small responses can
extend an exchange beyond that timeout. Native completion starts its response
deadline only after writing the request; a blocked upload does not observe the
existing cancellation probe. Blocking local connection establishment also occurs
before these controls. Seven original Unix-socket tests reproduced six failures;
the existing byte-ceiling case passed. The queued connection waited about 250 ms
and a slow response exceeded its intended budget. These are non-model component
observations, not an observed real-model or production failure.

## Decision

Give each existing private socket exchange one monotonic deadline established
before building headers and connecting. Completion starts its deadline before
encoding its prompt as well. Carry the deadline through request upload and
response reading without renewal. Use nonblocking local sockets and bounded
readiness polling through the existing dependency. A full connection queue is
unavailable; do not wait for another accept or retry the exchange. No dependency,
endpoint, scheduler, worker, authority or model owner is added.

For completions, observe the existing exact cancellation probe before connection,
during upload and while reading. Keep current cancellation identity validation,
parser validation, byte ceilings, sampling tuple and terminal-event semantics.
Cancellation or expiry during upload produces the existing inert interrupted
completion, with one terminal fragment and no fabricated output. A probe failure
or mismatched identity remains a failure. Close the owned socket on every return;
this is not proof that a model process or an isolated namespace stopped.

Health, properties and token-count calls retain their existing per-exchange
budgets and lack of a cancellation argument. Report exchange expiry distinctly
from other read/write failures. Keep native process/file ownership, manifest and
served-profile admission, the inference lease, uncertain cleanup and all grants
unchanged. The transport does not reconnect or resend a failed request. Existing
bounded readiness polling remains with the native driver.

The deadline is cooperative: it does not preempt serialization, parsing or a
kernel-stalled syscall. Check expiry before accepting I/O progress and successful
HTTP response parsing. Endpoint work around the exchange, including token-request
encoding and endpoint JSON parsing, still lacks shared run control. A sequence of
separate preflight calls does not acquire a shared run deadline from this change. Model manifest hashing, startup readiness,
exact token binding and synchronous factory composition still need their existing
owners integrated with run cancellation and remaining time. Moving model loading
onto the current live worker remains separate open work.

Keep the exchange helper and its socket fixtures inside the existing driver file.
Existing evidence that binds that complete file continues to bind the implementation
and tests; no source input is narrowed or moved outside that boundary.

The unchanged source audit initially refused the low-level socket import. Admit
only the complete private Unix exchange module through an exact SHA-256 pin in
that audit. Recognize its actual top-level declaration and sole private import
with the existing conservative lexical mask; reject missing, duplicated, nested,
malformed or changed modules and every additional production use of either the
dependency or helper namespace in the driver. Keep the general socket allowlist
unchanged, and continue all other network and URI checks over the full production
source. This admits the fixed Unix address, stream type and nonblocking flags
together with the complete control implementation; it grants no general socket
API exception. Future helper changes require explicit pin renewal and review.
The compiler and native confinement remain necessary: a lexical developer audit
is not a Rust parser or runtime permission boundary.

## Verification boundary

Retain actual failing pre-fix tests using only owned Unix sockets. Cover a slow
response across all three closed endpoints, silence, blocked request uploads,
completion upload cancellation/expiry, a full connection queue and byte ceilings.
Preserve successful exchanges and existing streaming/parser/cleanup tests. Test
cancellation after an observed output fragment by that event, not by relying on a
particular number of internal probe calls.

Exercise audit mutations for other address families and socket types, weakened
flags/control, alias and grouped imports, extra calls, visibility changes,
comment/string lookalikes, nested or missing declarations, malformed delimiters,
trailing production after tests and copying the helper into another file. Verify
that an otherwise valid helper does not suppress other network or URI detectors.

Run checks through the shared build reservation and retain every failure. Batch
source changes before one applicable SBOM/evidence pass. Socket fixtures are
component evidence; they do not qualify native inference, active process cleanup,
the Linux coding demo or a model profile. Independent, human-only and release
gates remain open.
