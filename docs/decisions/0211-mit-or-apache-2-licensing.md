# Decision 0211: MIT OR Apache-2.0 from this commit forward

| Field | Value |
| --- | --- |
| Status | Accepted under explicit owner instruction |
| Date | 2026-11-16 |
| Authority | Owner licence choice, 2026-11-16 14:25 |
| Supersedes | Decision 0051 prospectively; licence part of the 14:20 README order |
| Scope | Project-owned work, manifests, headers, notices and contribution terms |

From the commit introducing this decision, the project's own work is licensed
`MIT OR Apache-2.0`, at the recipient's choice. LICENSE-MIT and LICENSE-APACHE
contain the standard texts with `Copyright 2026 Aaron N. Horvitz`. LICENSE
contains both complete alternatives so existing package paths carry the actual
terms, and the contract crate carries that same combined text. No new version
is governed by Business Source License 1.1. The README may call this open source
and carries the owner's line about a harness for the world to use and improve.

Versions published before 2026-09-07 were under Apache License 2.0. Versions
published from 2026-09-07 until this relicensing commit were under Business
Source License 1.1. Those published versions keep the licence they were
published with. Preserve Decision 0051 unchanged, along with historical licence
statements in the now-historical inventory, old decisions and evidence. Do not
rewrite history or imply a withdrawal of earlier grants.

All Cargo packages resolve to the chosen SPDX expression, including the excluded
experimental crate. The root and editor npm manifests and their workspace lock
entries agree. Package spec templates, source SPDX headers, package validation
and first-party supply-chain metadata agree. Third-party expressions and old
hash-bound evidence retain their original meaning; no historical artifact is
relabelled as fresh verification. Batch source changes before regenerating the
full-tree supply-chain outputs once. Preserve their hash binding.

The licence validator checks complete pinned texts, every manifest, applicable
headers and notices; negative controls reject drift. This supersedes obsolete
validators that required the Business Source text. README checks now enforce the
engine account and current safety truth rather than requiring obsolete product
names and desktop claims. Other historical document checks remain in place;
lifecycle and verification values are not promoted.

Existing dependency records are reviewed in THIRD-PARTY-NOTICES.md. The missing
khroma declaration and elkjs's separate EPL obligations are findings, not reasons
to change the chosen project licence or claim all dependencies are permissive.
Third-party attribution and packaging obligations remain with those components.
No public release or package admission is authorized by this decision.

CONTRIBUTING.md accepts contributions under both alternatives, requires every
commit to carry a Signed-off-by line, and states that the licences grant no
right to the project's name. Existing independent core review remains required.
This licence order is combined with Decision 0210's README batch and reviewed
at Tier: core. MEASURE FIRST resumes after this ordered batch; no evaluation
input, threshold, hidden test or model qualification is changed by assertion.

## Standard text sources

The Apache text is the existing standard contract-crate licence, checked against
[the Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0.txt); only the
application copyright placeholder is filled. The MIT text follows
[the MIT License](https://opensource.org/license/mit), with the owner's copyright
and plain ASCII quotation marks. These are licence texts, not imported program
code or product design.
