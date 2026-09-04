#!/usr/bin/env python3
"""Validate Sprint 78's fail-closed capability package contract."""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/"shells/host/src/capability_package.rs"
CORPUS=ROOT/"docs/verification/sprint-78-capability-package-corpus.json"
REQUIRED=("CapabilityPackageManifest","CapabilityCompatibility","CapabilityPackageScope","CapabilityDependency","CapabilityEntryPoint","CapabilitySideEffect","CapabilityLifecycleAction","Install","Enable","Disable","Update","Rollback","Uninstall","verify_strict","narrow_package_scope","applied: false")
FORBIDDEN=("std::fs","std::process","TcpStream","reqwest","Command::new","applied: true")
EXPECTED={"manifest":14,"verification":10,"lifecycle":8,"scope":12}
def validate():
    failures=[]; source=SOURCE.read_text().split("#[cfg(test)]",1)[0]; corpus=json.loads(CORPUS.read_text())
    for token in REQUIRED:
        if token not in source: failures.append(f"required package boundary absent: {token}")
    for token in FORBIDDEN:
        if token in source: failures.append(f"package executor admitted: {token}")
    cases=corpus.get("cases",[])
    if corpus.get("case_count")!=44 or len(cases)!=44 or len(set(cases))!=44: failures.append("package corpus count drifted")
    if corpus.get("categories")!=EXPECTED or sum(EXPECTED.values())!=44: failures.append("package corpus categories drifted")
    if corpus.get("local_contract_passed") is not True or corpus.get("installed_package_count")!=0 or corpus.get("enabled_package_count")!=0 or corpus.get("native_lifecycle_complete") is not False or corpus.get("independent_review_present") is not False or corpus.get("substitution_set")!=[]: failures.append("package truth state drifted")
    return failures
if __name__=="__main__":
    failures=validate()
    if failures: print("\n".join(failures)); raise SystemExit(1)
    print("validated 44 capability package trust and lifecycle cases")
