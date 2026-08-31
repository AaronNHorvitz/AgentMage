import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "artifacts/sprints/sprint-16/story-16.3/tool-composition-report.json"


def test_story_16_3_report_is_source_bound_and_truthful():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "PASS_LOCAL_DETERMINISTIC_COMPOSITION"
    assert len(report["preflight_families"]) == 9
    assert len(report["terminal_states"]) == 8
    truth = report["product_truth"]
    assert truth["read_only_native_mapping_count"] == 17
    assert truth["coding_native_mapping_count"] == 21
    assert truth["duplicate_effect_count"] == 0
    assert truth["false_completion_count"] == 0
    assert not truth["installed_worker_campaign_executed"]
    for artifact in report["artifacts"]:
        path = ROOT / artifact["path"]
        assert path.stat().st_size == artifact["byte_length"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]


def test_story_16_3_retains_all_hostile_and_recovery_markers():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    markers = report["required_markers"]
    assert len(markers) == 10
    assert any("preflight_drift" in marker for marker in markers)
    assert any("crash_boundaries" in marker for marker in markers)
    assert any("duplicate_calls" in marker for marker in markers)
