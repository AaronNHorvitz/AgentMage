#!/usr/bin/env python3
"""Validate Sprint 70 temporary connector contracts."""
from __future__ import annotations
import json
from pathlib import Path
from typing import Final
ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS: Final = ROOT / "docs/verification/sprint-70-connector-corpus.json"
DEPENDENCIES: Final = ROOT / "docs/verification/sprint-70-connector-dependency-manifest.json"
GROUPS: Final = ("positive_cases", "prohibited_cases", "invalid_cases", "boundary_cases", "hostile_cases")
COUNTS: Final = {"positive_cases":12,"prohibited_cases":12,"invalid_cases":14,"boundary_cases":10,"hostile_cases":12}

def validate() -> list[str]:
    """Return deterministic failures."""
    failures=[]; corpus=json.loads(CORPUS.read_text()); dependencies=json.loads(DEPENDENCIES.read_text()); seen=set()
    for group in GROUPS:
        values=corpus.get(group,[])
        if len(values)!=COUNTS[group] or len(values)!=len(set(values)): failures.append(f"{group} count or uniqueness drifted")
        if seen.intersection(values): failures.append("case overlap")
        seen.update(values)
    for field in ("network_clients","dns_resolvers","proxy_clients","credential_values","connector_accounts","cache_writers","background_workers"):
        if dependencies.get(field)!=[]: failures.append(f"unadmitted dependency: {field}")
    source=(ROOT/"capabilities/knowledge/src/connected_connector.rs").read_text()
    for fragment in ("pub fn prepare_network_grant(","pub fn activate_connected_profile(","pub fn record_connector_response(","pub fn finish_connected_operation(","strict_local_baseline_restored: true","fresh_grant_required: true","policy_authority: false","completion_authority: false"):
        if fragment not in source: failures.append(f"boundary absent: {fragment}")
    for forbidden in ("reqwest","TcpStream","UdpSocket","HTTP_PROXY","Authorization: Bearer"):
        if forbidden in source: failures.append(f"network implementation admitted: {forbidden}")
    return failures

if __name__=="__main__":
    problems=validate()
    if problems:
        for problem in problems: print(f"error: {problem}")
        raise SystemExit(1)
    print(f"validated {sum(COUNTS.values())} Sprint 70 connector cases")
