# Decision 0040: Local Platform Validation and Manual macOS

## Status

Accepted execution-policy decision.

## Context

The private repository exhausted its included GitHub Actions minutes while the
product workflow launched separate hosted Linux and Windows jobs for every push
to the development branch. Those jobs supplied useful clean-machine and native
Windows evidence, but their trigger frequency coupled routine source-control
publication to metered hosted compute.

AgentMage is local-first, and its first-GA platform matrix is Fedora, Ubuntu,
and Windows 11 x64. The development workstation has hardware-accelerated KVM
available and can execute disposable guests sequentially. Apple Silicon macOS
remains a separately evidenced post-GA lane and cannot be represented by a
non-Apple x86_64 host.

The execution venue must not change evidence truth. A local result is not a
GitHub-hosted result, a virtual-machine result is not a physical-host result,
Windows Server evidence is not Windows 11 evidence, and a hosted M1 result is
not MacBook Pro M5 release evidence.

## Decision

1. Routine format, lint, build, unit, contract, Visual Studio Code shell, and
   documentation checks execute locally through repository-owned commands.
2. Native platform acceptance for first GA executes in fresh, disposable,
   hardware-accelerated local guests for Fedora x86_64, Ubuntu x86_64, and a
   properly licensed Windows 11 x64 installation. Each run starts from an
   immutable admitted base image, creates an isolated overlay, checks out one
   exact clean source revision, records an allowlisted environment identity,
   runs the declared commands, exports content-free evidence, and destroys the
   overlay and transient credentials.
3. The local virtual-machine controller, admitted image manifests, Windows
   virtual TPM and Secure Boot configuration, network phases, standard-user
   identities, cleanup, evidence schema, and hostile tests remain implementation
   work in their owning platform and release sprints. This decision does not
   mark that work complete.
4. GitHub-hosted execution is reserved for a manually dispatched Apple Silicon
   macOS compatibility workflow. The workflow requires an explicit budget
   confirmation input, uses read-only repository permissions, contains no
   signing or notarization credential, and never runs on push, pull request,
   schedule, or another workflow.
5. The hosted macOS lane runs source-level Swift package build and test checks
   against the exact selected commit. A pass is preliminary Apple Silicon
   compatibility evidence only. It cannot satisfy MacBook Pro M5 hardware,
   Developer ID signing, notarization, stapling, Gatekeeper, App Sandbox, XPC,
   Keychain, Metal model, installation, lifecycle, or support gates.
6. The existing GitHub product and documentation workflow files remain as
   machine-checked, no-runner sentinels. They have no automatic trigger and
   their jobs are unconditionally skipped. This preserves discoverability and
   prevents an ordinary push or pull request from consuming hosted minutes.
7. Historical GitHub-hosted Ubuntu and `windows-2022` evidence remains
   immutable and accurately labeled. It is not rewritten as local evidence and
   does not satisfy future Windows 11 or release-candidate gates.
8. Git commits, pushes, pull requests, and code review remain available without
   hosted execution. A missing hosted status is not represented as a pass.
   Branch protection must not require disabled sentinel checks until an
   approved execution venue supplies them.
9. Self-hosted GitHub runners are not introduced by this decision. A future
   runner must use a disposable isolated guest, trusted refs, minimal
   credentials, one-job lifecycle, and a separately reviewed threat model.
10. Prices and account budgets remain external mutable facts. The repository
    records no payment method, dollar budget, credential, or claim that a
    hosted macOS run is currently affordable or available.

## Verification

- Machine validation rejects `push`, `pull_request`, `schedule`, and
  `workflow_run` triggers from every billable product workflow.
- Machine validation rejects any enabled GitHub-hosted Linux or Windows job.
- Machine validation requires the product and documentation sentinel jobs to
  be unconditionally skipped before runner allocation.
- Machine validation requires the macOS workflow to use only
  `workflow_dispatch`, the pinned Apple Silicon runner, read-only permissions,
  an explicit budget-confirmation condition, commit-pinned actions, and the
  declared source build and test commands.
- Mutation tests remove each guard or introduce an automatic/non-macOS hosted
  path and require deterministic rejection.
- Existing local product and documentation commands continue to pass without
  invoking GitHub Actions.

## Consequences

- Ordinary pushes stop consuming GitHub-hosted Actions minutes.
- Granular local commits may still be pushed in sensible groups without
  coupling every commit to eight hosted jobs.
- Clean Fedora, Ubuntu, and genuine Windows 11 evidence becomes a local
  automation responsibility and remains blocked until its owning tasks pass.
- macOS source compatibility can be checked economically at deliberate
  milestones when budget exists, while full M5 support stays deferred and
  truthful.
- Reviewers retain exact execution provenance, but GitHub status checks are no
  longer the sole or automatic product-verification authority.
