# Native Ubuntu Control Evidence

This procedure reproduces AgentMage's native Ubuntu 26.04 kernel evidence for
`S-009-UT01`, `S-009-ST01`, and `S-009-UT02`. It starts an official Ubuntu
cloud image with KVM, runs the committed Linux platform and kernel tests as UID
and GID `10001`, and destroys the disposable guest state after the run.

This is native Ubuntu-kernel evidence in a virtual machine. It is not a claim
that AgentMage has been certified on every physical Ubuntu host.

## Boundaries

The process has two intentionally separate stages:

1. **Bootstrap:** Download the official Ubuntu 26.04 cloud image over HTTPS,
   verify its fixed SHA-256 digest, install the closed package list, remove the
   bootstrap SSH authorization, clean cloud-init state, and retain a local
   digest-bound prepared image under `~/.cache/agentmage/ubuntu-vm/`. The
   bootstrap overlay has a fixed sparse 12 GiB virtual capacity so package
   installation cannot exhaust the cloud image's intentionally small root
   filesystem; only written blocks consume host storage.
2. **Acceptance:** Build test binaries from one committed revision in the
   pinned Ubuntu build image with networking disabled, start a disposable KVM
   overlay with QEMU `restrict=on` and one loopback-only SSH forward, prove an
   external connection fails, execute the tests, clean the synthetic keyring,
   power off the guest, and remove the overlay and ephemeral SSH key.

The acceptance guest receives no private workspace, model, account credential,
or host secret. Its fresh Secret Service keyring exists only for synthetic
store, lookup, clear, and operational-key tests. The harness verifies that no
AgentMage item or keyring file remains before shutdown and never contacts the
host Secret Service.

## Prerequisites

- Fedora 44 x86-64 with readable and writable `/dev/kvm`;
- rootless Podman and the pinned
  `localhost/agentmage-clean-build:ubuntu-x86_64` image;
- `ssh`, `scp`, `ssh-keygen`, and `curl` on the host; and
- QEMU, `qemu-img`, and `cloud-localds` either on the host or in the default
  `fedora-toolbox-44` toolbox.

The toolbox is only a launcher for host KVM/QEMU. It is not the Ubuntu test
kernel. The report records the QEMU executable digest and the guest verifies
`systemd-detect-virt=kvm`, Ubuntu 26.04, the Ubuntu kernel release, x86-64,
cgroup v2, and the unprivileged numeric test identity.

## Bootstrap

Run this only when the prepared image is missing or its pinned inputs change:

```bash
npm run evidence:story9.1-linux-ubuntu-native:bootstrap
```

The command may use the network. It accepts no credential arguments. To replace
an existing verified prepared image deliberately, append `--
--force-bootstrap`. A failed digest, incomplete package set, inaccessible KVM
device, or incomplete guest cleanup fails closed.

## Run And Verify

Commit the exact source under review, then run:

```bash
npm run evidence:story9.1-linux-ubuntu-native:build
npm run evidence:story9.1-linux-ubuntu-native:check
```

The build command proves all of the following under the Ubuntu kernel:

- 11 live Bubblewrap worker tests and eight required zero-escape attack classes;
- observed `NoNewPrivs: 1`, seccomp mode 2, and enforced runtime termination;
- cgroup v2 and the five declared systemd resource-limit properties;
- eight authenticated Unix-socket IPC tests;
- four static and three live Secret Service tests using only synthetic values;
- two complete startup-control mapping tests, one live seven-control preflight,
  and the shared kernel activation-refusal test; and
- root ownership, non-writability, package version, canonical path, and SHA-256
  identity for Bubblewrap, the path executor, `secret-tool`, and `systemd-run`.

The committed report is
`artifacts/sprints/sprint-9/story-9.1/linux-native-ubuntu-control-verification.json`.
It stores no SSH key, keyring password, synthetic secret, private path, raw
diagnostic, or guest disk. It makes no supported-release, physical-host, model,
inference, or macOS claim.

## Cleanup Check

The harness refuses to publish evidence unless the QEMU process is gone, the
loopback SSH listener is absent, the disposable overlay and test binaries are
removed, and both halves of the ephemeral SSH key are absent. The prepared
network-bootstrap image remains cached by design and is verified by digest
before each acceptance run.
