# Linux VM Regression Authority

Decision 0040 assigns Fedora and Ubuntu native regression to disposable local
KVM guests. The admitted base catalog is
`architecture/linux-vm-base-images.json`; it contains no credential or private
host path and permits no release claim.

## Base And Overlay Checks

Run the contract and cached-image checks from a clean checkout:

```bash
npm run linux-vm-regression:check
npm run linux-vm-regression:verify-bases
```

Exercise the non-destructive overlay lifecycle only from a clean exact source
revision:

```bash
npm run linux-vm-regression:exercise-overlays
```

The controller verifies KVM and QEMU, rehashes both admitted upstream bases,
creates one fresh qcow2 overlay at a time, checks its backing identity, writes
path-free evidence before cleanup, removes the overlays, and verifies the
transient directory is absent. It does not start a guest, access the network,
or inject a repository credential.

## Current Boundary

The retained overlay report proves only Tasks `9.1.4.1` and `9.1.4.2`. The
three-phase guest runner is available through:

```bash
npm run linux-vm-regression:phases:build
npm run linux-vm-regression:phases:check
```

It executes dependency acquisition and local connected-adapter observation,
restarts the same disposable overlay under restricted networking, proves an
external connection is denied, runs its committed contract and focused tests,
and retains only bounded hashes, counts, package identities, and cleanup state.
This closes Task `9.1.4.3`. The independent promoted matrix controller is run
from a clean exact source revision with the pinned native-runtime input:

```bash
AGENTMAGE_LLAMA_CPP_ARCHIVE=/path/to/pinned-b10333.tar.gz \
  npm run linux-vm-regression:matrix:build
npm run linux-vm-regression:matrix:check
```

Each distribution gets a distinct fresh overlay. Dependency and browser
acquisition occur before the restart into restricted networking. Product,
documentation, package, lifecycle, security, automated accessibility,
recovery, removal, native-runtime, and Docker-compatibility lanes then execute
inside that guest. Native and Docker receipts are separate and cannot satisfy
one another. Automated accessibility does not substitute for human review,
and KVM evidence is not physical-host or release evidence.

The current retained result is
`artifacts/sprints/sprint-9/story-9.1/linux-vm-promoted-matrix.json`. It closes
Task `9.1.4.4` and the local Linux VM regression task. Story 9.1 remains open
for its required independent critical-boundary review; this project cannot
self-certify that review.
