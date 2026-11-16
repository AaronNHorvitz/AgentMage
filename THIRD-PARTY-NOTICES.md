# Third-party notices and distribution findings

AgentMage's own work is MIT OR Apache-2.0. Dependencies retain their original
licences, copyrights and notices; neither alternative relicenses a dependency.
This account uses the existing Cargo external catalogue, package lock and
[dependency provenance](supply-chain/dependency-provenance.json). No new upstream
code, model or package was downloaded for the check.

The recorded permissive choices (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode,
Unlicense, 0BSD, CDLA-Permissive-2.0, BlueOak-1.0.0 and Python-2.0) permit
redistribution alongside MIT OR Apache-2.0 with their own notice obligations.
For OR expressions choose a permissive alternative, including Apache-2.0 rather
than MPL-2.0 and MIT/Apache rather than the LGPL alternative. Historical slash
notation in six Cargo declarations denotes their recorded MIT/Apache choices;
retain the upstream expression rather than editing third-party records.
AND expressions retain every component's obligations, including BSD/ISC and
Unicode notices. The font/icon package below also requires its asset licences.

## Findings and additional obligations

| Recorded dependency | Scope | Disposition |
| --- | --- | --- |
| `elkjs` 0.9.3, EPL-2.0 | Development, excluded from runtime | Finding: not covered by a permissive-only dependency assertion. Distribution must retain its separate EPL terms and source obligations; it cannot be relabelled MIT OR Apache-2.0. Resolve its packaging disposition before distributing development tools. No licence wording is changed to hide this. |
| `khroma` 2.1.0, NOASSERTION | Development, excluded from runtime | Finding: compatibility cannot be confirmed from the existing records because no licence is declared. Obtain provenance/permission or exclude it from a distributed package before release. NOASSERTION is not approval. |
| `@fortawesome/fontawesome-free` 7.3.1, CC-BY-4.0 AND OFL-1.1 AND MIT | Development, excluded from runtime | Preserve CC attribution, font licence and code notices when shipping these assets/tools; the project licence does not replace them. |
| `dompurify` 3.4.13, MPL-2.0 OR Apache-2.0 | Development, excluded from runtime | Apache-2.0 is the permissive alternative for this account; keep its notices. |
| `argparse` 2.0.1, Python-2.0 | Development, excluded from runtime | Retain its separate licence and notices; it is not project-owned code. |

No mandatory nonpermissive licence or missing declaration was found among the
recorded external Cargo dependencies. The two findings above prevent a blanket
claim that every development dependency is confirmed permissive. This review
covers recorded dependency metadata, not an audit of upstream source contents
or proof that a distributable package already includes every required notice.
Package acceptance remains open; historical evidence is not fresh admission.

## Complete recorded external expression inventory

Every registry component in the existing provenance is counted once here;
individual names, versions, origins and checksums remain in that record.
Repository-owned workspace components use the project licence and are excluded
from this external table. The external package set and expressions are unchanged.

| Upstream licence expression | Components |
| --- | --- |
| `(Apache-2.0 OR MIT) AND BSD-3-Clause` | 1 |
| `(CC-BY-4.0 AND OFL-1.1 AND MIT)` | 1 |
| `(MIT OR Apache-2.0) AND Unicode-3.0` | 1 |
| `(MPL-2.0 OR Apache-2.0)` | 1 |
| `0BSD` | 1 |
| `0BSD OR MIT OR Apache-2.0` | 1 |
| `Apache-2.0` | 29 |
| `Apache-2.0 AND ISC` | 1 |
| `Apache-2.0 OR BSD-3-Clause` | 1 |
| `Apache-2.0 OR ISC OR MIT` | 1 |
| `Apache-2.0 OR MIT` | 12 |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 3 |
| `Apache-2.0/MIT` | 1 |
| `BSD-2-Clause` | 7 |
| `BSD-3-Clause` | 15 |
| `BlueOak-1.0.0` | 1 |
| `CDLA-Permissive-2.0` | 1 |
| `EPL-2.0` | 1 |
| `ISC` | 46 |
| `MIT` | 288 |
| `MIT AND BSD-3-Clause` | 1 |
| `MIT OR Apache-2.0` | 92 |
| `MIT OR Apache-2.0 OR BSD-1-Clause` | 1 |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | 1 |
| `MIT OR Apache-2.0 OR Zlib` | 1 |
| `MIT OR Zlib OR Apache-2.0` | 1 |
| `MIT/Apache-2.0` | 5 |
| `NOASSERTION` | 1 |
| `Python-2.0` | 1 |
| `Unicode-3.0` | 18 |
| `Unlicense` | 1 |
| `Unlicense OR MIT` | 2 |
| `Zlib` | 1 |
| `Zlib OR Apache-2.0 OR MIT` | 1 |
