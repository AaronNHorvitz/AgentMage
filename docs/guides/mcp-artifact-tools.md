# MCP Artifact Tools

AgentMage exposes its existing source-artifact inspection family to an MCP client only when an
exact reviewed read-only manifest is enabled. The MCP names are
`mcp.agentmage.artifact.list`, `mcp.agentmage.artifact.metadata`,
`mcp.agentmage.artifact.read`, `mcp.agentmage.artifact.range`,
`mcp.agentmage.artifact.sections`, `mcp.agentmage.artifact.search`, and
`mcp.agentmage.artifact.get_log_errors`. Their schema versions, input and output schemas, limits,
effects, and single-use `workspace_read` grant template are identical to the native tools.

The MCP prefix prevents a server from shadowing a native registration. Each request is admitted by
the MCP gateway, translated only at the tool identity, and revalidated by the existing native
`ToolRegistry` and `ToolDispatcher`. MCP does not receive a parser, source store, grant issuer,
approval path, model channel, or effect executor. Server output remains untrusted and cannot serve
as completion evidence without the existing verifier.

Disabling the manifest removes its MCP registrations. The native tools are rebuilt independently
and remain available. A platform adapter must prove descendant termination and remove its process,
socket, cache, and transport state before claiming complete removal.

MCP cannot recover bytes it was not given through an admitted usable handle or staged artifact.
Malformed, stale, oversized, restricted, unsupported, cancelled, duplicated, timed-out, crashed,
or cross-session requests retain the same native denial and receipt semantics. Native installed
transport parity and residue testing remain required before support can be claimed.
