import json,unittest
from scripts.windows_runtime_boundary import AMBIENT_ATTACKS,FAULTS,PATH_ATTACKS,STATE_ATTACKS,build
class WindowsRuntimeBoundaryTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_path_and_sandbox_minima(self):self.assertGreaterEqual(len(self.value["path_cases"]),1000);self.assertGreaterEqual(len(self.value["ambient_cases"]),500);self.assertEqual(tuple(self.value["path_attacks"]),PATH_ATTACKS);self.assertEqual(tuple(self.value["ambient_attacks"]),AMBIENT_ATTACKS)
 def test_state_and_recovery_are_closed(self):self.assertEqual(tuple(self.value["state_attacks"]),STATE_ATTACKS);self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["fail_closed"]and not c["authority_leak_count"]for c in self.value["state_cases"]));self.assertTrue(all(c["cleanup_complete"]and not c["duplicate_completed_operation_count"]for c in self.value["fault_cases"]))
 def test_native_claims_remain_zero(self):self.assertEqual(self.value["native_guest_execution_count"],0);self.assertEqual(self.value["observed_network_minutes"],0);self.assertEqual(self.value["promoted_platform_count"],0)
if __name__=="__main__":unittest.main()
