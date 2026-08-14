# Task 12.2.4.1 D027 State Results

## Result

Pass for `D027-S12-STATE`; no unverified success was admitted.

Focused cases closed: **5 of 5**.

## Cases

| Case | Campaign boundary | Expected result | Result |
|---|---|---|---|
| `D027-STATE-01` | State graph | Evaluate all 289 pairs as exactly 52 legal and 237 illegal edges | Pass |
| `D027-STATE-02` | Terminal behavior | Keep all nine terminal states sticky and gate ordinary success targets | Pass |
| `D027-STATE-03` | Resource ceilings | Admit every inclusive limit, then select one exact sticky terminal result | Pass |
| `D027-STATE-04` | Proposal identity | Reject exact replay and competing proposal without replacing admission | Pass |
| `D027-STATE-05` | False completion | Reject all five advisory sources for both success dispositions | Pass |

## Evidence

- Four `d027_s12_state_*` Rust tests are retained beside the kernel code they
  exercise.
- The graph/controller campaign evaluates every ordered pair, records 48
  ordinary admissions, and sends all 34 ordinary success-target calls to the
  verifier-required path.
- The ceiling campaign checks all 13 resources, including `STALLED` only for
  repeated no progress and `EXHAUSTED` for the other 12 resources.
- The verifier campaign submits ten advisory completion candidates and two
  invalid postcondition candidates; none produces a completion proof.

## Limits

- This campaign proves the in-process deterministic state boundary; encrypted
  persistence wiring and the full crash-before-and-after campaign remain Task
  12.2.4.4.
- Tests use synthetic identifiers and records with no production model,
  classifier, tool, worker, private user data, or external network operation.
- Cross-platform execution, packaging, release acceptance, and manual fuzzing
  remain later tasks and gates.
