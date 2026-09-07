# S-011-RT01 Seeded Crash-Recovery Results

**Status:** Pass for the current encrypted operational-store boundary  
**Result version:** 1  
**Task:** `11.1.3.3`  
**Fixture data:** Synthetic only

This campaign executes 224 deterministic subprocess runs against real
SQLCipher files. Each child stops with a fixed nonzero process exit before or
after one named durable transition, without Rust unwinding or destructor
cleanup. The parent then reopens or reconciles the files and proves that the
transition has exactly one completed durable result.

## Seed Schedule

Seeds `0` through `223` map deterministically to sixteen boundary families and
two positions. Every one of the 32 boundary-position pairs runs exactly seven
times. The test rejects missing, extra, or uneven coverage.

| ID | Boundary | Before-stop recovery | After-stop recovery | Duplicate assertion |
|---|---|---|---|---|
| `RT-01` | Transaction | Publish the absent session row once | Retain the committed row | Unique identity rejects a second publication; final count is one |
| `RT-02` | Checkpoint | Publish generation one once | Retain generation one | Final generation is one with exactly two checkpoints including genesis |
| `RT-03` | Migration | Upgrade encrypted schema v2 on recovery | Retain schema v3 | Schema history contains versions 1, 2, and 3 exactly once |
| `RT-04` | Key retrieval | Retrieve the synthetic key and create once | Reopen the interrupted open result | One valid schema-v3 store at generation zero |
| `RT-05` | Backup | Create the missing backup once | Retain the completed backup | Occupied-path retry fails and the encrypted-file digest is unchanged |
| `RT-06` | Restore | Create the missing candidate once | Retain the completed candidate | Occupied-path retry fails and the encrypted-file digest is unchanged |
| `RT-07` | Expiry | Commit the due transition once | Retain the committed transition | Repeated expiry returns no row; revision is two with two total events |
| `RT-08` | Deletion | Destroy the synthetic key and ciphertext once | Retain verified absence | Key file and all known SQLite artifacts remain absent |
| `RT-09` | Manifest | Publish the missing artifact manifest once | Retain the committed manifest | Exact identity and uniqueness prevent duplicate publication |
| `RT-10` | Extraction | Publish the missing extraction once | Retain the committed extraction | Exact source identity and uniqueness prevent duplicate extraction |
| `RT-11` | Index | Publish the missing artifact index once | Retain the committed index | Exact manifest identity and uniqueness prevent duplicate indexing |
| `RT-12` | Attempt | Publish the missing workflow attempt once | Retain the committed attempt | Exact attempt identity and uniqueness prevent duplicate creation |
| `RT-13` | Receipt | Publish the missing receipt once | Retain the committed receipt | Exact effect identity and uniqueness prevent duplicate receipt publication |
| `RT-14` | Verification | Publish the missing verification once | Retain the committed verification | Exact verifier identity and uniqueness prevent duplicate verification |
| `RT-15` | Recovery | Reconcile the interrupted recovery once | Retain the reconciled recovery | Recovery state remains exact and does not replay a completed transition |
| `RT-16` | Session checkpoint | Publish the missing session checkpoint once | Retain the committed checkpoint | Generation and checkpoint identity prevent duplicate publication |

## Authority Effect Replay

The operational-store campaign does not invent an external effect for storage
operations. The separate authority restart test covers all six effect-state
checkpoints from prepared storage through result reconciliation. After each
interruption it reopens encrypted schema-v3 authority, produces one terminal
receipt, and proves that a replay driver launches zero times. Together, the two
tests cover durable store publication and the product's sole current external
effect-launch boundary.

## Result

- Deterministic seeds executed: **224**.
- Durable boundary families exercised: **16 of 16**.
- Before/after positions exercised: **2 of 2**.
- Runs per boundary-position pair: **7**.
- Abrupt subprocess stops observed: **224**.
- Repeated completed durable transitions observed: **0**.
- Authority recovery/replay launches observed: **0**.
- Private user records or credentials used: **0**.
- External network operations used: **0**.
- Manual fuzzing operations used: **0**.

## Invalidation Rule

This result becomes stale if a transaction, checkpoint, migration, key,
backup, restore, expiry, deletion, authority-recovery, or artifact-publication
boundary changes. Any new durable transition must receive a named before and
after stop, deterministic seed coverage, a post-restart invariant, and a
duplicate-effect assertion before this gate can pass again.

## Deliberate Limits

- The subprocess uses deterministic `exit(86)` without unwinding; this is an
  abrupt process-stop test, not a kernel `SIGKILL`, host power-loss, torn-sector,
  storage-controller, or filesystem-corruption campaign.
- Synthetic file-backed key state is used only to make deletion observable
  across processes. Live Linux Secret Service interruption remains
  `S-011-IT01`.
- Backup and restore verify fresh immutable candidates. Live-store swap,
  clean-device continuity, and remote backup providers remain later work.
- The campaign runs on the current Linux development host. Fedora/Ubuntu clean
  package, macOS, Windows, release, and support claims remain separate gates.
- Manual fuzzing remains deferred until the end of development and is not
  executed by this verification.
