# Linux Platform Adapter

This Rust module implements the Phase 8 Fedora/Ubuntu aggregate, descriptor-safe
workspace and strict-local roots, exact-object paths, sandbox, native
configuration store, authenticated local IPC, stable process inventory, Secret
Service, explicit first-install operational-key lifecycle, and descriptor-held
SQLCipher state composition.

Every authority-bearing constructor requires an independently activated
`VerifiedPlatformAdapter<LinuxPlatformAdapter>`. The repository does not contain
a production signed release manifest or supported package, so these mechanisms
remain pre-alpha building blocks rather than an integrated product claim. See
[`linux-platform-lifecycle.md`](../../docs/architecture/linux-platform-lifecycle.md).
