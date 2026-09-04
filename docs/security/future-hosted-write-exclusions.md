# Future Hosted-Write Exclusions and Prerequisites

The first GA and v0.7 candidate exclude every hosted mutation: issue/comment creation or edit,
label/assignment/milestone/project changes, review submission, workflow dispatch or cancellation,
notification state change, branch/ref/file/release mutation, commit, push, merge, deployment, and
security-setting change. Provider mutation methods, submission commands, publication hooks,
background synchronization, ambient credentials, and reusable raw credentials must remain absent.

A future hosted-write proposal requires a separate accepted Decision. Its threat model must cover
exact destination and canonical transport identity; minimum per-operation credentials; preview and
fresh approval; idempotency and uncertain-result reconciliation; concurrent/stale provider state;
branch/rule/protection enforcement; injection and confused-deputy attacks; audit receipts; rollback
or compensating action; revocation and deletion; rate limits; cancellation; native platform
isolation; cross-account/repository/host separation; and explicit release/support gates. Read-only
evidence, local drafts, or shadow patches cannot substitute for that decision or evidence.
