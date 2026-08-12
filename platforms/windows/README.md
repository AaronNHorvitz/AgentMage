# Windows Platform Boundary

This crate freezes the first version of the Windows adapter contract and its
required native enforcement controls. Decision 0024 adds one narrow native
current-process identity observer: query-only token access, redacted user SID
digest, session, elevation state, and bounded executable digest. It exposes no
filesystem, IPC, worker, key, storage, package, or model effect implementation.

Cross-compilation and genuine Windows-runner identity tests are necessary but
insufficient for platform enrollment. Windows remains blocked until the native
implementation and the independent standard-user test matrix in
`WINDOWS-BOUNDARIES.md` pass on a real Windows 11 x64 runner.
