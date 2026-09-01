# Durable Verified Chat Surface

## Scope and ownership

Story 23.7 completes the repository-controlled portion of AgentMage Verified Chat. The VS Code
extension owns a disposable editor view and transport adapter. The authenticated Rust Engineering
Runtime remains the only owner of sessions, artifacts, modes, approvals, policy, grants, tools,
recovery, verification, lifecycle, and terminal truth.

The webview cannot access the filesystem, network, credentials, model gateway, tool dispatcher, or
host bridge. It sends only closed user decisions to the extension. Ask, Plan, Agent, and the visible
future Team option are policy inputs; choosing a mode does not mint authority. Agent execution
starts only after the Rust runtime records an exact Plan approval and creates a hash-bound handoff
session. Team remains visibly unavailable until its later runtime story is installed.

## Disposable authenticated transport

Every editor instance receives a random, bounded channel identity. Each outbound command contains
that identity and one exact monotonically increasing sequence. The extension checks exact keys,
closed command and mode values, payload limits, channel binding, and sequence before doing any
work. Replayed, skipped, duplicated, spoofed, widened, or unknown messages fail closed. Closing the
view destroys the channel.

The editor uses a restrictive Content Security Policy: no default origin, no inline script or
style without the per-document nonce, and no network or frame permission. Rendering uses
`textContent` and DOM creation rather than HTML insertion. The Activity Bar launcher has no
runtime authority and only opens the canonical editor.

Host-to-view messages pass through one ordered queue bounded by both message count and serialized
bytes. The active send counts against both ceilings. A slow or closed view produces an explicit
backpressure or delivery failure instead of an unbounded queue or silently dropped state.

## Reconstruction and integrity

The extension persists only the last session identifier in VS Code workspace state. On view or
extension-host restart it opens that session through authenticated Engineering IPC. Rust verifies
the durable snapshot and event chain before responding; the extension then independently:

1. checks the exact closed snapshot and artifact-receipt shapes;
2. recomputes the Rust-compatible snapshot digest with its digest field zeroed;
3. replays events from sequence zero and checks session binding, exact sequence, previous digest,
   unique event identity, closed event shape, and every recomputed event digest;
4. projects content-free artifact, context, route, tool, verification, approval, lifecycle, Team,
   and terminal cards; and
5. restores a draft or approved Plan by reading its immutable artifact in at-most-1 MiB pages,
   validating every range receipt and page digest, validating the complete source digest, and
   requiring strict UTF-8.

The projection applies each replay page atomically. An invalid later event therefore cannot leave
an accepted prefix in view memory. The view never persists model reasoning, raw endpoint values,
credentials, grants, executable calls, or a competing terminal state. Captured artifacts that are
not reselected for a future turn are shown as durable history, not silently restored as model
context.

## Streaming, lifecycle, and accessibility

Runtime events and Agent output parts are delivered in order. Ask and Plan output is emitted in
bounded presentation chunks only after the host returns its verified turn; this is not represented
as live model-token evidence. Agent output uses the existing controlled runtime callback and then
an explicit final marker. Runtime cards and terminal diagnoses come only from verified durable
events.

Starting an Agent run returns control to the command channel so cancellation can race the active
runtime instead of waiting behind it. While a run is active, the presentation gate admits only
cancel; it refuses another start, send, mode change, pause, or resume. Cancel first signals the
existing controlled runtime token and then requests the durable Rust session cancellation. Closing
the view also signals cancellation. The gate cannot launch work, grant authority, or declare
completion.

The transcript is an ARIA live log, runtime cards are labeled status entries, controls have
accessible names, keyboard-native buttons and selects are used, focus remains under VS Code, text
wraps without horizontal dependency, and the layout collapses for narrow editors. These are
source-level accessibility controls, not a claim that packaged keyboard or assistive-technology
campaigns have run.

## Retained evidence and limitations

Source-bound local evidence is retained in
`artifacts/sprints/sprint-23/story-23.7/verified-chat-report.json`, with raw command output beside
it. The local campaign covers TypeScript build/tests/lint/format, hostile IPC and replay fixtures,
bounded delivery, cancellation arbitration, snapshot/Plan reconstruction, Rust Engineering
Runtime persistence and lifecycle tests, stable VS Code API constraints, and documentation.

It does not claim an installed VSIX campaign, native accessibility audit, qualified production
model, independent review, Windows physical execution, or macOS execution. Windows stays open
until the final local Windows campaign is run against the release-candidate commit. macOS and
GitHub-hosted validation are intentionally deferred by the current run instructions.
