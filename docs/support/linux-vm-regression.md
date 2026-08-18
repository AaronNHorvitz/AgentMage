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
dependency-acquisition, connected-adapter, and strict-offline guest phases and
the complete independent Fedora and Ubuntu product matrices remain open under
Tasks `9.1.4.3` and `9.1.4.4`. KVM evidence is not physical-host or release
evidence.
