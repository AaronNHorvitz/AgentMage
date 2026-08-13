# Linux Packaging

This module defines Fedora RPM and Ubuntu DEB candidates containing the native
host, the isolated inactive native-inference adapter, the Visual Studio Code
VSIX, the Apache-2.0 license, and an exact payload manifest. Normal operation is
non-administrator; package-manager installation may follow the host
distribution's administrative policy.

The host verifies regular-file identity, size, mode, and SHA-256 before package
activation. Unsigned candidates are accepted only by the explicit candidate
verification command and cannot satisfy signed-release verification. Generated
packages live under ignored `release-output/` and are never source artifacts.
The packaged adapter has zero enabled models and no inference operation until
the candidate-neutral runtime gate is implemented and independently admitted.
The separately generated b10333 CPU-library runtime bundle is not embedded in
the RPM, DEB, VSIX, or signable release bundle. It remains an ignored,
identity-pinned, inactive input until later installer, runtime, model, and
release gates admit it.

Current lifecycle coverage includes install, candidate verification, upgrade,
corrupt-upgrade refusal, rollback, uninstall, and residue checks in disposable
Fedora and Ubuntu containers. Decision 0022 adds a separate version 2 signable
bundle and detached Ed25519 verifier. The signature and public trust root remain
external to the package, and the private seed is accepted only through the
isolated signer standard input. Production identity approval, independent
trust-root delivery, RPM/DEB repository signing, and the trusted VS Code-to-host
bootstrap remain blocked release work.

## Clean candidate lifecycle

Run the complete source build separately through `npm run clean-build:run`, then
exercise package installation and recovery against the pinned minimal base
images with:

```bash
python3 scripts/package_lifecycle.py \
  --output release-output \
  --with-containers \
  --fedora-image docker.io/library/fedora@sha256:89f61a124414261868224666aa7fb8df1b78397a53623774bdfb105d1612b48b \
  --ubuntu-image docker.io/library/ubuntu@sha256:7b202b0e2e0028c6250f5fcf41d04df492d145a1654c6995a6553f0c1f6f1960
```

The command uses rootless Podman with local images only, disables container
networking, drops all capabilities, and mounts a temporary package-only
directory read-only. Container root is limited to the inert container lifecycle
process and native package-manager operations. Both AgentMage executables,
package verification, component
manifest inspection, ownership checks, and residue scans run as numeric user
`10001:10001`. A passing run covers clean install, corrupt-upgrade refusal,
upgrade, rollback, uninstall, reinstall recovery, and a final residue-free
uninstall on Fedora 44 and Ubuntu 26.04.

The separate [Linux clean-image acceptance procedure](../../docs/support/linux-clean-image-acceptance.md)
adds an actual Visual Studio Code extension-host launch and provider-contract
exercise on both distributions. It keeps dependency bootstrap separate from
the networkless acceptance run and records the test-only nested Chromium
sandbox limitation explicitly.
