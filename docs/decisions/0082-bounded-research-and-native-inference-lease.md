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

Preserve the exact policy-evaluated network scope on the existing nonforgeable effect
authorization, only for `NetworkAccess`. A future network driver must match it to the
prepared destination as well as matching exact call arguments and expected effects.
A validated context scope must not disappear before the launch boundary or become
implicit permission for a different endpoint. This adds no grant constructor or client.

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
Observed expiry and clock rollback are terminal for a budget. A later older timestamp
cannot revive it; quota refusals still preserve the trusted clock high-water mark.

The Linux native driver obtains one nonblocking, descriptor-held exclusive lease in the
standard user's private runtime directory before launching a model. The fixed shared name
does not vary by model, workspace or client. Validate ownership, mode, file type, link count
and descriptor/path identity; never delete a lock to obtain admission. Release follows owned
process cleanup, not a cancellation request. This coordinates participating AgentMage hosts,
not arbitrary external processes; the operator's separate machine reservation and existing
model/resource/isolation admission remain mandatory. No new scheduler or GPU quota is claimed.

If driver destruction cannot verify owned cleanup, intentionally retain its one lock descriptor
until the owner process exits instead of announcing an available slot. Normal cleanup does not
leak descriptors. This fail-closed disposition is not a claim that an uncertain descendant has
stopped; the enclosing supervisor/campaign guard must still verify cleanup before handoff.
Failed startup without an established runtime descendant is explicitly uncertain, even if
the launcher was reaped. A descendant identified before a failed sandbox check remains
tracked for cleanup. Neither case receives a successful unload/released-slot receipt merely
because startup failed.

The explicit 32K development candidate path also repeats the existing preparation resource
preflight in Rust after acquiring the native lease and before spawning inference. Read actual
available RAM, current cgroup-v2 memory/CPU controls and the pinned platform utility's single-GPU
memory observation. Use the existing 16 GiB available RAM, 21,000 MiB free VRAM, 22,528 MiB
maximum total-used VRAM, 5/6 GiB memory high/max, 512 MiB swap and two-core quota limits;
missing, malformed, unbounded or changed observations deny launch. This is not a new quota,
hardware qualification, model admission or proof of future availability. The outer supervised
campaign's sampled GPU guard and deadline remain mandatory. The existing 8K demo configuration
is not silently converted to this development profile.

## Verification and unchanged gates

Use process-level contention and unsafe-object cases plus exact native development coding
checks. Deterministic provider/plan tests are component evidence, not real research success.
Retain rejected runs and exact identities. A live research campaign additionally needs a
configured provider, admitted outbound effect boundary and exact local model admission.
Independent review, human acceptance, production activation and licensing remain unchanged.

The existing developer harness additionally records the content identities of non-ignored
untracked files, including new Rust modules. This supplements its complete tracked diff,
HEAD and binary identities; it does not narrow existing bindings or replace a pinned clean
campaign. Bound the inventory and refuse aliases, special files and unstable observations.
No source bytes enter the diagnostic record, and historical campaign records are not rewritten.

The strict-local source inventory enrolls the new kernel module only for standard-library
address values/classification, as already done for other inert network validators. The closed
socket/client API allowlist is unchanged; no kernel network client is introduced.

Address-classification review consulted the IANA
[IPv4 special-purpose registry](https://www.iana.org/assignments/iana-ipv4-special-registry)
and [IPv6 special-purpose registry](https://www.iana.org/assignments/iana-ipv6-special-registry).
The first increment is deliberately conservative (including refusal of special-purpose
exceptions), not a routability guarantee. A classifier cannot replace actual connection
pinning, TLS validation, hop-by-hop checks or the native adversarial transport campaign.
