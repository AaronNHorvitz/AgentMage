# Linux Docker Reachability Evidence

This offline KVM procedure probes the private Docker Model Runner endpoint from
the declared hostile positions on Fedora and Ubuntu. LAN, host, ordinary-user,
VS Code extension, tool-worker, ordinary-container, and separate-namespace
positions must all fail to connect.

The positive control uses an evidence-only runtime peer whose exact process and
cgroup identity is supplied to the production guard. It completes the fresh
challenge handshake and sends a deliberately disallowed HTTP request. The guard
must terminate with `docker-guard.service.request-policy`, proving peer
authentication without forwarding to Model Runner or performing inference.
The disposable test delivers the secret through a private evidence-only file so
Python can construct the transcript; it removes that file before connecting and
does not represent this as the production inherited-channel mechanism.

```bash
npm run evidence:story9.2-docker-reachability:build
npm run evidence:story9.2-docker-reachability:check
```

Guests use prepared digest-pinned images, disposable overlays, restricted QEMU
networking, freshly rebuilt AgentMage packages, and complete process and secret
cleanup. This is reachability evidence, not model-quality or release evidence.
