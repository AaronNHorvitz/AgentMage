# Native model socket deadlines — verification

Date: 2026-09-28. Source: `5afe3a2ba716c74cf1d5894eb71da1f92da8c0ad`,
tree `dacb3309ae4a6aba0b6400cb1dfc2ff117c201e5`.
[Decision 0100](../decisions/0100-native-model-socket-exchange-deadlines.md)
governs this transport prerequisite. The [retained record](model-socket-deadlines-2026-09-28.json)
binds exact executed source, failures, checks and CLI observations.

## Behavior and limits

Native health, properties, token-count and completion exchanges now carry one
monotonic deadline across nonblocking Unix connection, upload and response.
Small response fragments cannot renew it. Full connection queues fail promptly
without reconnecting or resending. Completion observes its existing exact
cancellation probe during upload as well as reading. Upload cancellation or
expiry returns the existing interrupted result with one terminal fragment and
no fabricated output. Byte ceilings and HTTP/stream validation remain enforced.

The helper and its fixtures remain inside the complete, already hash-bound driver
file. The source audit admits only the exact pinned private Unix helper and its
sole private import. Other uses of its dependency or namespace are refused;
the general socket allowlist is unchanged. Other network and URI rules still
scan all production source. The lexical audit supplements compilation and native
confinement; it is not a parser or runtime authority boundary.

These cooperative bounds do not preempt serialization, parsing or stalled kernel
calls. Token-request encoding, endpoint JSON parsing, manifest hashing, startup
readiness, exact token binding and aggregate preflight still need shared run
cancellation and remaining-time control. Factory model loading remains synchronous.
Socket closure does not prove model-process or namespace cleanup. The existing
lease and uncertain-cleanup refusal remain unchanged.

## Executed checks and retained failures

Seven original socket regressions executed against the prior production source:
**one passed and six failed**. Slow responses exceeded the deadline, silent reads
and blocked uploads returned generic errors, completion upload missed cancellation,
and a full connection queue waited about 250 ms. The first fixed attempt failed
compilation with `E0432`; no tests ran in that attempt. Its source and log remain.

After correction, **all ten socket regressions passed**. The blocked-upload peer
receives 64 KiB, including payload beyond the complete HTTP header, before
requesting cancellation or stopping reads. The client must return before the
fixture lifeline; the still-open peer observes EOF and only a partial request.
Other cases exercise cumulative upload/read time, three closed endpoints, a silent
peer, byte limits, cancellation identity and probe failure before connection.
Existing cancellation-after-output coverage now uses an observed fragment rather
than an internal probe-call count. These are actual local socket component
fixtures with no inference or model process.

The full inference library returned **143 passed, zero failed and four ignored**.
Strict inference/host Clippy and formatting passed. The full host suite returned
**292 passed, 50 failed and eight ignored**, with no filtered tests. All 50 names
and recorded causes match the preceding host result; the suite remains failed.
The unchanged Linux suite was not rerun; its prior 19 failures remain retained.

The initial source audit refused the new socket import. Its narrow admission and
adversarial mutations then passed with **31 Python tests**, including the shared
lexical tests. Source, dependency, effect, module and status checks passed. Full
Markdown lint reported **five formatting issues in one ignored generated
font-license file under `target/doc`**. That failure is retained; the license and
configured gate were unchanged. A separate check excluding generated `target`
output passed all **577 maintained Markdown files**. This scoped result does not
replace the failed full invocation.

## Actual CLI and evidence

At the clean source revision, the CLI, host and read worker rebuilt successfully.
Help, setup, diagnosis and status succeeded in fresh disposable repositories.
The explicit scripted fail/repair launch returned **exit 5** before IPC, tools or
inference: `linux.repository.git_artifact.invalid`, followed by
`linux.development.launch-envelope.failed`. Idle stop returned **exit 1**; active
cancellation was not reached. A separate repository with staged, unstaged and
untracked work returned **exit 1** at preflight, before its launch log directory
was created. File bytes, modes, HEAD, raw index, index entries and both Git diffs
were unchanged in both cases. All three binary identities were stable within
the observations. Preservation during authorized effects remains unverified.

The batch regenerated the SBOM once; only the inference component content hash
changed. Six affected foundation builders and the current canary campaign passed,
with **61 Python tests**. The canary campaign retains eight commands, seven Rust
fixtures and eight coverage rows. All report input sets remain intact. The
coordinator runtime, automated boundary, security and evidence-index inputs stayed
unchanged; their four read-only checks passed without renewing their source pins.
The index still has 16 complete and one partial mapping, with story, sprint and
release completion false. Generated evidence is committed at
`025909e8cbb45e2751cc1d886186271b1db127a7`.

The new record preserves the previous record's complete public input set. Six
unchanged historical verification records retain their original source pins;
no global, transitive or line-span freshness is claimed. Source automation is
separate from independent review, which remains pending. Real-model, manual-user
and release runs are absent. No task or acceptance gate closes. The positive Linux
edit/validate/fail/repair workflow, protected denial and active cancellation remain
blocked by native trust and user-systemd prerequisites in this lane.

Use the [local launch instructions](../LOCAL-TESTING.md). The focused component check is:

```sh
bash /tools/build-slot env CARGO_BUILD_JOBS=1 cargo test --locked --offline \
  -p agentmage-platform-linux-inference --lib transport_budget_ -- --test-threads=1
```
