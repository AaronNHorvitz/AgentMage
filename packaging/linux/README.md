# Linux Packaging

This module defines Fedora RPM and Ubuntu DEB candidates containing the native
host, the Visual Studio Code VSIX, the Apache-2.0 license, and an exact payload
manifest. Normal operation is non-administrator; package-manager installation
may follow the host distribution's administrative policy.

The host verifies regular-file identity, size, mode, and SHA-256 before package
activation. Unsigned candidates are accepted only by the explicit candidate
verification command and cannot satisfy signed-release verification. Generated
packages live under ignored `release-output/` and are never source artifacts.

Current lifecycle coverage includes install, candidate verification, upgrade,
corrupt-upgrade refusal, rollback, uninstall, and residue checks in disposable
Fedora and Ubuntu containers. External release signing and the trusted VS Code
to-host bootstrap remain blocked release work.
