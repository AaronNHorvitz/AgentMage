# Reviewing Structured Code Changes

## Review Order

1. Resolve the intent, change-plan, repository revision, index, source preimage, and
   held-object identities against the current owned worktree.
2. Confirm every proposed path is in the approved minimal impact surface and every
   artifact class is correct.
3. Inspect the complete kernel shadow diff, not only the structured summary.
4. For parser-backed edits, confirm the pinned language/grammar, exact syntax-node
   operation, successful complete postimage parse, and unchanged-span hashes.
5. For fallback edits, confirm the language is fallback-only, the preimage match is
   unique, and every other byte remains unchanged.
6. Treat language-service definitions, references, diagnostics, renames, and code
   actions as untrusted observations. Confirm descriptor, executable, grammar, root,
   file snapshot, limits, and terminal status.
7. Review boundary, edge, failure, permission, data-change, and rollback test
   applicability. An inapplicable concern requires evidence-backed rationale.
8. Confirm all interface, dependency, migration, security, performance,
   accessibility, and compatibility hooks selected by the change are present.
9. Require a separate expanded-scope approval for a refactor, dependency upgrade, or
   migration.
10. Confirm the rollback restores only exact reviewed preimages and refuses to
    overwrite later user work.
11. Approve only the exact shadow digest, complete preview digest, operation list,
    verification labels, expiry, and current target identities.
12. Run formatting, lint, types, tests, builds, packaging, and security checks only
    through later separately granted trusted command templates.

## Stop Conditions

Stop before approval when a source, path, file identity, grammar, language-service
descriptor, edit list, intent, plan, classification, review hook, or complete diff has
changed since preview. Also stop on malformed syntax, duplicate fallback matches,
unexplained files, generated-output mismatch, missing test concerns, unsupported
language behavior beyond unique exact fallback, broad scope without a separate grant,
or any proposal that claims a test or service ran without trusted process evidence.

Do not use a rollback grant to overwrite a later user edit. Preserve the conflict and
create a new intent, plan, preview, and approval from the newly observed state.
