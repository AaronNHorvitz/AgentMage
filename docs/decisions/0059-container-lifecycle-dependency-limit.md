# Decision 0059: Container Lifecycle Cannot Demonstrate Dependency-Satisfied Install

| Field | Value |
|---|---|
| Status | Accepted under owner delegation, 2026-09-20 |
| Date | 2026-09-20 |
| Scope | The install step of `linux-clean-package-lifecycle` container evidence |
| Authority | Decision 0054 standing owner delegation |
| Supersedes | Decision 0057's chosen resolution, which was implemented far enough to be tested and disproven |
| Preserves | Every container control: immutable published base image, `--network=none`, `--cap-drop=all`, `no-new-privileges`, `--pids-limit=64`, `--memory=512m`, read-only package mount |

## What Decision 0057 proposed, and what testing showed

Decision 0057 directed resolving the package's declared prerequisites on the host, hashing them,
mounting them read-only, and installing them offline in a recorded bootstrap step before the
lifecycle. That design was implemented far enough to test against the real pinned images, and it
does not work. Three experiments, all under the lifecycle's exact controls:

1. **Resolution succeeds.** `dnf download --resolve` from the pinned
   `docker.io/library/fedora@sha256:89f61a12…` produced a 48-package, 38 MB closure for
   `bubblewrap git-core systemd`.
2. **`rpm -i` of that closure fails**, because the closure carries newer versions of packages the
   image already has:

   ```text
   error: Failed dependencies:
     systemd-libs < 259.9-1.fc44 conflicts with systemd-shared-259.9-1.fc44.x86_64
   ```

3. **`rpm -U` of that closure also fails**, with exit 29, because upgrading requires erasing the
   superseded files and `--cap-drop=all` denies that:

   ```text
   error: glibc-2.43-5.fc44.x86_64: erase skipped
   error: openssl-libs-1:3.5.5-2.fc44.x86_64: erase skipped
   ```

   After the transaction `bwrap`, `git` and `systemctl` were all still absent.

A fourth check ruled out the obvious narrowing: resolving only `bubblewrap git-core`, without
`systemd`, still pulls `util-linux-core-2.41.5` which file-conflicts with the image's installed
`util-linux-core-2.41.4`.

## Root cause

The pinned base image is deliberately immutable and therefore **older than the current
distribution repositories**. Any dependency resolution against those repositories selects current
versions, which means upgrading packages already present in the image. Upgrades require the
capabilities to unlink and replace root-owned files, and the lifecycle container drops all
capabilities by design. The two properties are mutually exclusive: an immutable old base image
and a current dependency closure cannot be reconciled inside a capability-less container.

This also explains the history. The container lifecycle last passed at revision `36e2d4d2`, when
the package declared only `Requires: glibc, openssl-libs` — both already present in the base image,
so the install needed no new packages at all. The step broke when `bubblewrap`, `git-core` and
`systemd` were correctly added as real runtime requirements.

## Decision

1. The container lifecycle **does not** gain a prerequisite bootstrap step. Decision 0057's
   mounting design is withdrawn as disproven rather than left standing as unimplemented direction.
2. No control is relaxed to make the step pass. Specifically rejected, and rejected again here
   with test evidence rather than argument: `--nodeps`/`--force-depends`, granting the container
   package-management capabilities, and substituting a locally built derived image for the
   immutable published one.
3. The dependency-satisfied install is **already demonstrated elsewhere**, in the Decision 0040
   installed-platform lane: `artifacts/sprints/sprint-16/installed-linux-worker-matrix.json`
   records `pass-installed-linux-worker-operation-attack-lifecycle-matrix`, installing the real
   candidate packages in strict-offline native Fedora 44 and Ubuntu 26.04 KVM guests under QEMU
   10.2.2, where a real operating system supplies the prerequisites and the privileges a package
   manager needs. That lane, not a capability-less container, is the correct home for this
   assurance.
4. `artifacts/sprints/sprint-9/story-9.1/linux-clean-package-lifecycle.json` therefore remains
   **stale at its last passing revision `36e2d4d2`** and Story 9.1's container-lifecycle element
   stays open and explicitly blocked. It is not re-bound, not regenerated, and not claimed.

## What would reopen the container lane

Any of these is an owner-level product choice, not a delegated implementation detail, because each
changes either what the package declares or what the evidence asserts:

- advance the pinned base images so their package set matches the repositories the closure is
  resolved from, accepting the periodic re-pinning that implies;
- give the lifecycle container the capability set real package management needs, and restate the
  isolation the evidence claims;
- split the artifact so container evidence covers payload, modes, upgrade, rollback, uninstall and
  residue with dependency checking explicitly out of scope, paired with a separate strict assertion
  that the declared dependency set equals the documented platform prerequisites.

None of these was taken. The blocker is recorded exactly rather than resolved by weakening.
