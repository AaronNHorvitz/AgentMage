# Decision 0009: Productivity, Finance, and Cloud Observer Expansion

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-08-11 |
| Scope | First-GA productivity, communications, finance, and read-only cloud capabilities |
| Supersedes | The rule that final v1.0 closure occurs at Sprint 126 and the unnamed generic-only first-GA connector boundary |

## Context

Decision 0008 expanded AgentMage into a local-first software-delivery control plane. The existing
kernel, capability-grant, provider-adapter, connected-identity, synchronization, receipt, and
removal designs can also support the surrounding work that makes an agent materially productive:
communications, schedules, documents, commitments, budgeting, financial records, and cloud
observation.

The prior roadmap included generic email, messaging, calendar, document, database, and connector
stories, but it did not commit to named provider matrices, a unified activity inbox, a confirmed
cross-provider identity graph, a persistent Autonomy Center control, deterministic financial correctness,
or an explicit prohibition on money movement and cloud mutation. Slack and Teams were otherwise
limited primarily to incident notifications.

Adding provider names without a domain architecture would create ambiguous authority and testing.
Communications can disclose data or create commitments, financial calculations can be materially
wrong while appearing plausible, and cloud read permissions can expose broad operational data.
These domains require independent packs and release-blocking negative tests.

## Decision

1. AgentMage adds removable Communications, Personal Information and Documents, Finance and
   Budgeting, and Cloud Observer capability packs governed by `PRODUCTIVITY-SYSTEM.md`.
2. The first supported v1.0 GA release includes the reference adapters and gates appended after
   Sprint 126. Sprint 126 remains a stable delivery-and-Windows checkpoint and no longer closes GA.
3. The native Visual Studio Code Chat shell exposes an Autonomy Center control with Disabled, Read
   only, Draft only, Confirm each write, Scoped autonomy, and Autonomous within policy levels.
4. Autonomy is a policy ceiling enforced by the kernel. The effective level is the intersection of
   global, pack, connector, account, workspace, operation, destination, recipient, channel, and
   schedule limits. A display control, prompt, model, message, or connector cannot broaden it.
5. Communications support read and write behavior through separately manifested operations.
   Outlook and Exchange Online, Teams, Gmail, Slack, Proton Mail Bridge, and capability-detected
   IMAP, SMTP, and JMAP are reference providers. Thunderbird, Evolution, and KMail interoperate
   through provider or standard protocols rather than direct private-profile mutation.
6. Calendar, contact, task, and document-repository support includes Microsoft, Google, CalDAV,
   CardDAV, OneDrive, SharePoint, Google Drive, and conformance-gated additional repositories.
7. The unified inbox and work graph preserve native identity, source, freshness, uncertainty, and
   coverage gaps while correlating communications, meetings, delivery work, finance, and cloud
   observations.
8. Event and scheduled workflows compile to inspectable deterministic graphs. They cannot edit
   themselves, widen authority, choose undeclared destinations, or treat external content as an
   instruction.
9. The Finance and Budgeting pack uses fixed-point decimal arithmetic, immutable imports, explicit
   currency and rounding, statement reconciliation, adjustment records, and source-preserving
   evidence. Actual Budget is the first local reference budgeting adapter.
10. Bank and investment data access is read-only in v1.0. Money movement, payments, trades, credit,
    tax filing, beneficiary changes, and financial-account administration are absent at every
    autonomy level.
11. Approval-gated accounting-system writes may cover non-money-movement records only after their
    separate read, draft, precision, reconciliation, and recovery gates pass.
12. AWS, Azure, and Google Cloud adapters are observation-only in v1.0. Cloud writes, remote command
    execution, deployment, secret reads, identity changes, policy changes, and administration are
    absent from the Cloud Observer pack.
13. Cross-pack data movement requires an explicit classified data-flow rule and receipt. One pack's
    credential, cache, identity, context, grant, or retention authority cannot be reused by another.
14. Every pack must be independently disabled and removed, restoring strict-local operation with
    no residual credential, cache, cursor, webhook, schedule, process, socket, or network authority.
15. Sprints 127 through 155 implement and verify the added capabilities. Sprint 156 becomes the
    expanded final v1.0 GA decision gate.

## Consequences

- The first-GA roadmap becomes substantially larger and must continue through sequential,
  dependency-bounded gates.
- Existing completed work and stable identifiers remain unchanged. New requirements, tests,
  stories, and evidence are appended.
- Communications can perform approved writes, but external effects remain exact, bounded,
  attributable, and recoverable.
- Financial analysis gains deterministic foundations without turning AgentMage into a bank,
  payment processor, broker, tax filer, or financial adviser.
- Cloud data can improve diagnosis and planning without creating a second infrastructure execution
  path outside the delivery system.
- The strict-local product and delivery system remain complete when all new optional packs are
  absent.

## Verification

- Canonical documents link `PRODUCTIVITY-SYSTEM.md` and preserve the Decision 0009 boundaries.
- The canonical inventory appends stable product requirements, acceptance tests, and construction
  checklists without changing prior rows.
- `TASKS.md` preserves Sprints 0 through 126 and appends Sprints 127 through 156.
- The security guide adds autonomy, communication, finance, cloud-observer, cross-pack, and removal
  controls plus explicit reviewer protocols.
- Generated registry, policy, traceability, Markdown, Mermaid, link, identifier, and clean-tree
  checks pass after the additions-only baseline is deliberately extended.
