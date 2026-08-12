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

Current lifecycle coverage includes install, candidate verification, upgrade,
corrupt-upgrade refusal, rollback, uninstall, and residue checks in disposable
Fedora and Ubuntu containers. Decision 0022 adds a separate version 2 signable
bundle and detached Ed25519 verifier. The signature and public trust root remain
external to the package, and the private seed is accepted only through the
isolated signer standard input. Production identity approval, independent
trust-root delivery, RPM/DEB repository signing, and the trusted VS Code-to-host
bootstrap remain blocked release work.
