# Release Xtask

This Rust module provides the bounded maintainer entry point for candidate and
release-bundle packaging without becoming an end-user runtime dependency:

```bash
cargo run -p agentmage-xtask -- package-candidate --version 0.0.0 --output release-output
```

To construct a signable bundle and sign its exact manifest with an already
approved external key:

```bash
cargo run -p agentmage-xtask -- package-release-bundle \
  --version 0.1.0 --release-sequence 1 --output release-output

secret-provider-command | cargo run -p agentmage-xtask -- \
  sign-package-manifest \
  --manifest release-output/agentmage-package-manifest-0.1.0.json \
  --public-key /independently-managed/agentmage-release-ed25519.pub \
  --signature release-output/agentmage-package-manifest-0.1.0.sig
```

The private seed must be exactly 32 raw bytes and is never a command-line or
repository value. These commands implement signing mechanics only. The public
key must be distributed independently, and trusted bootstrap, platform package
signing, protected production credentials, and release approval remain blocked.
