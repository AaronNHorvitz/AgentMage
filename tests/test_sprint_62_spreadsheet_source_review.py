from __future__ import annotations

import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/sprint_62_spreadsheet_source_review.py"
SPEC = importlib.util.spec_from_file_location("spreadsheet_review", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_current_revision_passes_every_review_check() -> None:
    value = MODULE.expected("HEAD")
    assert value["status"] == "PASS_LOCAL_SPREADSHEET_SOURCE_REVIEW"
    assert all(value["checks"].values())


def test_validation_rejects_a_removed_bound() -> None:
    value = MODULE.expected("HEAD")
    value["checks"]["parser_and_projection_resource_ceilings_fail_closed"] = False
    assert MODULE.validate(value) == ["review is stale, incomplete, or widened"]


def test_validation_rejects_widened_limitations() -> None:
    value = MODULE.expected("HEAD")
    value["limitations"] = []
    assert MODULE.validate(value) == ["review is stale, incomplete, or widened"]
