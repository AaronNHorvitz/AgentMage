# Story 1.3 AC1 Rust-owned boundary meaning

Story acceptance criterion `1.3.AC1` passes for the current canonical Engineering Runtime record
scope. All nine record families have one closed Rust-owned type and one generated versioned JSON
schema. The contract and generated schema expose the same complete top-level field sets, including
required nullable members, and preserve the canonical schema version across Rust and JavaScript
callers.

Missing, extra, malformed, oversized, and unsupported-version records fail before canonical
publication. The semantic Rust boundary additionally rejects cyclic workflow graphs and stale
cross-record request identity. No caller can admit an alternate record meaning or silently ignore
unknown data.

This closes the current Rust/JSON contract-boundary criterion using public synthetic fixtures. It
does not claim installed-product, native-platform, real-model, independent-review, Story, Sprint,
packaging, or release completion.
