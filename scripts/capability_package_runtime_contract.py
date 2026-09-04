#!/usr/bin/env python3
"""Validate Sprint 79 package hooks, catalog, recovery, and inventory contracts."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/"shells/host/src/capability_package_runtime.rs"
CORPUS=ROOT/"docs/verification/sprint-79-package-runtime-corpus.json"
REQUIRED=("PackageHookPhase","PackageHookOutcome","safe_mode_required","failure_isolated","transaction_unchanged","all_receipts_present","PackageCatalogEntry","activation_reason","assess_package_compatibility","PackageSafetyPolicy","recover_package_lifecycle","verify_inventory_delta","declared_only: true")
FORBIDDEN=("std::fs","std::process","TcpStream","reqwest","Command::new","public_auto_discovery: true","automatic_download: true","automatic_enable: true","unsigned_execution: true")
EXPECTED={"hooks":12,"catalog":8,"compatibility":8,"policy":8,"recovery":6,"inventory":6}
def validate():
    failures=[];source=SOURCE.read_text().split("#[cfg(test)]",1)[0];corpus=json.loads(CORPUS.read_text())
    for token in REQUIRED:
        if token not in source:failures.append(f"required package runtime boundary absent: {token}")
    for token in FORBIDDEN:
        if token in source:failures.append(f"package runtime executor admitted: {token}")
    cases=corpus.get("cases",[])
    if corpus.get("case_count")!=48 or len(cases)!=48 or len(set(cases))!=48:failures.append("package runtime corpus count drifted")
    if corpus.get("categories")!=EXPECTED or sum(EXPECTED.values())!=48:failures.append("package runtime categories drifted")
    if corpus.get("local_contract_passed") is not True or corpus.get("native_lifecycle_complete") is not False or corpus.get("safe_mode_startup_observed") is not False or corpus.get("independent_review_present") is not False or corpus.get("installed_package_count")!=0 or corpus.get("enabled_package_count")!=0 or corpus.get("substitution_set")!=[]:failures.append("package runtime truth state drifted")
    return failures
if __name__=="__main__":
    failures=validate()
    if failures:print("\n".join(failures));raise SystemExit(1)
    print("validated 48 package hook, catalog, recovery, and inventory cases")
