# macOS Signed Reference Package Evidence

Task 8.1.2.1 accepts a signed reference package only as the output of the fixed
isolated-runner ceremony. `scripts/macos_signed_reference_package.py` ingests
four external files from that one run:

- the `release-approved` non-secret ceremony policy;
- the stapled `AgentMage-<version>.pkg` candidate;
- the release-derived macOS component/package manifest; and
- the content-free successful terminal record emitted after install, launch,
  uninstall, residue, and rollback verification.

The verifier accepts no credential, signing key, notary password, Apple ID,
issuer ID, API key, environment dump, arbitrary command, URL, or upload target.
Every input must be an owner-matching, single-link, non-symlink regular file that
is not group/world writable. The policy and manifest are closed records. The
package digest must agree byte-for-byte across the package, manifest, and
terminal record; the exact source, version, runner, architecture, Team ID,
bundle identifiers, App Group, Keychain group, and minimal entitlement sets
must agree across every applicable record. All four component hashes must be
nonzero, distinct, and release-derived. Designated requirements must bind the
matching bundle identifier and Team ID.

Successful ingestion writes a new content-free record atomically. It records
the hashes of the package, policy, manifest, and terminal inputs, but it does not
copy the package, assert that a signature or ticket was observed on the current
machine, make a support claim, or satisfy Task 8.1.2.2's signing/notarization
reports. Those native facts remain owned by the isolated ceremony and its later
review artifacts.

The checked-in source-contract report is not reference-package evidence. No
signed package, release-derived manifest, production policy, successful
terminal record, Apple Silicon execution, or independent review is present in
this repository. Task 8.1.2.1 therefore remains `BLOCKED-MACOS` until the
external ceremony supplies and a reviewer retains the exact accepted bundle.
