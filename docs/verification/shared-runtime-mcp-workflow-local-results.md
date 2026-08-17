# Shared Runtime, MCP, and Workflow Local Results

## Scope

This local source campaign covers Story 50.2 caller authority and lifecycle,
the dependency-safe Sprint 95 shared-runtime attachment, and the implemented
Sprint 80-81 MCP contract, manifest, identity, registry, and gateway slices.

## Commands

```text
cargo test -p agentmage-kernel-contracts --lib
cargo test -p agentmage-kernel-engine mcp_ --lib
cargo test -p agentmage-kernel-engine workflow_authority --lib
cargo test -p agentmage-host --lib
cargo clippy -p agentmage-kernel-engine --lib -- -D warnings
cargo clippy -p agentmage-host --lib -- -D warnings
cargo test --workspace --lib
cargo clippy --workspace --lib -- -D warnings
npm run docs:check
```

## Implemented Evidence

| Area | Source | Local evidence |
|---|---|---|
| Workflow authority | `kernel/engine/src/workflow_authority.rs` | Seven required layers, narrowing-only intersection, malformed-layer and aggregation rejection |
| Caller lifecycle | `shells/host/src/workflow_caller.rs` | Sealed submission, ordered events, explicit approval resume, terminal evidence and artifacts |
| Interactive CLI client | `shells/host/src/cli_runtime.rs` | Same host-framed runtime port as native Chat; independent request, stream, artifact, approval, cancellation, outcome, and release verification |
| Three-client parity | `shells/host/src/runtime_parity_tests.rs` | Identical read-only and controlled-write requests across native Chat, interactive CLI, and workflow caller; exact event, artifact, receipt, checkpoint, evidence, and outcome equality |
| Authority attack matrix | `shells/host/src/runtime_parity_tests.rs` | Eight narrower-node broadening attempts rejected before one instrumented coordinator advance |
| Real runtime attachment | `shells/host/src/linux_coding_runtime.rs` | Deterministic Linux fake-model no-op and denial paths through the production coordinator composition |
| Child attachment | `shells/host/src/workflow_assignment.rs` | Bound ancestry/dependencies/depth and authority-free pending-review result |
| MCP contracts | `kernel/contracts/src/mcp.rs` | Closed transport, manifest, request, response, cancellation, disconnect, error, and receipt schemas |
| MCP identity | `kernel/engine/src/mcp_registry.rs` | Manifest sealing, exact identity revalidation, expiry, non-shadowing optional common-registry adaptation |
| MCP mediation | `kernel/engine/src/mcp_gateway.rs` | One-use requests, bounded untrusted responses, lifecycle receipts, verified cancellation and disconnect cleanup |

## Truthful Disposition

Focused local tests, the workspace library suite, and strict workspace Clippy
pass. The complete host library reports 120 passing tests and three ignored
tests that require the Fedora or Ubuntu systemd user session, Bubblewrap, or
built worker manifests. Those ignores are not converted into passes.

Authenticated CLI stdin/history and installed transport, a production runtime
factory, live MCP transport, installed real-runtime client parity,
reference-hardware campaigns, Sprints 89-94, complete Sprint 95
coordination, supported-platform evidence, independent review, and deferred
manual fuzzing remain open. No sprint or release gate is closed by this report.
