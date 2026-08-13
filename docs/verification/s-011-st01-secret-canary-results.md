# S-011-ST01 Secret-Canary Results

**Status:** Pass for every currently representable persistence input and reachable surface  
**Result version:** 1  
**Task:** `11.1.3.2`  
**Fixture data:** Synthetic only

This campaign verifies that every input representable by the current
pre-persistence contract either remains ephemeral, is reduced to a digest, is
denied, or reaches only encrypted operational storage. It also scans every
currently reachable durable and diagnostic surface. Future adapters and model
execution must rerun this gate when they make new input or output surfaces
reachable.

## Input Closure

| Dimension | Closed set | Canary expectation |
|---|---|---|
| Record family | Sessions, objectives, plans, tasks, actions, evidence, decisions, grants, receipts, checkpoints, files | Same decision semantics for all 11 |
| Record identity | One synthetic canary identity in every family | Present only in prepared encrypted record; hashed in receipt/debug |
| Persist field | One synthetic canary value | Present only in prepared encrypted record |
| Digest-only field | One synthetic canary value | Raw value absent; SHA-256 retained in prepared encrypted record |
| Ephemeral field | One synthetic canary value | Raw value and digest absent |
| Raw attachment | Synthetic canary bytes | Structurally ephemeral; raw value and digest absent |
| Full tool output | Synthetic canary bytes | Structurally ephemeral; raw value and digest absent |
| Environment material | Synthetic canary bytes | Structurally ephemeral; raw value and digest absent |
| Prompt | Synthetic canary bytes | Structurally ephemeral; raw value and digest absent |
| Model response | Synthetic canary bytes | Structurally ephemeral; raw value and digest absent |
| Credential-marked persist field | Synthetic canary under `api_key` | Complete candidate denied; no prepared record |
| Restricted sensitivity | Synthetic metadata | No storage selected |
| Ephemeral retention | Synthetic metadata | No storage selected |

The separate deterministic scanner suite covers credential field names,
private-key envelopes, bearer credentials, provider-token prefixes, cloud
access-key identities, and URI user information. Secret findings retain only
the closed class; they do not retain the value or its digest.

## Surface Closure

| ID | Surface | Verification | Result |
|---|---|---|---|
| `SC-01` | Prepared record | Inspect canonical bytes before encryption | Only persist value/identity and digest-only SHA-256 occur as authorized |
| `SC-02` | Persistence receipts | Serialize complete admitted and denied receipts | Raw canary absent |
| `SC-03` | Debug and stable errors | Render candidate, decision, prepared store, receipts, and error codes | Raw canary absent outside prepared callback |
| `SC-04` | SQLCipher main file | Byte scan while open and after close | Raw canary absent |
| `SC-05` | SQLCipher WAL | Byte scan when present while open and after close | Raw canary absent |
| `SC-06` | SQLCipher shared memory | Byte scan when present | Raw canary absent |
| `SC-07` | SQLite temporary storage | Verify `temp_store=MEMORY`; scan state directory fixture | No disk temporary store admitted |
| `SC-08` | Encrypted backup | Separately keyed online backup, close, and byte scan | Raw canary absent; backup reopens |
| `SC-09` | Derived JSON Lines | Generate complete derivative and byte scan | Raw canary absent; no import/startup authority |
| `SC-10` | Logs | SQLCipher logging disabled before key validation; no product persistence logger exists | No reachable log sink receives candidate values |
| `SC-11` | Model context | No model is enabled or connected to persistence; raw-content constructors force prompts/responses ephemeral | Zero reachable persistence-to-model-context path |
| `SC-12` | Crash output | Public errors and debug values are content-free; deterministic crash result retains identities/hashes only | Raw canary absent from reachable crash diagnostics |

## Sanitization Result

- Synthetic record families exercised: **11 of 11**.
- Field-handling modes exercised: **3 of 3**.
- Structurally ephemeral raw classes exercised: **5 of 5**.
- Secret-finding classes exercised: **6 of 6**.
- Reachable output surfaces closed: **12 of 12**.
- Raw canary values retained in evidence: **0**.
- Plaintext fallback paths observed: **0**.
- Unauthorized model-context paths observed: **0**.
- External network operations used: **0**.

## Invalidation Rule

This result becomes stale if a new record family, field constructor, secret
class, persistence adapter, disk-temporary path, logger, exporter, backup
format, crash reporter, or model-context bridge is added. Such a change must
extend both the closed test matrix and this result before the new surface can
be admitted.

## Deliberate Limits

- Typed production writers for all 11 normalized tables do not yet exist; the
  policy matrix tests the public pre-persistence contract, while the encrypted
  artifact test directly seeds the currently available session table.
- Logs and model context are closed by structural absence, not by an active
  model workflow. Sprint 12 and later model activation must rerun and extend
  this gate before those paths can ship.
- Deterministic byte scanning is not forensic proof about physical remanence in
  SSD firmware, filesystem snapshots, swap outside the declared no-swap
  profile, or external host instrumentation.
- The at-least-100-seed crash campaign remains `11.1.3.3`.
- Live Secret Service, cross-platform, packaging, release acceptance, and
  manually deferred fuzzing remain separate gates.
