# macOS S-008-ST01 Sandbox Attack Matrix

Task 8.1.3.2 (`S-008-ST01`) executes one closed hostile-access campaign
against the installed, signed kernel host, Visual Studio Code bridge, XPC tool
helper, and Metal inference service. Each component is attacked for ambient
home, device, process, environment, credential, network, workspace-write,
grant, and cross-user access. The Cartesian product is exactly 36 cases.

The Swift acceptance test defines the canonical component and attack-class
order, invokes a probe for every pair, and rejects an incomplete or substituted
identity. Every case must have been attempted and must report zero unauthorized
accesses, bytes, network connections, network bytes, descendants, residue,
canary observation, workspace modification, and authority broadening. The
test-local probe interface grants no production authority and the checked-in
probe is synthetic; the signed native acceptance harness supplies the external
probe on the pinned Apple Silicon runner.

The raw native output is retained with the existing task 8.1.2.4 procedure in
[`SANDBOX-ATTACK-RESULTS.md`](SANDBOX-ATTACK-RESULTS.md). That assembler requires
the exact release policy, manifest, signed package, successful terminal record,
four profile logs, 36 attack logs, and the ordered closed results. The independent
verifier emits only identities, counts, hashes, and explicit non-claims.

After the native Swift test and installed-package probes finish, use:

```text
python3 scripts/macos_sandbox_attack_results.py --assemble \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/evidence/AgentMage-1.2.3.pkg \
  --terminal /absolute/evidence/terminal.json \
  --evidence-root /absolute/native-sandbox-evidence \
  --results-output /absolute/private-review/sandbox-attack-results.json
```

The repository source contract cross-checks the Swift 4-by-9 campaign against
the task 8.1.2.4 evidence assembler and its retained source report. It has no
native process, filesystem, device, credential, network, workspace, grant,
cross-user, attack, or support-promotion authority.

No native hostile-access campaign, signed installed component, active sandbox
profile, protected raw log, production release bundle, independent review, or
physical Apple Silicon execution exists here. Task 8.1.3.2 remains
`BLOCKED-MACOS` until those external artifacts are produced and reconciled.
