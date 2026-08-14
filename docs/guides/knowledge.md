# Knowledge Guide

AgentMage treats validated user-owned Markdown as canonical knowledge. Stable record identity is
independent of path. Record kinds, fields, links, evidence, privacy, retention, and timestamps use
closed schemas; unsupported fields and unresolved links fail before use.

Indexes, dashboards, workflow results, and JSON Lines files are derived views. Correct the source
Markdown and rebuild them. In v0.2, AgentMage may inspect records and show exact create or update
previews, but it cannot apply a write, move, or deletion.

When an answer lacks current source evidence, preserve `Unknown/Blocked` rather than promoting a
summary, memory, model output, or derived index to source authority.
