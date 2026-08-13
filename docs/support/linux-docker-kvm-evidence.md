# Linux Docker KVM Evidence

This procedure prepares reusable Fedora 44 and Ubuntu 26.04 KVM guests and runs
the live Docker topology work in Sprint 9 Sub-task `9.2.2.1`. It does not
activate AgentMage Docker inference or make a release-support claim.

## Boundary

- Official distribution images are HTTPS-downloaded and SHA-256 verified.
- Network access exists only while preparing the reusable guests.
- Docker Engine, the exact Model Runner image, and the exact two-file model
  payload are installed during bootstrap.
- The local model source is hash-checked before transfer and again inside each
  Docker-managed volume.
- Bootstrap SSH keys and cloud-init instance identity are removed before the
  prepared image is retained.
- Later acceptance guests use disposable overlays, QEMU restricted networking,
  and one host-loopback SSH forward.
- The exact six-file package is installed, the inactive native adapter is
  self-checked, and the production collector inspects the live Docker topology.
- The disposable guest replaces the package-default service and `fd://` socket
  activation with a minimal hash-recorded unit that requires `containerd` and
  starts a direct `/run/docker.sock` daemon listener. This preserves the
  collector's exact daemon peer-identity check and is an explicit Docker-mode
  prerequisite rather than an ambient host assumption.
- A held ordinary-user acceptance peer and dedicated non-root guard expose only
  bounded identity metadata; no inference request is sent.
- Process, group, capability, namespace, socket, mount, cgroup, image, model
  file, resource, route, and egress metadata must all pass the packaged
  preflight in one fresh replay-protected observation.
- The one-session guard secret is deleted inside the guest and is never written
  to the evidence report.
- Prepared images and metadata remain under `~/.cache/agentmage/docker-kvm/` and
  are ignored reproducible test inputs, not repository artifacts.

## Commands

```bash
npm run evidence:story9.2-docker-kvm:bootstrap
npm run evidence:story9.2-docker-kvm:build
npm run evidence:story9.2-docker-kvm:check
```

Use `--force-bootstrap` only together with `--bootstrap-images` when an existing
prepared image has already failed exact metadata or digest validation. The
acceptance build is a separate offline operation and writes bounded evidence to
`artifacts/sprints/sprint-9/story-9.2/linux-docker-kvm-topology.json`.

The acceptance peer proves the exact topology, not a product inference path.
Native llama.cpp execution, the real kernel-to-guard inference path, model
quality, hostile-position reachability, independent primitive disablement,
release support, and physical-host evidence remain separate gates.
