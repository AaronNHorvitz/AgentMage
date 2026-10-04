# Decision 0150: Standalone Evidence Development Host

| Field | Value |
| --- | --- |
| Status | Accepted under owner delegation, 2026-10-04 |
| Date | 2026-10-04 |
| Authority | Decisions 0048, 0052, 0054, 0061, 0063, 0081 and 0088; current owner restart |
| Scope | First repository-local increment of Story 76.2: the Rust host side of the standalone first run |

## Context

- [The work selection record](../verification/work-selection-2026-10-04.md) shows that
  Story 76.2 is the next dependency-ready unit in this lane. The coding rows ahead of it
  need a native Linux login session that this lane lacks. They keep their state.
- Decision 0048 defines the first release workflow. The user selects one local folder,
  asks questions and receives cited answers with evidence cards. The Rust host remains
  the only authority.
- No host operation serves that workflow over the authenticated local IPC. The production
  bootstrap stops at `agentmage.bootstrap.platform_activation_required`. The folder
  prototype behind `scripts/demo.py` uses unauthenticated standard I/O and a loopback
  browser page. It is not a product path, and this decision leaves it unchanged.
- Story 22.5 composed a read-only runtime slice in tests. Admitted sources pass through
  the prepared-source context, inert model proposals, artifact tools and a deterministic
  verifier to one terminal outcome. No host composes that slice.
- The standalone window hosts the Verified Chat surface in a secured webview (Decision
  0048 item 2). No webview crate is in the locked dependency set or the offline cache,
  this lane has no network access, and the workspace forbids unsafe code. Selecting and
  acquiring that dependency needs network access and dependency review outside this lane.

## Decision

1. Add a distinct `standalone-evidence-development-v1` activation. Like Decision 0063, it
   names an explicit private state root and a private disposable root. Production
   bootstrap never selects it, and it never reads production signing state. It cannot
   qualify a package, platform, model or release.
2. Add the host operation `--standalone-evidence-development-host <state-root>
   <disposable-root> <fixture-step-delay-ms>`. It authenticates its exact parent through the existing development
   bootstrap and private IPC. It then serves only folder admission and read-only runs.
   It composes no repository, command, write, Git or network capability.
3. Folder admission is a closed request answered only by the host. The client names one
   absolute folder. The host admits it only when the folder is a canonical directory
   beneath the disposable root. The folder must be owned by the user and not writable by
   group or others. It is opened without following symbolic or magic links. The host
   enumerates only that folder, within fixed depth, entry, file and total byte limits.
   Each entry is accounted as accepted, skipped or rejected with a stable reason, and
   hidden entries are skipped. Accepted UTF-8 text becomes immutable prepared sources.
   The admission digest binds the folder, the accounting and every accepted source.
   Re-admission replaces the snapshot only while no run is prepared or active.
4. Each question is one `EphemeralReadOnly` run over the current snapshot, bound to it by
   the admission digest. The run's tools are the read-only artifact tools over admitted
   sources and nothing else. The tool boundary allows only those tools on the current
   snapshot. Selecting the folder is the read authorization, so no per-call approval
   prompt exists. The required context source is a small folder inventory. Document
   sections are included as the window allows, and the context manifest records each
   omission.
   Each run is also one job in a development operational store in the host's state root,
   opened under the development key there, as the coding host does. Over the channel a
   run is cancelled only through its job's ledger under the authenticated client's scope
   (Decision 0120). The store keeps job control records, never folder content.
5. The only selectable model is the source-controlled fixture profile
   `standalone-evidence-fixture-v1`, labeled `executable-scripted`. Like the coding
   development host's scripted profile, it is admitted only for contract testing. The
   checked-in deterministic catalog profiles keep their `no-product-selection` limit
   and are not used. The fixture is not a language model. It searches each admitted
   source for the question's terms of four or more characters, not counting common words. It takes at most four terms, as written,
   within the run's tool-call limit. It then quotes each line that matches the most
   terms, provided the line matches at least two terms, or one when the question has
   only one. Each quote carries a citation of the exact search evidence. With no
   qualifying line, it says that no admitted source contains matching text. Any other
   profile is refused as unavailable, because no approved catalog entry exists. For
   cancellation tests between processes, the host may be launched with a development
   delay of at most two seconds before each proposal, during which the fixture keeps
   observing cancellation.
6. The verifier admits success only for a closed answer with at least one statement,
   where every statement cites evidence observed in this run. Every quoted text must
   appear exactly in the cited fragment. It checks citation membership and exact quotes,
   not semantic entailment. An answer without citations is a truthful non-success.
7. The client side is the standalone application's bridge. It launches the sibling host,
   authenticates it and exposes closed states: starting, ready, unavailable with a code,
   and safe mode after a host failure, which a restart with a new host leaves. Every
   question goes through the development CLI's verified run driver. A cancellation is
   sent once per request. The bridge projects only a verified run's statements into
   evidence cards, and only when each card names this run's evidence and an accepted
   file of the snapshot. The window, keyboard and screen-reader presentation and the
   packaged local assets are not implemented. They are blocked on the webview dependency
   above, and no other client substitutes for them.
   `agentmage-standalone` without arguments says so and exits. Its `--development-stdio`
   mode is the bridge's message channel: closed JSON requests with strictly increasing
   sequences on standard input, and closed JSON events on standard output. It serves
   development and tests; it is not the product's presentation.
8. The folder operation raises the IPC wire to 18. The runtime producer contract becomes
   version 4, with the same records on the new wire. Version 3 and its fixtures stay as
   they were, as Decision 0144 did for version 2.
9. Evidence labels stay `component` or `executable-scripted`. This decision changes no
   status-model entry, enables no model and makes no Windows, package or release claim.

## Limits

- Runs are ephemeral. Durable sessions, conversation history across questions and
  reconnect to a running run are not part of this increment.
- The host's socket lives in the state root, and a Unix socket path is limited to 107
  bytes, so the roots must be short, as for the coding host.
- The fixture's term match is a deterministic stand-in for a model. Answer quality with
  a real model is unassessed.
- Hardware fit and model selection belong to Story 76.3.

## Consequences

- `shells/host`: the activation, folder admission, runtime factory, service, host
  operation, IPC folder operation, bridge, `agentmage-standalone` and their tests; the
  coding key provider can open its key in another development state root.
- `platforms/linux`: the sibling-host launch for the new operation, and the development
  platform adapter admits the new activation's name.
- `fixtures/runtime-producer/v4`, the version 4 contract document and its checks.
- `docs`: the work selection record, this decision and the local testing guide.
- No TASKS.md row changes state until its evidence exists.
