# Release Xtask

This Rust module provides the bounded maintainer entry point for candidate
packaging. It orchestrates the standard-library package builder without becoming
an end-user runtime dependency:

```bash
cargo run -p agentmage-xtask -- package-candidate --version 0.0.0 --output release-output
```

The command emits unsigned stabilization candidates only. Release signing,
trusted-key distribution, and activation remain separate fail-closed gates.
