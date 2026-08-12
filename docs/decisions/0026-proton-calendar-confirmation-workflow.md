# Decision 0026: Proton Calendar Confirmation Workflow

| Field | Value |
|---|---|
| Status | Accepted scope refinement |
| Date | 2026-08-12 |
| Scope | Proton Calendar confirmed-UI operations and direct-invitation or email-first confirmation workflows |
| Adds | `AM-PCAL-001`, `AT-PCAL-001`, Story 139.2, and explicit Sprint 141 workflow coverage |
| Preserves | All prior requirements, identifiers, gates, authority boundaries, current-state truth, and dependency order |
| Does not authorize | Immediate implementation, out-of-order development, credential extraction, generic browser authority, unattended ambiguous-response interpretation, or a current support claim |

## Context

AgentMage already plans structured Google Calendar and Microsoft calendar
operations, Proton Mail Bridge access, confirmed computer use, exact
communication-effect controls, personal-information adapters, and deterministic
productivity workflows. A concrete user need makes the missing provider boundary
clear: ask another person to confirm a proposed event, observe the response, and
create or update the event without losing exact recipient, time, account, or
consent state.

Google Calendar and Microsoft Graph expose structured calendar operations.
Proton Calendar currently does not expose CalDAV for direct two-way external
synchronization. Proton does support event participants and invitations through
its own interfaces. The safe first-GA path is therefore a provider-specific,
confirmed user-interface adapter behind the same kernel transaction and
operation contracts, not direct mutation of a desktop calendar client's private
profile and not unrestricted model-driven browser use.

Current provider behavior is documented by Proton at:

- <https://proton.me/support/subscribe-to-external-calendar>
- <https://proton.me/support/send-invitations-to-existing-proton-calendar-events>

These links record planning context, not permanently assumed provider behavior.
The promoted support matrix and conformance evidence remain authoritative at
implementation and release time.

## Decision

1. Add `AM-PCAL-001` and `AT-PCAL-001` as first-GA requirements under the
   existing productivity and communications capability family.
2. Add Story 139.2 as the dedicated Proton Calendar adapter story. It depends on
   confirmed computer use, Proton Mail Bridge, common synchronization, exact
   communication writes, and the shared personal-information contract.
3. Prefer structured provider APIs and protocols. Use Proton Calendar UI
   interaction only while no admitted structured write path can satisfy the
   operation.
4. Bind the Proton adapter to an exact allowlisted Proton origin, visible
   foreground session, authenticated user profile, calendar, event, operation,
   event fields, attendee identities, expected notifications, provider-surface
   version, preconditions, and postconditions.
5. Keep passwords, second factors, recovery material, cookies, session tokens,
   and browser-profile state outside model context, logs, receipts, screenshots,
   exports, and operation payloads. AgentMage neither enters nor recovers Proton
   credentials.
6. Use structured document or accessibility information before bounded visual
   interpretation. Screenshot or OCR assistance is a declared fallback only and
   cannot establish identity, authorization, successful effect, or standing
   authority by itself.
7. Require a fresh exact preview under the effective Autonomy Center policy for
   every event or invitation effect. Generic confirmed-computer-use authority
   cannot substitute for a calendar operation grant.
8. Support two explicit workflow forms:
   - Direct invitation: create the approved event with the approved participant
     and let the calendar provider send its invitation.
   - Email first: send an approved question, correlate only the exact account,
     recipient, thread, and proposal, classify the reply, and prepare the event
     only after a clear affirmative response.
9. Treat ambiguous, conditional, conflicting, stale, superseded, multi-proposal,
   or identity-uncertain replies as requiring user review. A model conclusion
   alone cannot constitute attendee consent.
10. Persist operation intent before effect, prevent blind retries, and re-read
    the provider state after submission. Timeout, navigation drift, focus loss,
    changed controls, missing confirmation, or an unreadable result remains
    `unknown` until reconciliation proves effect or non-effect.
11. Detect provider-surface drift through a versioned capability manifest.
    Unsupported controls or semantics are absent from registration; the adapter
    fails closed instead of guessing coordinates or labels.
12. If Proton later provides an admissible structured calendar API or CalDAV
    write path, promotion requires an explicit reviewed adapter change and its
    own conformance evidence. It does not silently inherit the UI adapter's
    approval or support claim.

## Required Verification

- Mutate account, origin, profile, calendar, event, title, start, end, time zone,
  recurrence, location, attendees, notification behavior, thread, proposal,
  reply identity, UI version, focus, and postcondition around preview and effect.
- Exercise direct invitations, email-first confirmation, affirmative, negative,
  tentative, conditional, alternative-time, conflicting, stale, duplicate, and
  malicious replies.
- Exercise UI changes, inaccessible controls, overlays, redirects, popups,
  session expiry, reauthentication requests, second-factor prompts, focus loss,
  timeout, cancellation, crash, partial effect, duplicate submission, and
  provider outage.
- Prove that credential material, authenticated profile state, hidden fields,
  screenshots, and Proton content cannot enter model instructions or authorize
  another operation.
- Verify exact postconditions or an explicit unknown result, zero blind retry,
  zero duplicate event or invitation, and complete adapter removal with no
  surviving session authority owned by AgentMage.

## Consequences

- The accepted inventory grows from 227 to 229 stable requirements without
  changing the 17-epic, 169-sprint execution sequence.
- Sprint 139 gains one separately reviewable story; Sprint 141 gains the
  concrete confirmation-to-calendar workflow cases.
- Gmail and Outlook calendar integrations remain structured API paths. Proton
  Mail remains a loopback Bridge path. Proton Calendar becomes an explicit
  confirmed-UI exception with a narrower authority and a higher maintenance
  burden.
- The current product remains `scaffolded`. No Proton, Gmail, Outlook, calendar,
  mail, or workflow capability is implemented, enabled, supported, or released
  by this planning decision.

## Approval Record

On 2026-08-12, the user explicitly approved adding the recommended Proton
Calendar adapter and confirmation workflow to the roadmap while preserving
dependency order and deferring implementation until its owning gates are
reached.
