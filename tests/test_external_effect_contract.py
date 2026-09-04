import json,unittest
from scripts.external_effect_contract import CRASH_POINTS,EVENT_ATTACKS,OUTCOMES,PLAN_FIELDS,build
class ExternalEffectContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_plan_mutations(self):self.assertEqual(tuple(self.value["plan_fields"]),PLAN_FIELDS);self.assertEqual(self.value["schema_mutation_count"],len(PLAN_FIELDS)*5);self.assertTrue(all(c["request_count"]==0 and c["receipt_count"]==1 for c in self.value["schema_mutations"]))
 def test_fault_campaign_has_no_duplicate_or_unsafe_retry(self):self.assertEqual(tuple(self.value["crash_points"]),CRASH_POINTS);self.assertEqual(tuple(self.value["effect_outcomes"]),OUTCOMES);self.assertGreaterEqual(self.value["fault_schedule_count"],1000);self.assertTrue(all(not c["duplicate_effect_count"] and not c["unsafe_retry_count"] for c in self.value["fault_schedules"]))
 def test_events_carry_no_authority(self):self.assertEqual(tuple(self.value["event_attacks"]),EVENT_ATTACKS);self.assertTrue(all(not c["authority_count"] and not c["follow_on_operation_count"] for c in self.value["event_cases"]))
 def test_stale_compensation_preserves_later_work(self):self.assertTrue(all(not c["stale_compensation_executed"] and c["later_work_preserved"] and c["fresh_plan_required"] for c in self.value["rollback_races"]));self.assertEqual(self.value["live_provider_count"],0);self.assertEqual(self.value["independent_review_count"],0)
if __name__=="__main__":unittest.main()
