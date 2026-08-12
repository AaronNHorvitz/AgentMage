# Windows Platform Boundary

This crate freezes the first version of the Windows adapter contract and its
required native enforcement controls. It deliberately exposes no filesystem,
IPC, worker, key, storage, package, or model effect implementation.

Compilation and contract tests on Windows are necessary but insufficient for
platform enrollment. Windows remains blocked until the native implementation
and the independent standard-user test matrix in `WINDOWS-BOUNDARIES.md` pass on
a real Windows 11 x64 runner.
