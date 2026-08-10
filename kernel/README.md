# Kernel

The kernel owns interface-independent AgentMage policy and authority. Shared
contracts live in `contracts`; their implementation and orchestration live in
`engine`. The kernel cannot depend on platform adapters, capability packs, or
shells.
