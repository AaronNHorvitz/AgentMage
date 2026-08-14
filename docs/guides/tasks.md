# Task Guide

Canonical task records include stable identity, owner, optional project, evidence, dependencies,
status, blocker, next action, priority, and optional deferral. Derived active, blocked, next-action,
deferred, and terminal views never replace the Markdown record.

Every transition requires new content-addressed evidence and produces an exact no-write preview.
Blocked tasks require a blocker, deferred tasks require a deferral boundary, actionable tasks
require a next action, and terminal tasks cannot be silently reopened. Duplicate warnings identify
matching normalized title, project, and owner without merging records.
