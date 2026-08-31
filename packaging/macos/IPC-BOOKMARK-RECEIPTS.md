# macOS IPC Authentication and Bookmark Lifecycle Receipts

Task 8.1.2.3 retains two content-free receipt families from the installed,
signed macOS candidate: authenticated bridge admission and the read-only
security-scoped bookmark lifecycle. These records are post-execution evidence.
They cannot authenticate a peer, create authority, resolve a bookmark, or
repeat a native effect.

The native acceptance harness writes exactly these owner-only files beneath an
owner-only evidence directory:

- `ipc-authentication-receipt.json` records the live audit-token, designated
  requirement, signature, bundle, Team, App Sandbox, App Group, peer
  credential, launch process, protocol, socket-mode, fresh-challenge,
  round-trip, one-use, and replay-refusal dispositions. It retains only a
  digest for the observed peer audit identity, never the audit token itself.
- `bookmark-lifecycle-receipt.json` records the directory-only Powerbox
  selection, alias-disabled picker, app-scoped read-only bookmark, Keychain
  insert/load, no-UI/no-mount resolution, balanced scope access, read success,
  write denial, move and reboot reopen, stale refresh, revocation, and
  post-revocation denial. Resource, volume, and bookmark identities are
  retained only as distinct SHA-256 digests; paths and bookmark bytes are not
  retained.

After the native run, assemble the two records against the exact release
bundle:

```text
python3 scripts/macos_ipc_bookmark_receipts.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/native-receipts \
  --receipts-output /absolute/private-review/ipc-bookmark-receipts.json
```

The assembler accepts only owner-matching, single-link regular files from the
owner-only evidence directory. It requires exact source revision, version,
macOS and Xcode build, arm64 architecture, package digest, Team ID, bundle IDs,
designated requirement, App Group, Keychain access group, protocol limits,
socket modes, and every required successful or denied lifecycle disposition.
Unknown, writable, linked, path-bearing, credential-shaped, private-environment,
partial, stale, replayable, or mismatched input fails closed.

An independent reviewer reconciles the assembled record against the same exact
release bundle:

```text
python3 scripts/macos_ipc_bookmark_receipts.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --receipts /absolute/private-review/ipc-bookmark-receipts.json \
  --review-output /absolute/private-review/accepted-ipc-bookmark-evidence.json
```

The accepted record contains identities, counts, hashes, closed dispositions,
and explicit non-claims only. The ingestor neither invokes native IPC nor
opens a socket, picker, Keychain, or security-scoped resource. It has no
network, signing, credential, package-copy, workspace, or support-promotion
authority.

The checked-in source contract and synthetic mutation suite are source evidence
only. No signed installed host or bridge, native peer, Powerbox selection,
Keychain item, security-scoped bookmark, lifecycle run, independent review, or
physical Apple Silicon execution exists here. Task 8.1.2.3 remains
`BLOCKED-MACOS` until those external artifacts are produced and reconciled.
