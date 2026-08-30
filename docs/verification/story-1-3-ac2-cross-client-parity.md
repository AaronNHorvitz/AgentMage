# Story 1.3 AC2 cross-client parity

Story acceptance criterion `1.3.AC2` passes for the current canonical record corpus. JavaScript
schema admission and the Rust public decoder consume the same 56-case synthetic corpus. Every one
of the nine admitted record families round-trips through Rust with the exact canonical bytes and
SHA-256 identity recorded by the JavaScript generator.

The borrowed publication boundary produces the same bytes as the authoritative Rust encoder.
Request, session, task, source, policy, content, graph, attempt, receipt, verifier, and terminal
bindings remain unchanged across callers. The callers validate or borrow records; neither owns
runtime state, persistence, lifecycle transition, or execution authority.

This closes the current deterministic Rust/JavaScript caller-parity criterion. It does not claim
installed-client workflow parity, native-platform, real-model, independent-review, Story, Sprint,
packaging, or release completion.
