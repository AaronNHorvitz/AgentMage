# Story 11.2 AC3 lifecycle acceptance

Story acceptance criterion `11.2.AC3` passes for the current encrypted operational-store scope on
Linux. Whole-store backup and fresh-candidate restore preserve the complete schema, including all
31 source/workflow families, under a distinct destination key without overwriting an occupied path.
Source release, expiry, deletion, and collection update the logical source state and its exact
runtime-artifact reference in one transaction, preserve held rows, and retain a shared payload until
the final live reference is released.

Every current source/workflow family has one content-free derived export. Synthetic-canary evidence
scans encrypted main, WAL, SHM, backup, derived export, receipts, diagnostics, and stable errors;
the raw value is absent, the backup reopens under its own key, and no raw canary or private user data
is retained in evidence.

This closes the criterion for the current store and synthetic surfaces. Complete product uninstall,
separately managed copies, physical remanence, later active logging/model-context/adapter surfaces,
cross-platform and installed-package execution, independent review, Story or Sprint completion,
packaging, and release readiness remain separate gates.
