# Declarative Skills Guide

Declarative skills are bounded data packages, not plugins or agents. Inspect identity, source,
hash, signer or provenance, license, version, compatibility, purpose, file inventory, requested
scope, precedence, and trust state before admission.

Only prompt, schema, example, and template data is accepted. Skills cannot read files, run shell
commands, resolve secrets, use a network or connector, register tools, create grants, approve
actions, execute code, expand a workspace, promote memory, or write. Conflicting semantic keys are
reported and omitted. Removing a registry entry grants no file-deletion authority.

The v0.4 source candidate defines nine coding skills under this same contract. Their advisory tool
names describe existing evidence they may consume, not tools they can invoke. Each definition must
name supported or lexical-only language coverage, mandatory validation, evidence-specific
completion, bounded path scope, and the complete prohibited-operation set. Invalid definitions are
disabled. See the [coding-skill architecture](../architecture/coding-skills-and-release-boundary.md)
and [bounded coding guide](./bounded-coding-workflows.md).
