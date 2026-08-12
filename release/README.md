# Release Tooling

Release tooling produces deterministic builds, manifests, hashes, bills of
materials, evidence checks, and packages from declared inputs. It cannot waive a
failed or blocked gate.

The current `xtask` can build deterministic unsigned RPM, DEB, and VSIX
candidates and a separate deterministic signable Linux release bundle. The
Linux payload binds the host, inactive isolated native-inference adapter, VSIX,
and license as four exact files. Adapter presence enables no model or inference.
The signer accepts exactly one raw Ed25519 private seed through standard input,
checks it against an external public key, and emits a new detached signature.
It does not accept a private-key path, generate an identity, overwrite a
signature, or place trust material inside the package.

A signable bundle is not a release. Production signing identity approval,
independent trust-root delivery, RPM/DEB repository signatures, trusted Visual
Studio Code host bootstrap, clean release lifecycle evidence, and release review
remain mandatory.
