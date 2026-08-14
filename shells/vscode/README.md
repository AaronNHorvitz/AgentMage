# Visual Studio Code Shell

This TypeScript module registers AgentMage through the pinned stable Visual
Studio Code language-model chat-provider API. It discovers only exact current
profiles projected by the authenticated host; no model ID or family is compiled
into the picker. An unavailable, stale, blocked, incompatible, quarantined,
rejected, retired, or changed profile is absent from ordinary selection and can
never trigger automatic substitution.

The current bounded interaction controller supports `models`, `doctor`,
`export diagnostics`, and `read <workspace-relative-path>`. Model metadata,
status, limitations, diagnostics, denials, cancellation, citations, and receipts
are emitted as structured text through native Chat. Exact model revalidation
runs before every provider response.

The shell has display, interaction, provider-registration, and authenticated
IPC client responsibilities only. It does not read repository files, open a
listener, inspect credentials, or access the Internet. Ordinary activation
launches only the independently verified local host package and authenticates a
one-use local channel; package, platform, or activation failure leaves an inert
unavailable bridge and an empty picker.

The production model-run route, native accessibility evidence, macOS host, and
complete session indicators are not yet integrated. Their absence blocks the
Sprint 23 product gate even though the shell contracts and local tests pass.
