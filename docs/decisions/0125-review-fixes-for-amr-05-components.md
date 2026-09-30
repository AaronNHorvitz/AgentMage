# Decision 0125: Review Fixes for the First AMR-05 Components

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-09-30 |
| Date | 2026-09-30 |
| Authority | Decisions 0054, 0081, 0088 and 0124; current owner restart |
| Scope | Findings F1 to F3 and note N1 of the independent review of `8469649d`; amends the Decision 0124 action history |

## Findings

An independent read-only review of `3c69304c..8469649d` passed with three low
findings and five notes.

- F1: replaying an action history checked sequence, links, digests, time order
  and the head, but not the identifier rule. A retained entry whose action
  identity, reason code or grant identity was never redacted, with a consistent
  digest, replayed and could then be exported. The export rescan catches only
  the secret detector's classes, not prompts or paths. Only the owner's own
  retained positions reach replay, so no outside path reached it, but Decision
  0124's "a raw prompt, a path or a token never enters the history" held only
  at `append`.
- F2: four rules had no test that failed when the rule alone was removed: the
  issuer key a revocation list names against the key presented, the export
  rescan, the plain-identifier character set beyond the secret detector, and
  the previous-link check of replay.
- F3: the evidence reports that own the rows AMR-05.2, 05.3 and 05.5 extend
  (Story 15.2, Story 31.1, Sprints 15, 31, 34, 78, 79, 108 and Story 49.2)
  were left stale because they were already stale through other inputs. Their
  producers exist and were not run, and they were not listed as blocked.
- N1: an expired position kept the deadline its entry had, and no digest bound
  it once the entry's content was gone.

## Decision

F1. Replay refuses, as an integrity failure, any kept entry that `append`
could not have produced. Each identifier must be the redaction marker or a plain
identifier the shared secret detector does not flag. The entry's redaction count
must equal its markers. Every digest must be well formed and the evidence
digests sorted, unique and bounded. The retention deadline must follow the
recording time, and an entry without authorization must be denied or cancelled.
Redacting again during replay is not an option: it would change the entry's
digest. The export rescan stays as a last guard.

F2. New tests isolate each rule:

- two trusted issuers, where a list signed and presented by one names the
  other's key;
- a history built directly, bypassing `append` and replay, with a token in an
  identifier, which only the export rescan withholds;
- an identifier with uppercase letters and an underscore that the detector does
  not flag, which only the character set redacts;
- a chain of kept, expired and kept positions in which the expired place is
  replaced, which only the previous-link check refuses because the head binds
  only the last digest.

A table test changes one field of a one-entry chain per case, re-digests it and
keeps the head consistent, so each replay rule alone must refuse it.

F3. The evidence pass for this batch runs every existing producer of those
reports. It records any producer that cannot pass in this sandbox as blocked,
with its cause, in the batch record. Sprints 78 and 79 have producers
(`scripts/sprint_78_evidence.py` and `scripts/sprint_79_evidence.py`), so none
is added.

N1. An expired position keeps only its sequence, its previous link and its
digest. Its deadline went with its content, since no digest could bind it
afterwards. The other notes need no change: N2 describes an inspection
surface that loads nothing, N3 a notice whose words are true in both cases, N4
an integration duty that AMR-05.9 takes on, and N5 a count in the request.

## Verification boundary

Engine unit tests cover each replay rule, the redacted round trip, the export
rescan and the expired-place link. The host unit test covers the named issuer
key. The action history has no persisted form yet, so dropping the expired
deadline needs no migration. No test uses a native host, a network, a model or
the GPU. Independent re-review of this batch is requested and remains open.
