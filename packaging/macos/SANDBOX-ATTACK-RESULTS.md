# macOS Sandbox Profiles and Attack Results

Task 8.1.2.4 retains the installed candidate's four native sandbox profiles and
one closed 36-case hostile-access matrix. These records are post-execution
evidence. They cannot enable an entitlement, launch a process, authorize a
workspace, or repeat an attack.

The native acceptance harness writes two owner-only closed JSON records beneath
an owner-only evidence directory:

- `sandbox-profile-results.json` contains exactly one profile for the kernel
  host, Visual Studio Code bridge, XPC tool helper, and Metal inference service.
  Each profile binds the exact signed component, designated requirement,
  component hash, minimal entitlement closure, Hardened Runtime, active App
  Sandbox profile, and absence of network, workspace-write, and temporary
  exception entitlements.
- `sandbox-attack-results.json` contains every cross product of the four
  components and nine attack classes: ambient home, device, process,
  environment, credential, network, workspace write, grant, and cross-user
  access. Every case must have been attempted and must record zero unauthorized
  accesses, bytes, network connections, network bytes, descendants, workspace
  changes, authority broadening, canary observation, and residue.

The evidence directory also contains owner-only `profile-logs` and
`attack-logs` directories. Profile logs use `<component>.log`. Attack logs use
`<component>-<attack-class>.log`. The assembler requires exactly four and 36
safe single-link regular files respectively, computes their SHA-256 digests,
and reconciles each digest against its closed JSON record.

Assemble the native records against the exact release bundle:

```text
python3 scripts/macos_sandbox_attack_results.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/native-sandbox-evidence \
  --results-output /absolute/private-review/sandbox-attack-results.json
```

An independent reviewer then reconciles the assembled artifact against those
same exact release inputs:

```text
python3 scripts/macos_sandbox_attack_results.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --results /absolute/private-review/sandbox-attack-results.json \
  --review-output /absolute/private-review/accepted-sandbox-evidence.json
```

Unknown, missing, duplicated, reordered, writable, linked, path-bearing,
credential-shaped, private-environment, partial, nonzero-access, stale, or
mismatched input fails closed. The accepted record contains only release
identities, closed dispositions, counts, and hashes. It never retains canary
values, private paths, environment values, credentials, or raw output.

The ingestor neither launches a native process nor invokes App Sandbox,
Seatbelt, filesystem, device, process, Keychain, network, or workspace probes.
It has no signing, credential, package-copy, workspace, attack, or
support-promotion authority.

The checked-in source contract and synthetic mutation suite are source evidence
only. No signed installed component, active native sandbox profile, hostile
native probe, protected raw log, independent review, or physical Apple Silicon
execution exists here. Task 8.1.2.4 remains `BLOCKED-MACOS` until those external
artifacts are produced and reconciled.
