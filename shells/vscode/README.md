# Visual Studio Code Shell

This TypeScript module registers AgentMage through the pinned stable Visual
Studio Code language-model chat-provider and chat-participant APIs. It discovers only exact current
profiles projected by the authenticated host; no model ID or family is compiled
into the picker. An unavailable, stale, blocked, incompatible, quarantined,
rejected, retired, or changed profile is absent from ordinary selection and can
never trigger automatic substitution.

The current bounded interaction controller supports `models`, `doctor`,
`export diagnostics`, and `read <workspace-relative-path>`. Model metadata,
status, limitations, diagnostics, denials, cancellation, citations, and receipts
are emitted as structured text through native Chat. Exact model revalidation
runs before every provider response.

The stable `@agentmage` participant enumerates only prompt references supplied on the current
request. It resolves `string`, `Uri`, and `Location` values through documented APIs, checks file
revision before and after each bounded read, and streams exact bytes through the existing
authenticated Engineering artifact RPC. Unsupported, unavailable, stale, omitted, failed, and
cancelled references remain visible; no ambient workspace scan substitutes for missing bytes. The
language-model provider is explicitly lossy: any non-text part stops visibly and directs the user
to the participant path.

The shell has display, interaction, provider-registration, request-bound reference resolution, and authenticated
IPC client responsibilities only. It cannot enumerate or parse repository content, open a
listener, inspect credentials, or access the Internet. Ordinary activation
launches only the independently verified local host package and authenticates a
one-use local channel; package, platform, or activation failure leaves an inert
unavailable bridge and an empty picker.

Installed-VSIX accessibility, remote-topology, Windows, macOS, and production-model evidence remain
external qualification gates. Their absence blocks the Sprint 23 product gate even though the
stable participant and provider-compatibility contracts pass locally.
