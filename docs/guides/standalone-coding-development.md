# Standalone Coding Development Harness

This is the explicit disposable Linux development activation accepted by Decision 0063. It runs
the actual `agentmage` CLI and ordinary `agentmage-host` process through authenticated private IPC,
the shared coordinator, native tools, exact grants, Linux confinement, and the deterministic
verifier. It does not activate production transport or qualify the scripted fixture as a coding
model.

## Build and setup

Start only when at least 16 GiB RAM is available. Build with no more than four Cargo jobs:

```bash
systemd-run --user --scope --quiet \
  -p MemoryHigh=5G -p MemoryMax=6G -p MemorySwapMax=512M \
  env CARGO_BUILD_JOBS=4 cargo build -p agentmage-host --bins --locked

python3 -m scripts.coding_harness setup --root /tmp/agentmage-coding-1
python3 -m scripts.coding_harness diagnose --root /tmp/agentmage-coding-1
```

`setup` refuses an existing target. It creates owner-only state and disposable roots, an ordinary
private Git repository on `agentmage/tasks/coding-fixture`, an exact path-bound activation marker,
and a genuine failing Python validation. Use a short root because Linux filesystem Unix sockets
have a bounded path; `diagnose` reports `transport_path=false` before launch when the worst-case
host socket would not fit.

The diagnosis separates these classes:

- `agentmage_binary` and `host_binary`: implementation/build availability.
- `activation`, private-root fields, `git_branch`, and `git_clean`: activation and workspace trust.
- `transport_path`: private IPC path viability.
- `confinement`: exact local Bubblewrap, systemd, and Git prerequisites.
- `profile_id` and `qualification`: scripted fixture identity and its deliberately limited
  `executable-scripted-only` status.
- `state_key`: whether the separate development SQLCipher key has been created.
- `lifecycle`: `ready`, `starting`, or `running` for the exact recorded process identity.

## Start, status, and stop

Run the genuine fail-repair-pass workflow and retain raw output in a new private directory:

```bash
python3 -m scripts.coding_harness start \
  --root /tmp/agentmage-coding-1 \
  --scenario failed-test-repair \
  --objective 'repair the failing synthetic add test' \
  --approve-this-run \
  --log-dir "$HOME/.local/state/agentmage-codex-coding/runs/repair-1"
```

`--approve-this-run` is an explicit invocation-scoped choice. It does not create wildcard,
persistent, automatic, or production authority. Without it, every exact protected challenge is
shown on stderr and only the literal answer `yes` allows that operation.

From a second terminal, inspect or cancel the exact recorded process:

```bash
python3 -m scripts.coding_harness status --root /tmp/agentmage-coding-1
python3 -m scripts.coding_harness stop --root /tmp/agentmage-coding-1
```

`stop` refuses `ready` and `starting` states. Once runtime preparation has installed the signal
handler and created the isolated key, it validates `/proc` executable and argument identity before
sending SIGINT. The runtime must then emit cancellation-requested and cancellation-observed events
and a truthful terminal result. A cancellation request alone is not completion.

The `no-op` scenario performs verified Git inspection without source mutation. The `slow-cancel`
scenario is a non-qualified test fixture that holds one cancellable model call for signal and
slow-client acceptance; it is not a performance simulation or model result.

## Failure handling

Invalid marker contents, permissions, workspace identity, Git state, profile identity, peer
identity, cursor, approval, or confinement fail closed. The wrapper never resets, cleans, stashes,
commits, deletes, or overwrites a working tree. Use a newly named root for another campaign.

Log directories are create-only and mode `0700`; `stdout.jsonl`, `stderr.log`, and `result.json`
are mode `0600`. Preserve failed and rejected runs alongside successful ones. A scripted success is
only executable-path evidence. Model qualification requires the separate repeated Muse Glimmer and
gpt-oss-20b campaigns through these same binaries and their exact admitted profiles.
