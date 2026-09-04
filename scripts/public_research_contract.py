#!/usr/bin/env python3
"""Validate Sprint 82 public-research and citation contracts."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "shells/host/src/public_research.rs"
CORPUS = ROOT / "docs/verification/sprint-82-public-research-corpus.json"
GUIDE = ROOT / "docs/guides/public-research.md"
REQUIRED = (
    "PublicSourceType", "PrimaryDocumentation", "OriginalResearch",
    "AuthoritativeRecord", "SecondaryAnalysis", "PublicSearchRequest",
    "PublicSearchCandidate", "PublicCitation", "prepare_public_search",
    "verify_public_results", "max_results", "max_total_bytes", "recency_days",
    "excerpt_sha256", "quotation_word_limit", "inference_label_required", "fresh",
    'strip_prefix("https://")',
)
FORBIDDEN = (
    "std::net", "std::process", "std::fs", "TcpStream", "Command::new", "reqwest::",
    "credential", "cookie", "download(", "publish(",
)
EXPECTED = {"query": 5, "domain": 5, "source": 5, "citation": 5, "limit": 5, "authority": 5}


def validate() -> list[str]:
    failures: list[str] = []
    source = SOURCE.read_text(encoding="utf-8")
    production = source.split("#[cfg(test)]", 1)[0]
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))
    guide = GUIDE.read_text(encoding="utf-8")
    for token in REQUIRED:
        if token not in production:
            failures.append(f"public research boundary absent: {token}")
    for token in FORBIDDEN:
        if token in production:
            failures.append(f"public research contract acquired authority: {token}")
    cases = corpus.get("cases", [])
    if corpus.get("case_count") != 30 or len(cases) != 30 or len(set(cases)) != 30:
        failures.append("public research corpus count drifted")
    if corpus.get("categories") != EXPECTED or sum(EXPECTED.values()) != 30:
        failures.append("public research corpus categories drifted")
    if (
        corpus.get("local_contract_passed") is not True
        or corpus.get("live_public_search_executed") is not False
        or corpus.get("browser_campaign_complete") is not False
        or corpus.get("privacy_review_present") is not False
        or corpus.get("substitution_set") != []
    ):
        failures.append("public research corpus truth state drifted")
    if "25-word quotation" not in guide or "Inferences remain visibly labeled" not in guide:
        failures.append("public research disclosure guidance drifted")
    if source.count("fn sprint_82_") != 4:
        failures.append("Sprint 82 focused test inventory drifted")
    return failures


if __name__ == "__main__":
    errors = validate()
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print("validated 30 public-research and citation cases")
