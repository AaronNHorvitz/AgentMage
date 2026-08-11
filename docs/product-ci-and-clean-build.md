# Product CI and Clean-Build Evidence

## Current Product CI Contract

The machine-readable product CI contract is
[`architecture/product-ci-policy.json`](../architecture/product-ci-policy.json).
[`scripts/product_ci.py`](../scripts/product_ci.py) validates that contract,
checks the declared toolchain versions, executes bounded lanes, and emits
sanitized deterministic failure summaries. The GitHub workflow is
[`product.yml`](../.github/workflows/product.yml).

The product workflow has independent required jobs for:

- CI-contract validation;
- Rust and TypeScript formatting;
- Rust, TypeScript, and strict-local-source linting;
- Rust and TypeScript compilation;
- default Rust unit and contract tests;
- Visual Studio Code shell tests; and
- inventory of tests that require native Linux execution.

The separate [`documentation.yml`](../.github/workflows/documentation.yml)
workflow remains the documentation gate. A documentation failure cannot be
represented as a product failure, and a product failure cannot be hidden by a
green documentation result.

The native Linux status job verifies the exact inventory of eleven ignored
tests. Its successful completion means only that the pending inventory is
complete and unchanged. It does not execute those tests and does not constitute
native evidence.

Recommended branch-protection checks are:

- `Documentation / validate`;
- `Product / Product CI Contract`;
- `Product / Product Format`;
- `Product / Product Lint`;
- `Product / Product Build`;
- `Product / Rust Unit and Contract Tests`;
- `Product / VS Code Shell Tests`; and
- `Product / Native Linux Evidence (Pending Execution)`.

Branch protection is hosted-repository configuration and is not claimed as
configured by this repository. Release qualification separately requires the
native tests to execute on an admitted native runner.

Run the local equivalents with:

```bash
npm run product-ci:check
npm run product:check
python3 scripts/product_ci.py --inventory-native
```

## Clean-Build Source Identity

The clean-build policy is
[`architecture/clean-build-policy.json`](../architecture/clean-build-policy.json).
New schema-v2 evidence binds the complete recursive Git tree to all four of:

- the resolved commit object identity;
- the resolved tree object identity; and
- the SHA-256 of the deterministic committed-source archive; and
- a canonical path, executable-mode, and content SHA-256 verified inside the
  image before dependency bootstrap.

There are no tracked-path exclusions. Before a build begins, the runner rejects
a dirty index, a dirty worktree, every untracked path, tracked symbolic links,
Git links, and ignored paths outside the policy's explicit local-data and
transient-output exclusions.

Dependency bootstrap occurs while the container image is built from the exact
committed archive. The verification container then starts with
`--network=none`, a read-only root filesystem, all capabilities dropped, no new
privileges, a fixed unprivileged identity, and fresh temporary writable storage.
Client offline flags remain defense in depth; they are not the network boundary.
Supply-chain outputs are regenerated and validated only in disposable storage;
that result does not rewrite or promote retained historical artifacts.

The retained schema-v1 clean-build report is preserved byte-for-byte as
historical evidence. The checker replays it against its recorded revision when
that Git object is available, but it never promotes that report to current
release evidence. A schema-v2 report is current only when its complete source
identity equals an explicitly reviewed revision and that reviewed worktree is
clean.

Generate Linux evidence only from a clean reviewed commit:

```bash
npm run clean-build:run
python3 scripts/clean_build_evidence.py --require-reviewed-revision HEAD
```

The checked-in report is an audit record. Release automation should retain or
publish the report against the exact reviewed commit without treating a later
report-storage commit as the source that was tested.
