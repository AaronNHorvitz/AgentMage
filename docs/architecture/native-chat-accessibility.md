# Native Chat Accessibility Contract

## Status

This document defines the Sprint 23 accessibility contract for the AgentMage
Visual Studio Code language-model provider. It is normative for AgentMage-owned
labels and generated output. Visual Studio Code owns the native Chat controls,
model picker, modal focus management, live regions, zoom, and reflow behavior;
those behaviors require native manual evidence and cannot be inferred from unit
tests.

## Interaction Inventory

| Workflow surface | Native role or structure | Accessible name or label | Keyboard operation | State and error communication | Required focus behavior |
| --- | --- | --- | --- | --- | --- |
| Model selection | Visual Studio Code model picker | Exact signed profile display name | Native picker keyboard commands | Tooltip and detail include exact profile, runtime, limits, support, and no-substitution notice | Opening, selection, dismissal, and unavailable refresh return focus through native Chat |
| Workspace selection | Current native workspace plus modal warning | Workspace name and exact relative components in the warning | `Review Read Preview` or native cancel | Missing, remote, non-file, or multiple workspaces produce a headed denial with status and code | Modal dismissal returns focus to Chat input |
| Chat request | Native Chat input | Visual Studio Code-owned Chat input name | Native submit and edit commands | Ordered `LanguageModelTextPart` updates use headings, prose, and labeled status fields | Streaming does not move focus from input or response navigation |
| Read approval | Modal warning | Exact path, byte count, content digest, and `Approve Read` action | Approve or native cancel | Cancellation and invalidation state that no read started | Completion or cancellation returns focus to Chat |
| Citation | Markdown link | `Open validated local file` | Native link navigation and activation | Invalid links produce a headed denial and are never rendered as links | Link activation follows Visual Studio Code behavior |
| Diagnostics | Structured Markdown | `AgentMage Local Status` and `Components` headings | Native Markdown navigation | Every component has a text state, reason code, and remediation code | No forced focus change during output |
| Diagnostic export | Native save dialog plus modal warning | `Select Diagnostic Export Destination` and `Write Diagnostic Export` | Native save-dialog and modal commands | Preview states bytes, fields, redactions, sensitivity, retention, and payload digest | Cancel and completion return focus through native dialogs to Chat |
| Cancellation | Visual Studio Code Chat cancellation control | Visual Studio Code-owned cancellation label | Native Chat cancellation command | `Request Cancelled`, `Status: cancelled`, and a stable code are emitted in order | Cancellation must not trap or discard Chat navigation |
| Handoff preview | Not implemented in Sprint 23 | Not tested | Not tested | Not tested; Sprint 24 owns this surface | Not tested |

## Output Rules

1. Meaning never depends on color, icon, animation, position, hover, pointer
   precision, or timing. Every state is written in text.
2. Generated status, model-management, read, diagnostic, denial, cancellation,
   limitation, citation, and receipt output begins with a descriptive heading.
3. Sections use headings and flat lists. Links have purpose-specific names.
   Content excerpts use indented code blocks so repository Markdown cannot
   become active response structure.
4. Ordered response parts remain complete when concatenated. Cancellation and
   failure produce terminal text instead of relying on a transient notification.
5. No AgentMage-owned custom webview, toolbar, focus manager, hover-only
   control, timed interaction, or color-only legend exists in this workflow.

Every runtime response begins with a structural `Session Boundary` heading and
textual session, workspace snapshot, exact model/runtime, tool, permission-policy,
offline, vision, and resource indicators. Ordered progress remains a Markdown
list, and the canonical outcome uses a separate `Result` heading with textual
state, evidence, receipts, limitations, and output disposition. These structures
preserve AgentMage-owned meaning independently of presentation styling.

The provider emits the session boundary and every content-free runtime event as
separate ordered updates. Once the kernel supplies a terminal outcome, the
renderer first validates its evidence bindings, payload digest, media type, and
safe Markdown subset, then emits the verified output and its result facts as
separate ordered updates. Raw model tokens never cross this boundary and cannot
be mistaken for verified user-visible output.

An explicit profile change uses a separate exact revalidation operation. Its
closed preserved-state input carries the task identity, plan digest, ordered
evidence digests, and an optional paired checkpoint identity/digest. Admission
returns the requested exact profile with the identical state object; refusal
returns no selected profile, the identical state, and a visible no-substitution
stop. Neither branch starts a runtime operation.

## Native Evidence Protocol

For every supported operating system, retain the exact OS, Visual Studio Code,
extension package, display scale, zoom, and assistive-technology versions. Run
model discovery, zero-profile state, selection, workspace refusal, approval,
read success, invalid citation, diagnostics, export cancellation, request
cancellation, profile invalidation, and no-fallback refusal.

Each workflow must be completed by keyboard alone at default zoom, 200 percent
zoom, and the declared narrow reflow width. Record visible focus, focus order,
focus restoration, clipping, overlap, pointer-only actions, traps, announcement
order, missing announcements, timeout pressure, and recovery. A missing,
failed, stale, or unreviewed observation blocks `RV-20` and Sprint 23.

VoiceOver evidence is required on supported macOS hardware. The declared Linux
screen reader and desktop session must be selected and recorded before Linux
manual evidence can pass. Automated TypeScript tests cover AgentMage-owned
structure and denial behavior only.
