# Decision 0056: Package-Scaffold Apache-2.0 Test Fixture

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | The five failing `package_scaffold` tests in `agentmage-capability-repository-map` |
| Authority | Decision 0054 standing owner delegation |
| Preserves | Decision 0051 Business Source License, the repository's own `LICENSE` and `NOTICE`, the scaffolder's Apache-2.0 output contract, and every assertion in the affected tests |
| Makes no licence choice | Correct. No licence was selected, changed, or reinterpreted by this decision |

## Problem

`npm run product:test` failed inside the reproducible clean-build container with five
failures, all `LicenseMismatch`, in `capabilities/repository-map/src/package_scaffold.rs`.
These are the long-recorded "five existing Apache scaffold fixture failures" noted in
`docs/DEMO-PROGRESS.md`.

The cause is a test-input defect, not a product or licensing defect. The scaffolder
generates **new user projects** under Apache-2.0 and pins the exact artifact it will emit:

```text
const APACHE_2_LICENSE_SHA256: &str =
    "02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f";
```

The shared test request helper fed it `include_bytes!("../../../LICENSE")` — the
**AgentMage repository's own root licence**. That was coincidentally correct while this
repository was itself Apache-2.0. When Decision 0051 relicensed AgentMage to the Business
Source License 1.1, the repository `LICENSE` became BSL
(`3c0fe2e36ea4933e3fbf92bc2bac81ae05bc47800e85be7e1bb40ede19c5e5f7`), the helper began
feeding BSL bytes into an Apache-2.0 scaffolder, and all five tests panicked at
`LicenseMismatch` before reaching a single assertion.

## Decision

1. The exact Apache-2.0 text that `APACHE_2_LICENSE_SHA256` pins is retained in the
   repository as an explicit test fixture at `fixtures/licensing/apache-2.0.txt`. Its
   SHA-256 is `02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f`, matching
   the pinned constant exactly.
2. The shared test helper reads that fixture instead of the repository's own `LICENSE`. A
   test for a scaffolder that emits Apache-2.0 projects supplies Apache-2.0 bytes; it must
   not depend on whatever licence this repository happens to carry.
3. Nothing else changes. The constant, every scaffolded-output `Apache-2.0` declaration,
   the repository's `LICENSE`, and its `NOTICE` are byte-identical to before.

## Why this is not a licence choice

The owner withheld licence and trademark choices from delegated authority, so the boundary
is stated explicitly:

- AgentMage remains licensed under the **Business Source License 1.1** per Decision 0051.
  `LICENSE` and `NOTICE` were not touched.
- The scaffolder still emits **Apache-2.0** projects, exactly as before. The pinned digest
  and all seven `Apache-2.0` output declarations are unchanged.
- The only change is which bytes a **test** passes as input.

## Why this does not weaken a test

The five tests previously panicked inside the shared helper and never evaluated their
assertions. With correct input they execute those assertions and pass. The suite moved from
43 passed / 5 failed to **48 passed / 0 failed** with no assertion, threshold, or fixture
expectation relaxed.

## Provenance of the fixture

The Apache-2.0 text was recovered from this repository's own history rather than fetched
from a third party: commit `b4b8f4d8` carries a `LICENSE` blob whose SHA-256 is exactly the
pinned value. That blob is the artifact the constant was computed from, so the fixture is a
byte-exact restoration of a file this repository already published.
