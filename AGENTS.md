# AGENTS.md — Working Rules for Coding Agents

Authoritative for any coding agent operating in this repository. Does not override
`TASKS.md`, `architecture/status-model.json`, or any accepted Decision. It governs **how**
work is sequenced, not **what** work is admissible.

---

## 1. Batch capability work before regenerating evidence

**Rule.** Do **not** regenerate evidence after each individual capability. Expose **all**
capabilities planned for the current work unit, then regenerate evidence **once** for the
whole batch.

**Prohibited pattern** — one capability, one full cascade, repeated:

```text
Expose <A> through native clients
Renew <A> dependency evidence
Renew <A> platform evidence
Renew <A> Story 7 review
Renew <A> Sprint 7 review
Renew <A> runtime parity evidence
Renew <A> Story 1.3 review
Retain <A> native evidence
Renew <A> traceability
… then the identical nine commits for <B>, <C>, <D>
```

**Required pattern:**

```text
Expose <A> through native clients
Expose <B> through native clients
Expose <C> through native clients
Expose <D> through native clients
<one evidence regeneration pass covering A–D>
```

### Why

Evidence artifacts are **hash-bound to their whole input files**. Each records the SHA-256
of every file it read, and `G-DOD-11` requires evidence indexes to be current. Changing one
byte of an input provably invalidates every artifact referencing it.

**The carrier is the SBOM.** `scripts/supply_chain.py` computes a `tree_hash` over every file
in each Cargo workspace member (`rglob("*")`, line 68; applied at line 162) and writes it as
that crate's component `content` hash in `supply-chain/sbom.cdx.json` and
`supply-chain/dependency-provenance.json`, which in turn changes
`supply-chain/dependency-hashes.sha256`. Those three outputs are hashed inputs of evidence
artifacts across many sprints — 11, 13, and 10 artifacts respectively as of `2026-09-01`.

Therefore **any source edit inside any workspace member — regardless of which file — flips
the SBOM the next time `supply-chain:build` runs and invalidates every artifact bound to it.**
On `2026-09-01`, editing `shells/host/src/cli.rs` for Sprint 56 changed the `agentmage-host`
tree hash, which invalidated `sprint-1/story-1.2` and `sprint-3/story-3.1` evidence.

The tree hash is **correct provenance** and must not be weakened (§2). The cascade cost is
controlled by **when** the SBOM is regenerated, not by what it hashes. Capabilities in the
same work unit flip the same SBOM; regenerating per capability repeats identical work N times
for no additional assurance. **The cascade cost is per batch, not per item.**

Most-referenced evidence inputs overall, for awareness (373 artifacts scanned):

| Input | Artifacts referencing it |
|---|---|
| `scripts/supply_chain.py` | 48 |
| `kernel/engine/src/operational_store.rs` | 36 |
| `kernel/engine/src/lib.rs` | 31 |
| `SECURITY-REVIEW.md` | 29 |
| `Cargo.lock` | 28 |
| `Cargo.toml` | 25 |
| `package.json` | 20 |

### Measured impact

On `2026-09-01`, 94 commits produced 21 closed items — **4.48 commits per item**. 72 of 94
(77%) were `Renew` / `Retain` / `Refresh` of existing evidence; only 4 were `Expose`. Four
capabilities triggered four full cascades of roughly 45 minutes each.

Batching those four into one cascade removes roughly 54 of the 72 renewal commits — a **~57%
reduction in total commits for identical output**.

## 2. Do not weaken evidence binding to go faster

The hash-bound input set is the reason a checked box can be trusted. **Do not** remove inputs
from an evidence artifact's hash set, narrow what is hashed, or skip regeneration to reduce
cascade cost. Batching is the sanctioned optimization. Any change to binding granularity is a
deliberate architectural decision requiring an accepted Decision record — not an agent
optimization.

## 3. Do not regenerate supply-chain outputs mid-batch

The cascade is triggered by **regenerating** the SBOM, not by editing source. Complete
**all** source edits for the work unit first. Run `supply-chain:build` and evidence
regeneration **once**, after the last source edit in the batch. Never interleave source edits
with evidence regeneration. Under Decision 0021, prefer grouping items whose source edits fall
in the same work unit so they share a single regeneration.

## 4. Report honestly

State what is open. Do not claim platform, runtime, model, or integration support that has not
been demonstrated. `architecture/status-model.json` is the authority on current status; leave
`lifecycle_status`, `verification_status`, and the "Current Implementation Truth" block in
`TASKS.md` accurate at all times.

## 5. Decision-gated option — not authorized for agents

A structural reduction exists: emit a **third-party-only dependency manifest** alongside the
full SBOM and re-point dependency-structure evidence (for example Story 1.2 and Story 3.1) at
the stable manifest, so first-party source edits stop invalidating it. Estimated benefit is
roughly two to three days over the remaining plan — batching already removes most cascade
cost, and a code-generation floor of about 38 days bounds the rest. Estimated cost is a
one-time invalidation of the **48 artifacts** that hash `scripts/supply_chain.py`, plus
re-pointing roughly 20 artifacts' inputs. **This requires an accepted Decision record. Do not
implement it on agent initiative.**

## 6. Execute within the Decision 0046 scope freeze

Decision 0046 activates the stabilization scope freeze over Epics 0 through 8, 10, 11, and
the Universal Story Definition of Done. Select the next unit of work under Decision 0021
**from that set only**. Do not execute Epics 9 or 12 through 16 while the freeze is active.
Do not close any row marked `blocked` for physical-platform unavailability by substitution.
Epic 10 executes through the Decision 0040 local Windows 11 KVM guest lane once its external
prerequisites exist.

## 7. Review-pin renewal is routine automation, not a human act

Several story gates pin an immutable `REVIEWED_COMMIT` / `REVIEWED_TREE` and compare a fixed
`REVIEWED_PATHS` set against it (for example `scripts/story_2_2_gate.py`). The reviewer identity
in those gates is the gate implementation itself; its own rationale states that no external-human
review claim is made. When a file inside `REVIEWED_PATHS` changes for a legitimate reason —
a regenerated `requirements/registry.json` after `Agent-Scaffolding-Inventory.md` changed, for
example — the sanctioned action is to **advance the pin to the commit containing the change,
rebuild the gate report (`story-N.N:gate:build`, then any aggregating `sprint-N:gate:build`),
rerun the gate check and its unit test, and record the reason in the run log.** The pin has been
advanced this way six times in repository history. Do not stop and report for this; it is not a
stop condition. Do stop if the change to a reviewed path is unexplained or unintended.

---

**Diagnosis of record:** `~/dev/10_obsidian/09-LLM_Handoff_Records/2026-09-01 2230 - SESSION
HANDOFF - Full Day Ledger, Counsel Retained, AgentMage Cascade Diagnosis.md`, §4.
