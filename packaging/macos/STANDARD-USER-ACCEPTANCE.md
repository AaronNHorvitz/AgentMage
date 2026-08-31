# macOS Standard-User Acceptance

Task 8.1.3.4 (`S-008-AT01`) consumes the content-free
`standard-user-acceptance.json` written atomically by the isolated release
runner only after the exact package completes nine ordered phases: build, sign,
notarize, staple, Gatekeeper check, install, launch, use, and remove.

The ceremony rejects root, unequal real/effective identity, macOS `admin` group
membership, and a home directory not owned by the invoking identity. Acceptance
also requires zero removal residue and closure of the bounded notarization
network phase before installed-product execution.

An independent reviewer verifies the record against the exact release policy,
manifest, package, and terminal record:

```text
python3 scripts/macos_standard_user_acceptance.py --verify \
  --policy /absolute/release-policy.json \
  --manifest /absolute/release-manifest.json \
  --package /absolute/AgentMage-1.2.3.pkg \
  --terminal /absolute/terminal.json \
  --acceptance /absolute/standard-user-acceptance.json \
  --output /absolute/accepted-standard-user-evidence.json
```

The verifier performs no build, signing, network, installation, launch, use, or
removal operation and grants no release or macOS support authority. No native
standard-user acceptance has run in this checkout; production identities,
package, pinned Apple Silicon image, protected outputs, and independent review
remain external, so Task 8.1.3.4 remains `BLOCKED-MACOS`.
