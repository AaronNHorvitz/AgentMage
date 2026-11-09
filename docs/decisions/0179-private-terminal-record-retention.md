# Decision 0179: Explicit private terminal-record retention

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-09 |
| Authority | Local agent mission; T-303/T-306; Decision 0178 |
| Scope | Terminal-client retention and offline inspection of complete event records |

Offer an explicit terminal save command over the existing authenticated request
and event routes. Reuse the unchanged record validator, limits and offline
replayer. Require a controlling terminal and exact save consent displaying the
request, destination and size. The selected directory must already exist, be
canonical, current-user-owned and mode 0700. Hold it across consent and recheck
its identity before writing. Create a mode-0600 file named by the sealed request
digest with exclusive, no-follow creation; never replace or clean up a name.
Synchronize the file and directory before reporting success. Partial failures
may retain a file and require inspection; never automatically retry.

The explicit saved-transfer variant must finish saving and flush its receipt
before offering terminal release. Declining save or failing retention preserves
the process-held run. Release, fresh two-root read consent and exact diff approval
remain separate decisions. The existing stdout-only workflow stays available.

The record is plaintext private operational data, not an authenticated execution
checkpoint. It retains canonical envelopes and payload references, not complete
payload bodies. Replay verifies self-consistency, not origin or task success.
It restores no owner, port, source, grant, approval, budget or execution cursor.
Path rechecks are observations, not an atomic filesystem race guarantee. This
adds no service-side store, effect replay, trust exception, model admission,
coding score or parent closure. Durable execution recovery remains open.
