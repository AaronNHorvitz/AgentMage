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

Measured input-reference counts across 373 evidence artifacts:

| Input | Artifacts referencing it |
|---|---|
| `kernel/engine/src/operational_store.rs` | 36 |
| `kernel/engine/src/lib.rs` | 31 |
| `SECURITY-REVIEW.md` | 29 |
| `Cargo.lock` | 28 |
| `Cargo.toml` | 25 |
| `package.json` | 20 |
| `scripts/revision_evidence.py` | 17 |
| `shells/vscode/package.json` | 11 |

Exposing a capability requires editing `kernel/engine/src/lib.rs` to register the module,
which alone invalidates 31 artifacts. A dependency change adds 53 more via `Cargo.lock` and
`Cargo.toml`. **A single capability addition invalidates on the order of 100 artifacts.**

Capabilities in the same work unit invalidate **the same** inputs. Regenerating per
capability repeats identical work N times for no additional assurance. **The cascade cost is
per batch, not per item.**

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

## 3. Sequence work to share invalidation

When choosing the next unit of work under Decision 0021, prefer grouping items that touch the
**same** hotspot inputs listed in §1. Work that touches `kernel/engine/src/lib.rs` should be
completed together before the evidence pass, not interleaved with unrelated work.

## 4. Report honestly

State what is open. Do not claim platform, runtime, model, or integration support that has not
been demonstrated. `architecture/status-model.json` is the authority on current status; leave
`lifecycle_status`, `verification_status`, and the "Current Implementation Truth" block in
`TASKS.md` accurate at all times.

---

**Diagnosis of record:** `~/dev/10_obsidian/09-LLM_Handoff_Records/2026-09-01 2230 - SESSION
HANDOFF - Full Day Ledger, Counsel Retained, AgentMage Cascade Diagnosis.md`, §4.
