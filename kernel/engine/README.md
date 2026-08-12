# Kernel Engine

This Rust module implements closed policy and grant state machines, exact
authority transactions, SQLCipher-backed canonical grant/receipt state, and
fail-closed restart recovery against kernel contracts. Configuration parsing,
migration calculation, canonicalization, comparison, and receipt binding remain
pure kernel logic; native loading, replacement, backup, and synchronization are
owned by platform adapters. Product orchestration, model mediation, and complete
data-lifecycle scope remain future work.
