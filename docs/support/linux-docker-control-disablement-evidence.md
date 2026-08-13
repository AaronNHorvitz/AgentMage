# Linux Docker Control-Disablement Evidence

This offline KVM procedure establishes one admitted Docker topology and then
changes each required isolation control independently on Fedora and Ubuntu. The
seven cases cover daemon privilege, Docker socket ownership, private API
binding, guarded-container reachability, immutable image identity, resource
limits, and zero egress.

Each case starts from a fresh topology, changes one declared fact, invokes the
production collector and preflight, and requires that control's exact stable
refusal code. The case must not admit Docker, select native inference as an
automatic fallback, expose a native listener, or perform inference. The runner,
guard, runtime peer, temporary group membership, test listener, bootstrap
material, virtual machine, and overlay are removed before completion.

```bash
npm run evidence:story9.2-docker-controls:build
npm run evidence:story9.2-docker-controls:check
```

Prepared digest-pinned guest images and model content are reused without
network access. This is independent control-disablement evidence, not model
quality, physical-host certification, or release-support evidence.
