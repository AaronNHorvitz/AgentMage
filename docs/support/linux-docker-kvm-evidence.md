# Linux Docker KVM Evidence

This procedure prepares reusable Fedora 44 and Ubuntu 26.04 KVM guests for the
live Docker topology work in Sprint 9 Task `9.2.2`. It does not itself activate
AgentMage Docker mode or make a release-support claim.

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
- Prepared images and metadata remain under `~/.cache/agentmage/docker-kvm/` and
  are ignored reproducible test inputs, not repository artifacts.

## Commands

```bash
npm run evidence:story9.2-docker-kvm:bootstrap
npm run evidence:story9.2-docker-kvm:check
```

Use `--force-bootstrap` only together with `--bootstrap-images` when an existing
prepared image has already failed exact metadata or digest validation. The
acceptance and hostile-reachability commands will be added only after their
source and mutation validators are committed.
