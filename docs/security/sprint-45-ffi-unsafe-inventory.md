# Sprint 45 FFI and Unsafe Inventory

## Result

Sprint 45 adds no `unsafe` Rust block, foreign-function declaration, native source
file, or new native library dependency. The repository-wide source scan finds one
existing reviewed Rust FFI file:

| Path | Purpose | SHA-256 at this evidence source revision | Ownership |
|---|---|---|---|
| `platforms/windows/src/native_identity.rs` | Narrow Windows current-process and token identity observation through Win32 APIs | `5450b3a4c87fef339d27ccf187270bd4aa56199f9c476857dcf3f846abed00fe` | Decision 0024 Windows boundary |

The file contains thirteen explicit `unsafe` expressions. No `.c`, `.cc`, `.cpp`,
`.h`, `.hpp`, `.m`, or `.mm` source file exists outside ignored build output.

## Sprint 45 Boundary

The repository-map, language-service, package-scaffold, test-generation, and host
composition modules all retain `#![forbid(unsafe_code)]` through their crate roots.
Their Tree-sitter and SHA implementations are dependencies resolved by Cargo rather
than new AgentMage-owned FFI. This inventory is not an independent safety review of
those dependencies or the existing Windows boundary.

The Windows file and every dependency-native edge remain subject to the owning
platform review, sanitizer evidence where applicable, and the deferred final manual
fuzz campaign before release.

