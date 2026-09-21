# Decision 0057: Package-Lifecycle Environment and Declared Platform Prerequisites

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | The container environment used by `linux-clean-package-lifecycle` evidence |
| Authority | Decision 0054 standing owner delegation |
| Preserves | The immutable digest-pinned base image control, the mutable-tag refusal control, `--network=none` during lifecycle steps, and full `rpm`/`dpkg` dependency checking |
| Changes no control in this record | Correct. This decision fixes the direction; the implementation is the next work unit |

## Finding

`npm run evidence:story9.1-linux-package:build` fails at the first container install step:

```text
error: Failed dependencies:
  bubblewrap is needed by agentmage-0.0.0-1.fc44.x86_64
  git-core is needed by agentmage-0.0.0-1.fc44.x86_64
  systemd is needed by agentmage-0.0.0-1.fc44.x86_64
```

This is the package manager behaving correctly. `packaging/linux/agentmage.spec.in` and
`debian-control.in` declare real runtime prerequisites — `Requires` moved from
`glibc, openssl-libs` to include `bubblewrap`, `git-core` and `systemd` when the read-only worker
and isolated Git inspection landed — and `architecture/clean-build-policy.json` already documents
the same set under `runtime_dependencies.platform_packaged`. The lifecycle evidence was never
re-run after that change.

Both pinned base images were probed directly and provide **none** of the three:

```text
docker.io/library/fedora@sha256:89f61a12…   bwrap MISSING  git MISSING  systemctl MISSING
docker.io/library/ubuntu@sha256:7b202b0e…   bwrap MISSING  git MISSING  systemctl MISSING
```

The lifecycle container also runs `--network=none`, so nothing can be resolved at install time.
The declared prerequisites are correct for a supported desktop platform; the bare container base
image is simply more minimal than the platform the package targets.

## Rejected resolutions

1. **`rpm -i --nodeps` / `dpkg --force-depends`.** This would stop the evidence verifying the
   dependency declarations at all, and would hide exactly the class of packaging defect this
   step exists to catch. Rejected as a weakening of the test.
2. **Building a derived image with the prerequisites pre-installed and running the lifecycle in
   it.** `validate_container_lifecycle` requires the lifecycle image to be an immutable published
   reference present in its own `repo_digests`, and
   `test_mutable_image_reference_is_refused_before_inspection` pins that behaviour. A locally
   built image has no repo digest and is addressed by a mutable tag. More importantly the control
   encodes a real assurance property — the lifecycle runs on an unmodified, publicly verifiable
   image that nobody tampered with — and substituting a locally assembled environment would
   silently retire that property. Rejected as a weakening of an evidence binding.

## Decision

The lifecycle keeps the immutable published base image, keeps `--network=none` for every
lifecycle step, and keeps full dependency checking. The declared platform prerequisites are
supplied to the container the same way the AgentMage packages already are:

1. The exact prerequisite package set is resolved **on the host**, where network use is already
   an accepted, separately recorded phase, and each resolved file is recorded with its SHA-256.
2. Those files are mounted **read-only** into the lifecycle container alongside the AgentMage
   packages, exactly as `CONTAINER_PACKAGE_ROOT` already works.
3. A recorded bootstrap step installs them offline inside the container before the lifecycle
   steps begin. The base image, its digest, and the container controls are unchanged.
4. The evidence records the prerequisite closure — every package file, version and digest — so
   the environment is fully reconstructible and auditable, and states plainly that the
   environment is "pinned base image plus the package's own declared prerequisites".

This preserves every existing control. The image reference stays immutable and published, the
mutable-tag refusal stays in force, no lifecycle step gains network, and `rpm`/`dpkg` still
verify dependencies — they simply now find them satisfied, as they would on a supported platform.

## Status of the affected evidence

`artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json` remains at its last
passing revision `36e2d4d2` and is **stale**, not renewed. No attempt was made to re-bind it to
the current source without a passing run. Story 9.1 stays open, and its independent-review gate
stays open and unclaimed.

## Ordering requirement for the implementing run

The package-lifecycle builder requires `clean_build.source_revision == HEAD`. Committing the
clean-build report advances `HEAD` past the revision the report records, which breaks that
equality. The implementing run must therefore: run the clean build, run the package-lifecycle
evidence at that same `HEAD` while the clean-build report is the only worktree change, and then
commit both together.

## Superseded — 2026-09-20

The resolution chosen above was implemented far enough to test against the real pinned images and
**does not work**. Resolving the closure succeeds, but installing it inside the lifecycle container
fails: the closure carries newer versions of packages the immutable base image already has, so the
transaction becomes an upgrade, and upgrades require capabilities that `--cap-drop=all` denies
(`rpm -U` exits 29 with `erase skipped` for `glibc`, `openssl-libs` and others, leaving `bwrap`,
`git` and `systemctl` absent).

**Decision 0059** supersedes this record's chosen resolution, states the root cause — an immutable
base image that predates the repositories a current closure resolves from — and identifies the
Decision 0040 KVM installed-platform lane as the place where dependency-satisfied installation is
already demonstrated. The finding above, and the two resolutions this record rejected, remain
accurate and are preserved unchanged.
