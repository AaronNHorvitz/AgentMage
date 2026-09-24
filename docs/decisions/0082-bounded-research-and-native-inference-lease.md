# Decision 0082: Bounded Research and Native Inference Ownership

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-24 |
| Authority | Decision 0054 and the explicit restart implementing Decision 0081 |
| Scope | AMR-02 and AMR-03 first bounded Rust increment |

## Decision

Reuse the existing coordinator, native effect owner, canonical journal/artifact store,
public citation verifier and secret detector. A research policy is an additional restriction,
not a capability grant, provider credential, effect receipt or model admission. No network
effect is enabled merely by constructing one. Retrieved content has no authority.

Freeze these first-increment ceilings before implementation. Callers may restrict them;
they cannot increase them through model proposals or resumed state.

| Dimension | Quick search | Deep research |
|---|---:|---:|
| Queries | 1 | 6 |
| Page visits | 5 | 20 |
| Exact allowed domains | 8 | 16 |
| Aggregate downloaded bytes | 1 MiB | 8 MiB |
| Redirect hops per request | 3 | 3 |
| Elapsed milliseconds | 60,000 | 300,000 |
| Provider charge allowance | 0 | 0 |

No paid provider, account, credentials or download allowance is supplied by this decision.
Use explicit public query disclosure, independent network permission and bounded source
fetches. Offline denies all outbound work; ask requires the existing exact approval path;
task authorization is exact and expires. Reject secrets before disclosure. Validate every
destination and DNS answer, including redirects, and pin the actual connection to admitted
addresses; a string-only URL check is not transport isolation. No ambient proxy, cookies,
workspace handles, executable download, browser state or authorization inheritance.
Uncertain in-flight external outcomes consume budget and are not automatically replayed.

The Linux native driver obtains one nonblocking, descriptor-held exclusive lease in the
standard user's private runtime directory before launching a model. The fixed shared name
does not vary by model, workspace or client. Validate ownership, mode, file type, link count
and descriptor/path identity; never delete a lock to obtain admission. Release follows owned
process cleanup, not a cancellation request. This coordinates participating AgentMage hosts,
not arbitrary external processes; the operator's separate machine reservation and existing
model/resource/isolation admission remain mandatory. No new scheduler or GPU quota is claimed.

## Verification and unchanged gates

Use process-level contention and unsafe-object cases plus exact native development coding
checks. Deterministic provider/plan tests are component evidence, not real research success.
Retain rejected runs and exact identities. A live research campaign additionally needs a
configured provider, admitted outbound effect boundary and exact local model admission.
Independent review, human acceptance, production activation and licensing remain unchanged.
