# Decision 0204: Open study of how agent memories are linked

| Field | Value |
| --- | --- |
| Status | Accepted planning direction; no implementation or method selected |
| Date | 2026-11-11 |
| Authority | Owner's memory-linking study instruction of 2026-11-11 |
| Scope | Later CAP-29 study; T-315 |

Stored facts help a run only when context selection can reach the facts it
needs. Matching words or meaning can find similar facts, but a needed fact may
instead be reached through a link from another fact. Which facts to connect,
when to connect them and how to follow those connections remain open questions
for AgentMage's source-backed project memory. A graph alone does not establish
useful recall. Study the choices and compare their results before adopting one.

The study has three requirements:

1. Linking is a replaceable part with a name and version, whether it runs in
   the Memory role's service or AgentMage's rebuildable derived index. Every
   link records the method and version that produced it. Different methods
   can coexist over the same facts; all links from one method can be disabled
   together as a set without rewriting any stored fact or link.
2. AgentMage's memory-writing and context-selection steps are replaceable too.
   The writing step chooses source-backed facts, decisions and events from a
   run. The selection step asks for memories and follows their links. Run
   evidence records the names and versions of the writing, linking and
   selection methods in force, allowing a run to be repeated with another
   method.
3. Adopt a method through measurement on the same fixed tasks under both
   methods. Report needed facts reached, wrong facts brought into context,
   cost and effects on task outcomes. Existing thresholds and held-out rules
   remain unchanged; held-out results never choose a method.

T-315 starts no earlier than the first CAP-29 work. This decision changes no
current work order and adds no work to T-303, T-306 or any other open row.
Measure-first and coding-agent priorities continue. Nothing is implemented or
qualified by this planning record. Existing sources remain authoritative;
memory initially remains a rebuildable derived index with scoped provenance,
correction, revocation and deletion obligations.

Describe the question and any later method originally under
[IP-POLICY.md](../../IP-POLICY.md). Do not copy or paraphrase another product's
design, documentation or code, identify private projects, or search for, read
or cite patents. T-315 requires independent review before closure.
