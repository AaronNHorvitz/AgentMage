# Linux Clean-Image Acceptance

This procedure reproduces `S-009-AT01` against the pinned Fedora 44 and Ubuntu
26.04 x86-64 container images. It builds disposable graphical test images,
builds the unsigned AgentMage candidate from committed source, installs the
native package, launches the packaged Visual Studio Code extension, exercises
the registered provider, uninstalls it, and scans for active product residue.

The image-bootstrap stage requires network access to obtain the pinned base
image dependencies and the versioned Visual Studio Code archive. The acceptance
stage runs with `--network=none`. Do not place credentials or private workspaces
in the test images.

## Prerequisites

- rootless Podman;
- the repository's pinned Rust, Node.js, and npm toolchains;
- `curl` and `sha256sum`; and
- a clean checkout of the revision under review.

## Bootstrap the test images

Download the exact Visual Studio Code 1.132.0 archive and verify it before any
container build:

```bash
curl --fail --location \
  --output /var/tmp/vscode-linux-x64-1.132.0.tar.gz \
  https://update.code.visualstudio.com/1.132.0/linux-x64/stable
printf '%s  %s\n' \
  acdaf0fa557bda1720956ff65ca0de0965e92d68f97e2db22341984400937aed \
  /var/tmp/vscode-linux-x64-1.132.0.tar.gz | sha256sum --check --strict -
npm run evidence:story9.1-linux-acceptance:bootstrap -- \
  --vscode-archive /var/tmp/vscode-linux-x64-1.132.0.tar.gz
```

The bootstrap command verifies the archive again and builds from immutable
Fedora and Ubuntu base-image digests. Network access ends when this stage ends.

## Run acceptance

Commit the exact source under review, then run:

```bash
npm run evidence:story9.1-linux-acceptance:build
npm run evidence:story9.1-linux-acceptance:check
```

The build command launches each local acceptance image through rootless Podman
with `--network=none`, all Linux capabilities dropped, no-new-privileges, fixed
process and memory bounds, and read-only package and probe mounts. Container
root is used only for RPM or DEB installation and removal. Every AgentMage
binary, Visual Studio Code command, provider probe, process inspection, and
residue inspection runs as numeric UID/GID `10001:10001`.

The test-only graphical harness passes `--no-sandbox` to Chromium because a
nested Chromium sandbox cannot initialize inside the outer capability-free
rootless container. The outer container remains the acceptance sandbox. This
flag is not an AgentMage runtime setting and is not a supported production
launch option. The harness also marks its generated empty workspace as trusted;
it never mounts a repository or user file into that workspace.

The committed report is
`artifacts/sprints/sprint-9/story-9.1/linux-clean-image-acceptance.json`. A pass
means both distributions completed the exact ordered steps and left no active
extension registration, package record, package-owned path, AgentMage runtime
process or socket, isolated test profile, or disposable container. It does not
claim native physical-host certification, signed release readiness, enabled
model operation, inference, or macOS evidence.
