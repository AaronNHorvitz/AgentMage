# Decision 0208: Trusted endpoint thinking control

| Field | Value |
| --- | --- |
| Status | Implementation detail under the accepted owner mission |
| Date | 2026-11-13 |
| Authority | Measure-first directive; T-303/T-306 |
| Scope | Credential-owning local chat transport and trusted API settings |

The six-profile JSON-object repeat recorded 36 incomplete responses, the largest
refusal cause in 180 development attempts. A diagnostic repeat of two development
tasks on A4B retained the actual upstream responses: both used their full output
allowances, returned empty answer content and returned substantial separate
reasoning content. This establishes the cause for these two attempts only.

Changing the same server's existing template option to `enable_thinking: false`
produced two strict ungraded proposals on the same tasks. This is a diagnostic
of server configuration, not a task score or a test of the new API setting.
Prompts, strict edit checks, deadlines and budget formula stayed fixed; rendered
input changed by two tokens per task. Neither hidden nor held-out tests ran.

Trusted endpoint settings may select `thinking_mode: provider_default | disabled`.
The default omits any thinking extension, preserving existing request bytes.
Disabled requests `chat_template_kwargs: {"enable_thinking": false}` from a
supporting endpoint. The exact upstream body, including this option, is bound
by the prepared request. This option is independent of response framing and
output syntax, and conversations do not require a workspace or repository.
Native development launchers retain their separate default configuration.

The setting is an optional endpoint extension. A server can reject or ignore it;
there is no retry, fallback or assertion that thinking was disabled. Only trusted
controls can select it. Client chat/proposal bodies cannot select it, and invalid
or duplicate settings leave the prior configuration unchanged. Omitting it on
a new choice restores the endpoint default. Existing settings revisions apply.

Reasoning remains untrusted metadata, never answer text, an instruction or an
approval. An empty answer still fails as incomplete, including when a server
ignores the option. Strict edit parsing, paths, exact preimages, overlap and size
bounds remain unchanged. No deadline, token or byte ceiling, credential policy,
network destination or tool permission changes.

Verify the API option against the real endpoint, then repeat the development
measurement in resumable resource jobs before further tuning. Report diagnostic
and full-suite results separately. Task grading and native execution remain
blocked by the existing admission and original grading-control prerequisites.
No original control is reconstructed. This adds no row, model role, independent
acceptance, release or owner-readiness claim. Independent core review is required.
