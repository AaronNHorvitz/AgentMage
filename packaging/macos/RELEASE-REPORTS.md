# macOS Native Release Reports

Task 8.1.2.2 retains five reviewable report families from the isolated macOS
release ceremony: exact entitlements, code-signing identity, notarization,
stapling, and Gatekeeper assessment. The reports are produced only after the
release runner has completed successfully. They are not inputs to signing and
cannot authorize or repeat any native effect.

Run the source-bound assembler on the isolated runner after the ceremony:

```text
python3 scripts/macos_release_reports.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/evidence \
  --reports-output /absolute/private-review/release-reports.json
```

The assembler accepts only owner-matching, single-link regular files beneath
an owner-only evidence directory. It finds exactly one raw log for each fixed
ceremony step, parses the four entitlement plists, requires the successful
terminal record, and binds every retained raw output by SHA-256. It neither
invokes `codesign`, `notarytool`, `stapler`, or `spctl` nor accepts credentials,
environment dumps, arbitrary commands, URLs, or upload targets.

After the release-derived manifest exists, a reviewer runs the independent
reconciliation path with the exact five ceremony products:

```text
python3 scripts/macos_release_reports.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --reports /absolute/private-review/release-reports.json \
  --review-output /absolute/private-review/accepted-release-reports.json
```

The verifier requires exact source, version, OS build, Xcode build,
architecture, Team ID, certificate fingerprints, bundle IDs, designated
requirements, component hashes, App Group, Keychain group, entitlement closure,
package digest, Accepted notarization with no issues, validated staple, and
accepted install-type Gatekeeper assessment. Any absent, extra, stale,
substituted, writable, linked, credential-shaped, unsuccessful, or mismatched
fact fails closed.

The accepted record contains only identities, sizes, hashes, and boolean
dispositions. It explicitly records that the ingestor executed no native tool
and makes no macOS support or release-approval claim. The checked-in source
contract is synthetic source evidence only: no native report, signed package,
notary result, staple, Gatekeeper assessment, independent review, or physical
Apple Silicon execution exists here. Task 8.1.2.2 remains `BLOCKED-MACOS` until
the external ceremony and reviewer supply those exact artifacts.
