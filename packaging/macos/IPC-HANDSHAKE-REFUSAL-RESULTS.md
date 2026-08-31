# macOS IPC Handshake Refusal Results

Task 8.1.3.1 (`S-008-UT01`) requires native evidence that the signed host
refuses every absent or wrong security field before admitting the Visual Studio
Code bridge. The Swift test source defines the exact 17-case refusal matrix:

- absent and wrong code identity, audit token, Team ID, bundle ID, App Group,
  protocol version, fresh challenge, and socket state;
- one replay of an already accepted fresh challenge; and
- a valid handshake after every non-replay refusal on the same admission
  object, proving that malformed or mismatched input did not consume the
  one-use challenge.

The pinned Apple Silicon acceptance harness runs that matrix through the exact
signed package. It writes `ipc-handshake-refusal-results.json` beneath an
owner-only evidence directory and exactly one owner-only raw log per case under
`refusal-logs/<case-id>.log`. The closed JSON record binds the source revision,
version, macOS and Xcode builds, arm64 architecture, Team ID, package digest,
required controls, ordered cases, exact refusal codes, and challenge-consumption
dispositions. It retains no audit-token bytes, launch secret, credential,
environment value, or private path.

Assemble the native evidence against the exact release bundle:

```text
python3 scripts/macos_ipc_handshake_refusal_results.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/native-handshake-refusals \
  --results-output /absolute/private-review/ipc-handshake-refusals.json
```

An independent reviewer reconciles the assembled record against the same
release inputs:

```text
python3 scripts/macos_ipc_handshake_refusal_results.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --results /absolute/private-review/ipc-handshake-refusals.json \
  --review-output /absolute/private-review/accepted-ipc-handshake-refusals.json
```

Unknown, missing, duplicated, reordered, accepted, writable, linked,
path-bearing, credential-shaped, private-environment, stale, or release-mismatched
input fails closed. The ingestor only validates and hashes already produced
evidence; it has no socket, process, signing, credential, or support-promotion authority.

The checked-in test, procedure, verifier, and mutation suite are source evidence
only. No native signed host or bridge, live App Group socket, native refusal
run, protected raw log set, independent review, or physical Apple Silicon
execution exists here. Task 8.1.3.1 remains `BLOCKED-MACOS` until the external
artifacts are produced and reconciled.
