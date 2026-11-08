# T-303/T-306: independent endpoint context admission

Non-closing core implementation under Decisions 0153 and 0154. Both parents
remain open and require independent exact-commit review. No new sub-row.

The kernel now admits the original ordered endpoint messages before transport
preparation in the existing snapshot, workspace and repair proposal routes.
`RuntimeEndpointContextPacket` accepts only the closed text roles and 1–256
nonempty messages ending in user input. Original UTF-8 content and serialized
JSON are bounded to 256 KiB; escaping and role syntax count. Its writer checks
the next byte count before hashing and retains no second plaintext buffer.

The model phase constructor requires this immutable packet, compares its entire
message descriptor against the frozen transport request, and retains only
content-free observations. There is no descriptor-only constructor. This refuses
role, order, content, count, length and digest drift before disclosure dispatch.
Repair preparation and context checks still precede spending the existing
single-use receipt. The transport remains responsible for the actual encoded
body and destination; this is not independent decoding of its private body.

The API publishes `runtime_endpoint_context` on the initial context event:
version, exact message-array count/bytes/hash and original content bytes. No
profile, tokenizer fact, new prompt, source selection, permission, grant, tool,
retry or write is introduced. Native coordinator contracts are unchanged.

Four new kernel regression methods exercise original roles, ordering, escaped
Unicode, immutable metadata without retained plaintext; empty/invalid-ending
and exact/excess message counts; exact/excess encoded-byte boundaries including
escaping and UTF-8; and well-formed but mismatched transport context before
phase/resource admission. All twelve previous phase/stream methods retain their
negative cases, now with independently admitted original messages. Existing
snapshot/workspace API checks assert the context record, and the synthetic
repair transport compares it with the actual received JSON array and content
size for valid/invalid strict edits with/without provider usage. These are
synthetic contracts, not real-model or native-workflow measurements.

The corrected complete scoped fast gate selected `agentmage-host`,
`agentmage-kernel-engine`, `agentmage-local-api` and `agentmage-platform-linux`.
All six unmodified commands ran under one shared reservation in **350.77 seconds**,
excluding the reservation wait. Formatting passed in 2.00 seconds and Clippy
with warnings denied in 29.14 seconds. Tests took 318.28 seconds: **43 targets,
2,528 passed, 72 failed, 71 ignored, zero filtered**. Cargo returned 101 and the
source gate remains **FAIL**.

The audit requires the same 43 complete targets as the preceding port gate,
which had 2,524 passes. Every previous passing identity remains, plus exactly
four new methods. All failure identities, normalized diagnostics and ignores
match. The failures remain 53 host-library and 19 Linux-library cases under the
unchanged deferred executable-ownership blocker. All 24 required context,
stream, phase, proposal and repair methods pass unfiltered. A matching failure
baseline does not make the source gate green.

The first attempt failed Clippy and test compilation on the new boundary-test
closure's borrow lifetime. It was replaced by a function with a fresh input
lifetime per call. Only that test source changed; every case and assertion is
preserved. The failed log, source freeze and original test file are retained.
All six corrected Rust hashes stayed fixed throughout the complete rerun. No
source changes or further Cargo runs followed. The predecessor review arrived
during tests; the final seven-command plan adds its light report lint only,
with identical Cargo commands. Final light checks verify the exact ten paths,
source hashes, byte-identical review, unchanged ledger rows, no private host
material and no added ignores. Raw logs and reproducible audits are retained
privately. No owned validation or model job remains.

No supply-chain regeneration, frozen task, hidden or held-out case, scorer,
threshold, ignore, GPU, model download, push or release changes.

Full configured-endpoint coordinator run/turn composition, native edit/test/
retry observations, durable ownership, approved Git publication and parent
scoreboard runs remain open.

The predecessor's [independent core review](../reviews/f97b5d3351aa-local-coding-endpoint-port.txt)
arrived during this source run with **Verdict: PASS** for exact commit
`f97b5d3351aabcba511f0accb6e9ccb2110f1d28` and no high/medium finding. It is
retained byte-identically. Low L1 (overflow-latch test discrimination) and L2
(pre-dispatch mismatch diagnostic wording) remain backlog with earlier lows.
The existing workspace and synthetic repair terminals now check phase usage
alongside their context assertions, partly addressing the older phase-coverage
low; API pre-dispatch zero-attempt coverage remains backlog. No low detour or
parent closure. This review accepts only the predecessor port, not this context
admission increment.
