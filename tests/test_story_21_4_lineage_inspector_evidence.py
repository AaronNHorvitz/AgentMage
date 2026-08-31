import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "artifacts/sprints/sprint-21/story-21.4/lineage-inspector-report.json"


def test_story_21_4_report_is_source_bound_and_truthful():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "PASS_LOCAL_LINEAGE_INSPECTOR"
    assert report["lineage_fact_categories"] == 18
    assert report["performance_metrics_qualified"] == 17
    truth = report["product_truth"]
    assert truth["ordered_lineage_complete"]
    assert truth["journal_reload_equivalence_complete"]
    assert truth["materialized_view_authority_count"] == 0
    assert truth["raw_secret_or_hidden_reasoning_field_count"] == 0
    assert truth["all_declared_thresholds_fail_closed"]
    assert not truth["full_RV_56_executed"]
    assert not truth["installed_runtime_campaign_executed"]
    assert not truth["supported_platform_campaign_executed"]
    assert not truth["independent_qualification_executed"]
    for artifact in report["artifacts"]:
        path = ROOT / artifact["path"]
        assert path.stat().st_size == artifact["byte_length"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact["sha256"]


def test_story_21_4_retains_distribution_pressure_cursor_and_cancellation_markers():
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    fixture = report["performance_fixture"]
    assert fixture["iterations"] == 100
    assert fixture["lineage_events_per_iteration"] == 26
    assert fixture["events_per_second"] >= 1
    assert fixture["p50_us"] <= fixture["p95_us"] <= fixture["p99_us"]
    markers = report["required_markers"]
    assert any("lagging_subscriber" in marker for marker in markers)
    assert any("cursor_drift" in marker for marker in markers)
    assert any("cancellation_survive" in marker for marker in markers)
    assert any("AGENTMAGE_STORY_21_4_PERFORMANCE" in marker for marker in markers)
