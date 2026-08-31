# macOS Packaging

The macOS release lane is a manual, fail-closed ceremony for a disposable,
isolated Apple Silicon runner. It is separate from the budget-gated source-only
GitHub workflow in `.github/workflows/macos.yml`; a successful source build is
not signing, notarization, installation, lifecycle, M5, or release evidence.

`release-runner.sh` is the repository-controlled ceremony entry point. It
accepts one externally approved policy file and one new evidence directory. The
policy carries only public release facts: the exact source revision, version,
runner and Xcode build identities, Team ID, certificate fingerprints, bundle and
App Group identifiers, the prior package path and digest, and the fixed
Keychain profile selector `agentmage-release-notary-v1`. It cannot carry a
password, private key, API key, Apple ID, issuer, or private environment value.
The actual Developer ID Application and Installer private keys and notarization
credentials remain in the release runner's Keychain and never enter the
repository, command environment, command line, retained report, or package.

## Required external ceremony inputs

Before execution, a release owner must provide all of the following outside the
repository:

- a disposable physical Apple Silicon runner whose exact macOS and Xcode build
  identities match the approved policy;
- a detached, clean checkout of the exact 40-character source revision;
- an approved `release-approved` policy derived from real release identities;
- exactly one unambiguous Developer ID Application certificate and exactly one
  unambiguous Developer ID Installer certificate matching the policy
  fingerprints and Team ID;
- the fixed notarization Keychain profile, provisioned without exporting its
  credential values;
- a previously shipped, signed, notarized, stapled package with an exact
  approved digest for rollback;
- an empty, owner-only evidence destination outside the source checkout; and
- the release-only network control, enabled for the bounded notarization phase
  and disabled again before installed-product launch and rollback verification.

The checked-in `release-runner-policy.contract-fixture.json` is deliberately
synthetic and is always rejected by live execution. It documents field closure
only. The ceremony also refuses a branch checkout, dirty source, root or
elevated execution, membership in the macOS `admin` group, a home directory not
owned by the invoking identity, wrong architecture, wrong runner/toolchain
build, ambiguous certificate set,
missing previous package, wrong previous-package digest, existing install, or
an evidence path inside the repository.

## Fixed lifecycle

The script performs one fixed sequence and accepts no shell command, arbitrary
build step, upload destination, alternate install root, or credential argument:

1. attest the isolated runner, detached clean revision, toolchain, certificates,
   prior package, and empty evidence root;
2. run locked Rust, TypeScript, and Swift checks, then archive the fixed
   `AgentMageRelease` Xcode scheme for `arm64` with Developer ID signing;
3. verify all nested signatures, Hardened Runtime flags, identifiers, Team ID,
   designated requirements, and exact entitlements before packaging;
4. build and Developer-ID-sign a per-user flat `pkg`, verify its signature, and
   retain component and package hashes;
5. submit only that outer package with `xcrun notarytool`, require `Accepted`,
   retrieve and inspect the complete notary log even on success, staple the
   ticket, validate it, and run Gatekeeper's install assessment;
6. install as the standard user, run the packaged content-free smoke check,
   uninstall from the one fixed per-user path, and prove absence;
7. reinstall the candidate, exercise rollback to the exact prior package, run
   the prior package's content-free smoke check, and leave that prior valid state
   installed; and
8. scan retained command output for credential-shaped material, write the
   content-free terminal and nine-phase standard-user acceptance reports
   atomically, and destroy private staging.

No step uses `sudo`, logs an environment, invokes `security` with a secret,
passes raw notary credentials, changes Gatekeeper policy, uploads evidence, or
publishes a package. A failed or interrupted step stops the ceremony; its
partial evidence remains non-successful and the package cannot be promoted.

Apple's release guidance requires Developer ID signing, valid signatures,
Hardened Runtime, secure timestamps, and exclusion of `get-task-allow` before
notarization. Apple also documents `notarytool` and `stapler` for automated
notarization, recommends inspecting the notary log even after acceptance, and
uses a distinct Gatekeeper assessment for installer packages:

- <https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution>
- <https://developer.apple.com/documentation/security/customizing-the-notarization-workflow>
- <https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac>
- <https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution>

## Current disposition

The ceremony is source-prepared but has not run. This checkout has no release
Xcode app/project or signed embedded targets, no production release policy or
Developer ID identities, no notarization Keychain profile, no prior signed
package, no isolated physical M5 runner, and no native lifecycle or independent
review evidence. Build, signing, notarization, stapling, Gatekeeper assessment,
install, launch, uninstall, rollback, and support therefore remain
`BLOCKED-MACOS`.
