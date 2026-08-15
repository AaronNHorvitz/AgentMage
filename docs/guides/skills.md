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

The v0.6 source candidate also defines eight executive-assistant skills under the same permanent
authority ceiling. They project approved local records into cycle views, trackers, priority
explanations, evidence briefs, correspondence reviews, local message-export triage, portfolio
reviews, and privacy audits. See the
[executive-assistant guide](./executive-assistant-local-workflows.md) and
[architecture boundary](../architecture/executive-assistant-and-portfolio.md).

Six meeting-record skills extend the v0.6 source candidate under the same zero-authority contract.
They cover agenda and request drafts, attendee evidence, source-preserving transcript cleanup,
minutes, closeout, and recurring continuity. See the
[meeting workflow guide](./meeting-records-local-workflows.md) and
[meeting architecture boundary](../architecture/meeting-records-and-continuity.md).
