# Sprint 37 Local Verification Results

| Field | Result |
|---|---|
| Gate | Sprint 37 |
| Authority-free plans and exact previews | Pass locally |
| Write versus high-risk delete grant separation | Pass locally |
| Grant-consuming transaction and receipt matrix | Pass locally |
| Fedora native create, patch, copy, move, and trash operations | Pass locally |
| Native collision, symlink, hard-link, limits, and restoration fixtures | Pass locally |
| Complete `S-030-UT01` operation and boundary matrix | Pass locally |
| Native socket/FIFO and parent-rename matrix | Pass locally for exercised Fedora fixtures |
| Thirty-case native process-stop matrix | Pass locally for exercised Fedora boundaries |
| Full disk/fault/recovery/race matrix | Incomplete |
| Ubuntu, macOS, and Windows native evidence | Absent |
| Isolated native write-worker proof | Absent |
| Upstream Sprint 36 gate | Blocked |
| Gate-owned Sprint 37 review | Pass |
| Sprint result | Blocked |

## Verified Locally

- Closed plans validate exact file preimages, direct-child destination parents, source ownership,
  protected paths, structured patch hunks, sibling collisions, modes, and resource bounds before
  authority exists.
- Exact previews and five-minute decisions derive only one short-lived single-use
  `WorkspaceWrite` grant or, for an isolated trash plan with separate confirmation, one
  `WorkspaceDelete` grant.
- The in-memory coordinator covers all five operation classes, cancellation, stale source,
  destination collision, complete success, no-change failure, known partial restoration,
  post-state mismatch, malformed driver reports, restoration failure, receipt tampering, terminal
  uncertainty, and no replay.
- The native Fedora fixture performs create, exact patch, copy, move, and trash-first delete against
  real files and directories; verifies bytes and modes; preserves a neighboring file; refuses
  case-folded collision, symlink, hard-link, and tight-limit cases; and restores an earlier create
  after a later staging collision.
- Empty-file creation and Unicode case-collision behavior are deterministic.
- All five primitives cover nominal, existing, missing, and wrong-object-type states. Exact
  operation-count, file-byte, aggregate-byte, path-depth, and sibling-count ceilings pass while
  one-over inputs fail before authority; zero through `0777` permission variants remain exact and
  undeclared mode bits are refused.
- Unix sockets and FIFOs substituted for sources, destination parents, or destinations are refused
  without changing the special object. Descriptor-held parent-rename schedules cover all declared
  create/copy, move/trash, and create/copy-restoration lifecycle boundaries while preserving both
  the authorized directory object and a competing replacement at the canonical path.
- Thirty real subprocess stops cover eight create/copy boundaries, six move/trash boundaries,
  eight create/copy-restoration boundaries, and before/after verification for commit and
  restoration. Reopened canonical paths contain only exact reviewed prestate or poststate bytes.
- No filesystem transaction executes a command, accesses a network, changes Git, creates parent
  directories, overwrites a destination, expands a wildcard, recursively deletes, or permanently
  deletes as its requested effect.

## Requirement Mapping

| Requirement | Local contribution | Remaining product evidence |
|---|---|---|
| `SR-PLT-004` | Linux adapter uses strict held paths and no-replace native primitives | Native Ubuntu, macOS, and Windows parity |
| `SR-ACC-002` | Exact plan and preview are bound before grant issuance | Complete product authority integration |
| `SR-ACC-003` | Explicit write/delete decisions and separate delete confirmation | Independent end-to-end review |
| `SR-ACC-004` | Single-use grants bind targets, arguments, policy, preview, and side effects | Complete cross-feature grant audit |
| `SR-ACC-005` | Fresh pre-state plus descriptor-held parent and target checks precede effects | Complete alias, mount, target-writer, and cancellation campaign |
| `SR-ACC-006` | Known changes restore; parent races reconcile; process stops retain reviewed canonical states | Startup artifact reconciliation plus disk-full, permission, and durability faults |
| `SR-OPS-001` | Operation receipts are hash chained and content minimized | Durable product audit integration |
| `SR-TST-004` | Fixed protected-path, collision, and mutation corpus is retained | Manual fuzzing remains deferred |
| `SR-TST-005` | Recovery and no-replay paths have deterministic tests | Full fault injection on every native operation |

Every mapping is a Sprint 37 contribution, not a product-completion claim.

## Open Evidence

Sprint 36 remains blocked, so the declared upstream dependency is not satisfied. The current native
tests do not exhaustively inject permission loss, disk full, mount replacement, device nodes,
target-writer schedules, move-restoration process death, and durability failure at every syscall
boundary for every operation. Abrupt exits can leave exact named staging or tombstone artifacts;
startup discovery and reconciliation of those artifacts is not yet implemented. The write driver
is not yet proven inside the final operating-system-isolated worker.
The gate-owned automated review is complete without a human-review claim. Native Ubuntu, macOS,
and Windows execution remain absent. Manual fuzzing remains deferred by the recorded project
decision.

Sprint 37 therefore remains blocked even though every locally implemented check in the retained
report passes with zero focused skips.
