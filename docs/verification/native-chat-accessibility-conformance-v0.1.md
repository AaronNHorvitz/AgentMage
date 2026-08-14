# Native Chat Accessibility Conformance Report v0.1

## Report Identity

| Field | Value |
| --- | --- |
| Surface | AgentMage provider in native Visual Studio Code Chat |
| Contract | `docs/architecture/native-chat-accessibility.md` |
| Evidence protocol | `RV-20` |
| Current disposition | **BLOCKED** |
| Blocking rule | Any failed, not-tested, stale, unavailable, or unreviewed required row blocks conformance |

## Current Results

| Requirement | Method | Linux | macOS | Evidence | Disposition | Remediation |
| --- | --- | --- | --- | --- | --- | --- |
| Exact model identity and textual availability state | Automated unit and authenticated-bridge tests | PASS (non-native) | PASS (non-native) | `shells/vscode/test/model_discovery.test.ts`, `shells/vscode/test/host_bridge.test.ts` | Automated pass only | Run native picker and accessibility-tree checks |
| Structured headings, lists, limitations, diagnostics, receipts, errors, and cancellation | Automated unit tests | PASS (non-native) | PASS (non-native) | `shells/vscode/test/provider.test.ts` | Automated pass only | Retain native announcement order |
| Valid named citation and unsafe-link refusal | Automated positive and seeded-failure tests | PASS (non-native) | PASS (non-native) | `shells/vscode/test/provider.test.ts` | Automated pass only | Exercise native link navigation and activation |
| Keyboard-only core workflows and visible logical focus | Manual native test | NOT TESTED | NOT TESTED | None | Blocking | Run every protocol workflow and retain transcript/video or equivalent review notes |
| Focus restoration after picker, modal, save dialog, cancellation, and error | Manual native test | NOT TESTED | NOT TESTED | None | Blocking | Record focus before and after every transition |
| Screen-reader names, roles, states, descriptions, errors, and ordered live updates | Manual assistive-technology test | NOT TESTED | NOT TESTED | None | Blocking | Select and record Linux screen reader; run VoiceOver on supported macOS hardware |
| 200 percent zoom and declared narrow reflow with no clipping or overlap | Manual native test | NOT TESTED | NOT TESTED | None | Blocking | Capture every core workflow at each declared setting |
| No color-only, icon-only, hover-only, pointer-only, or timed meaning | Automated source review plus manual native test | PASS (AgentMage output) | PASS (AgentMage output) | `shells/vscode/src/provider.ts`, `shells/vscode/src/model_discovery.ts` | Manual native portion not tested | Inspect native workflow and retain reviewer disposition |
| Handoff preview accessibility | Sprint 24 native test | NOT TESTED | NOT TESTED | None | Blocking | Implement and test the Sprint 24 handoff surface |

## Seeded Failures

The current automated suite rejects malformed model identities, stale entry
digests, self-consistent blocked profiles falsely labeled selectable, hidden
fallback fields, invalid response identities, malformed previews, unsafe
display links, cancellation races, and missing host state. These tests cover
AgentMage-owned data and output boundaries.

Missing native names, incorrect native focus order, inaccessible Visual Studio
Code live regions, native color-only meaning, timeout pressure, and native
zoom/reflow failures cannot be injected credibly in the headless TypeScript
suite. They remain explicit blocking manual cases rather than being labeled as
automated passes.

## Platform Differences

- Linux requires a declared desktop session, Visual Studio Code build, screen
  reader, speech stack, and display server in the evidence manifest.
- macOS requires supported physical Apple hardware and VoiceOver. No current
  macOS evidence exists.
- Native Visual Studio Code behavior may differ from the pure TypeScript shell.
  A TypeScript pass never substitutes for native evidence.

## Gate Decision

`RV-20` and Sprint 23 remain **BLOCKED**. Automated AgentMage-owned output and
failure checks pass, but keyboard, focus, screen-reader, live-update,
zoom/reflow, native-link, handoff, and macOS observations are not tested.
