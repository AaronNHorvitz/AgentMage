# Product CI and Clean-Build Evidence

## Current Product Validation Contract

Decision 0040 makes routine product and documentation validation local-first.
The machine-readable contract is
[`architecture/product-ci-policy.json`](../architecture/product-ci-policy.json).
[`scripts/product_ci.py`](../scripts/product_ci.py) validates that contract,
checks the declared toolchain versions, executes bounded local lanes, and emits
sanitized deterministic failure summaries.

The local product lanes remain independent commands for:

- Rust and TypeScript formatting;
- Rust, TypeScript, strict-local-source, and hostile-network linting;
- Rust and TypeScript compilation;
- default Rust unit and contract tests;
- Visual Studio Code shell tests; and
- inventory of tests requiring native Linux execution.

Run them with:

```bash
npm run product-ci:check
npm run product:check
python3 scripts/product_ci.py --inventory-native
```

The native inventory currently contains 19 tests. A successful inventory check
means only that the declared pending set is complete and unchanged. It does not
execute those tests and does not constitute Fedora or Ubuntu evidence.

The documentation gate remains independently executable through:

```bash
npm run docs:clean-check
```

### GitHub-hosted execution

[`product.yml`](../.github/workflows/product.yml) and
[`documentation.yml`](../.github/workflows/documentation.yml) are retained
no-runner sentinels. They have only a manual trigger and their sole jobs are
unconditionally skipped before runner allocation. Ordinary pushes, pull
requests, schedules, and workflow chaining cannot use them.

[`macos.yml`](../.github/workflows/macos.yml) is the only workflow allowed to
allocate a GitHub-hosted runner. It:

- runs only through `workflow_dispatch`;
- requires the maintainer to confirm that Actions budget is available;
- uses the standard Apple Silicon `macos-15` runner;
- has read-only repository permissions;
- receives no signing, notarization, or other secret;
- builds and tests only `platforms/macos`; and
- cannot establish MacBook Pro M5, signing, packaging, lifecycle, or support
  evidence.

No disabled sentinel status should be configured as a required branch check.
The absence of a hosted check is not a pass. Local result bundles and later
native-platform evidence must identify the exact source revision and execution
venue.

### Local platform execution

First-GA native acceptance is assigned to fresh local KVM guests for Fedora
x86_64, Ubuntu x86_64, and properly licensed Windows 11 x64. The future
controller must admit immutable base-image identities, create one isolated
overlay per run, use a standard user, separate dependency-acquisition and
offline phases, record allowlisted environment facts and exact commands,
export content-free evidence, and destroy transient state.

That controller and its complete image manifests are not implemented by
Decision 0040. Existing Fedora, Ubuntu, container, KVM, and historical
`windows-2022` artifacts retain their exact scopes. None is relabeled as the
new complete local-VM or Windows 11 result.

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
