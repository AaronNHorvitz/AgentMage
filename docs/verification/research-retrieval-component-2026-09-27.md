# Canonical Research Retrieval — Component Verification

Date: 2026-09-27. Source parent: `1862d485d48a6f77984d2f1bfd42040aa8bc9677`.
Status: Accepted under owner delegation, 2026-09-20. Authority: Decision 0054.
This bounded implementation supports AMR-03.1.2; it closes no package, native
connected-research, model, independent-review, platform or release gate.

## Implemented boundary

The [canonical source contract](../architecture/canonical-research-source.md) reads
complete retained sources through the existing durable authority runtime, SQLCipher
store, coordinator, grant issuer, journal and artifact owner. Six acyclic full
artifacts bind the original call, packet, native material, binary response frame,
canonical tool result and bundle. A digest, preview or provider citation is not
substituted for source bytes. Expected native identities come independently from
trusted admitted composition, not from the source being checked.

Reads verify the actual successful terminal transaction and receipt, issued versus
consumed grant revisions, requested/started/completed event order, original reserved
budget revision and full plan, native parent interval, full-byte identities and
current artifact ownership/lifecycle. A retained source may outlive the original
dispatch deadline or task cancellation without renewing permission or refunding
budget. The trusted read clock cannot precede the latest verified run event or
budget clock. The reader adds no global clock source or persisted read clock.

The read-work ceiling is 16,384 events and 8 MiB of serialized history. Excess
history is refused before full allocation, never truncated into accepted evidence.
The existing exclusive connection and store mutex remain unchanged. Returned source
values have no public constructor, clone or deserializer and expose inert content.
No network tool, provider, second store, execution loop or query language is enabled.

## Final-source verification

The reserved CPU run completed with exit zero. The 12-entry source manifest matched
before and after the run, then again during reconciliation. Manifest SHA-256:
`4560630d1a482aa8cbad3340fb6a5efa7edc78e23801a454b3e6266eeaea6d18`.
Raw run log SHA-256:
`b84a63455c2cf489a9e03723d0078730334e3077ea99e457a9f8f659b6d5da88`.
Private logs remain outside public Git.

| Check | Observed result |
| --- | --- |
| Focused journal/source tests | 38 passed |
| Engine library | 1,177 passed; 7 ignored |
| Host library | 324 passed; 8 ignored |
| Linux default library | 181 passed; 50 ignored |
| Linux optional-worker library | 192 passed; 55 ignored |
| Optional worker binary tests | 20 passed; 3 ignored |
| Linux classification integration | 1 passed |
| Engine integration files | 67 passed across 18 files |
| Engine documentation tests | 11 passed, including three opaque-source compile-fail cases |
| Eight Python suites | 93 passed |
| Actual CLI/host scripted matrix | All 22 cases passed |
| Strict Clippy | Workspace/all-targets and optional-worker/all-targets passed |
| Other checks | Effect mediation, offline source/dependency closure, status, formatting and 544-document lint passed |

Counts overlap where focused or feature variants select the same tests; they are
not an aggregate unique-test count. Ignored native cases were not selected here.
All heavy work used the shared reservation, existing capped scope, at least 16 GiB
available RAM at start, one Cargo job and one test thread. No inference or GPU was used.

The focused cases include 26 coherent-but-unbound variants, eight owner/native-pin/
clock variants, three full-source drift/lifecycle variants, history count/byte
refusal, competing-owner refusal, read-clock rollback, terminal-run reads and
reopen after budget advance/cancellation without replay. Synthetic drivers and
payloads exercise real canonical owners; they do not prove native HTTPS execution.
Direct historical-accounting cases recheck the original plan and complete journal.

The scripted matrix used the actual binaries in fresh disposable repositories.
It retained expected denial, stale/replayed approval, cancellation, overflow,
false-completion, artifact-integrity and rollback-conflict refusals alongside
successful patch/test/revision, new-file, multi-file, rollback and bounded correction
cases. Its report explicitly says `executable-scripted-only`. Report SHA-256:
`c4c2c4e88d82ae06e12dc44b6cc305ffa244956748191f72364c4edb442cbeb6`.

| Executable | SHA-256 |
| --- | --- |
| `agent` | `873bd824fa863734e24e8708430c1bb52d3e484fd9d0e31dbe831e210eb7444e` |
| `agentmage` | `50a2e38ccb2d1f0e08e9f45c66c8d69787af3aea8c90066451a459b305531c0b` |
| `agentmage-host` | `c56137ada4c9b6ea26b1fd8f34ad788ed35187491cfef9f8c4214cac6f3b1f7b` |
| `agentmage-read-only-worker` | `7cbcf459492012491a27607225ddc5c778c589b81956a462441d3ed4c9dec077` |

## Retained failures and corrections

| Attempt | Actual disposition | Log SHA-256 |
| --- | --- | --- |
| First focused build | Exit 101; seven test-only compilation errors, subsequently corrected | `37832ec2bff5eb326b4da8acf5d18850b53f0e494001290f59d21784b0627033` |
| Second focused run | Exit 101; 27 passed, six fixture failures from sealing publication events before attaching payload references | `51c45ac354edd10cf9322ebbe3b5f9454db26ec09c9affe70434fbc5242fa66a` |
| Third focused run | Exit zero; earlier snapshot, not blanket acceptance of later edits | `a7e7a60b163606278dd4c0d05f5ca11c9157f82a5f74572fee83f1ba25486c6b` |
| First broad run | Exit 101; 35 passed, one invalid second-owner fixture correctly refused with ConcurrentWriter; later checks did not run | `660077e0ee044ca0b6226fc4989299597be32b677e1aa92d1e3fc94e10201331` |

After the first broad run terminated, the adversarial fixture used the existing
test owner's shared mutex and added a competing-owner refusal regression. Production
locking was not relaxed. The synthetic proof-consuming helper moved into the existing
registered dispatch-test boundary without an audit exemption. The constructor
compile-fail case now tests private fields with correctly typed arguments. The
later-event read-clock guard was identified by source inspection, not an executed
pre-fix failure; its final-source regression passed. All original failures remain retained.

## Acceptance and freshness

This record does not reaccept historical planning/privacy, model or release evidence.
Native connected-worker success, configured provider composition, claim-linked
partial/conflicting reports, research-to-code
campaigns and independent acceptance remain open. Muse's earlier eight-case coding
results remain bounded historical development evidence; GPT-OSS remains separately
unqualified. No task checkbox changes follow from this component checkpoint.

The closed source batch had exactly one SBOM write. Its first evidence attempt
passed supply-chain checks, 11 supply-chain regressions, 52 runtime-schema tests,
69 contract-boundary Python tests, five report builders with 41 regressions and
545-document lint, then exited one: the security map correctly refused five
configuration reports made stale by the changed engine binding. Retained log:
`c6e8bcc4a798f1bf1c4e0fed126622af8a323380bb020119f289515e5d4505f4`.

The reserved continuation renewed those five prerequisites and their security map,
then 12 affected working-tree-bound storage reports, executing their required
commands and 95 builder regressions. It exited zero without another SBOM write.
Log: `6596a18037eeccd1412b1efac1a267fbecc5067e48837f590dcb9cc48d572fa3`.
The dependent retained-storage security index and five regressions also passed;
final supply-chain and source-identity checks passed. That separate final log is
`e28d39ad52f3fa0af11927b52e7aeb6036e8c4684059b9136fd212c6cf38da9b`.
Inspection found unchanged acceptance/limitation fields and full input bindings;
profile registration remains inactive and independent acceptance remains unperformed.

The final direct named whole-file inventory covers 750 JSON artifacts and 675
bindings to changed inputs: 74 current, 598 previously stale or historical, and
three newly stale relative to the working source. The three retained historical
records are `operational-schema-v3.json` and `s-011-ut01.json`, both pinned to
`ac55cb59db3057f95a82d66a93d21719d81d115d`, and `current-product-canary-sweep.json`,
pinned to `ae395badd950f5893c90cdb82ef5a89927a85705`. Their committed-source validators
passed; those validations do not rerun or qualify the new reader. The records stay
at their exact historical pins and are explicitly not current-source acceptance.
This inventory excludes transitive, unnamed and 1,602 line-span bindings. It is not
a whole-repository freshness or broader gate claim. Earlier planning/privacy and
model evidence remains subject to its recorded source and scope limitations.
