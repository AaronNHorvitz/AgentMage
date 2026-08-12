# Visual Studio Code Shell

This TypeScript module registers the Phase 9 AgentMage Secure Read provider
through the stable Visual Studio Code language-model chat-provider API. It
accepts only `read <workspace-relative-path>`, selects exactly one local VS Code
workspace, renders two modal confirmations, and returns bounded content, a
local citation, or a content-free denial.

The shell has display, interaction, provider-registration, and authenticated
IPC client responsibilities only. It does not read repository files, launch a
process, open a listener, inspect credentials, or access the Internet. The
authenticated Linux client is implemented for package injection, but ordinary
activation deliberately installs an unavailable bridge until Phase 11 provides
an independently signed host package, trusted endpoint, and direct one-use
launch credentials.
