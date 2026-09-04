import json,unittest
from scripts.adapter_removal_contract import ACTIONS,ATTACKS,INVENTORY,STAGES,STATES,build
class AdapterRemovalContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_dimensions(self):self.assertEqual(tuple(self.value["inventory"]),INVENTORY);self.assertEqual(tuple(self.value["actions"]),ACTIONS);self.assertEqual(tuple(self.value["stages"]),STAGES);self.assertEqual(tuple(self.value["states"]),STATES);self.assertEqual(tuple(self.value["attacks"]),ATTACKS)
 def test_removal_and_recovery_are_safe(self):self.assertTrue(all(not c["neighbor_mutation_count"]for c in self.value["plan_cases"]));self.assertTrue(all(not c["admitted_count"]and not c["network_attempt_count"]for c in self.value["attack_cases"]));self.assertTrue(all(not c["partial_authority_count"]and not c["unrelated_damage_count"]for c in self.value["recovery_cases"]))
 def test_native_claims_are_zero(self):self.assertEqual(self.value["native_platform_count"],0);self.assertEqual(self.value["strict_local_rerun_count"],0);self.assertEqual(self.value["zero_egress_minutes"],0);self.assertEqual(self.value["promotion_count"],0)
if __name__=="__main__":unittest.main()
