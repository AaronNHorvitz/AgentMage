# macOS Lifecycle Recovery Results

Task 8.1.3.3 (`S-008-RT01`) executes exactly eight installed-candidate recovery
scenarios: stale bookmark, revoked bookmark, helper crash, host crash,
interrupted install, failed launch, uninstall, and rollback. Each scenario must
finish in its declared terminal class, complete cleanup, recover the prior valid
state, leave zero descendants and residue, preserve the workspace, and broaden
no authority.

The Swift acceptance matrix fixes scenario order and rejects every incomplete,
substituted, or unsafe result. Its checked-in probe is synthetic and test-local;
the signed native harness supplies the real fault and lifecycle operations on
the pinned Apple Silicon runner.

The native harness writes `lifecycle-recovery-results.json` beneath an
owner-only evidence directory and one owner-only raw log per scenario under
`recovery-logs/<scenario>.log`. The closed record binds the exact release
source, version, macOS and Xcode builds, arm64 architecture, Team ID, package
digest, ordered scenario set, terminal classes, cleanup, and recovery
dispositions. It retains no bookmark bytes, audit token, launch secret,
credential, environment value, workspace content, or private path.

Assemble the native record against the exact release bundle:

```text
python3 scripts/macos_lifecycle_recovery_results.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/native-lifecycle-recovery \
  --results-output /absolute/private-review/lifecycle-recovery-results.json
```

An independent reviewer reconciles the assembled result against the same
release inputs:

```text
python3 scripts/macos_lifecycle_recovery_results.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --results /absolute/private-review/lifecycle-recovery-results.json \
  --review-output /absolute/private-review/accepted-lifecycle-recovery.json
```

Unknown, missing, duplicated, reordered, non-attempted, writable, linked,
path-bearing, credential-shaped, stale, partial, residue-bearing, state-losing,
or release-mismatched input fails closed. The ingestor validates existing
records only and has no bookmark, process, installer, filesystem, workspace,
credential, signing, package-copy, fault-injection, or support-promotion
authority.

No native recovery campaign, signed installed component, protected raw log,
production release bundle, independent review, or physical Apple Silicon
execution exists here. Task 8.1.3.3 remains `BLOCKED-MACOS` until those
external artifacts are produced and reconciled.
