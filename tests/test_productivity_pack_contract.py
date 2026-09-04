import json,unittest
from scripts.productivity_pack_contract import CASE_TYPES,INVENTORY,MATRIX,PACKS,STATES,build
class ProductivityPackContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_support_and_inventory_contract(self):self.assertEqual(tuple(self.value["states"]),STATES);self.assertEqual(tuple(self.value["packs"]),PACKS);self.assertEqual(tuple(self.value["case_types"]),CASE_TYPES);self.assertEqual(tuple(self.value["inventory_fields"]),INVENTORY)
 def test_finance_and_cloud_prohibitions_are_absent(self):self.assertNotIn("money_movement",MATRIX["finance"]["operations"]);self.assertNotIn("cloud_mutation",MATRIX["cloud_observer"]["operations"]);self.assertIn("communication_write",MATRIX["communications"]["operations"])
 def test_invalid_or_inactive_cases_create_no_registration(self):self.assertTrue(all((case["case_type"]=="valid"and case["state"]in {"supported","degraded"})or(not case["admitted"]and not case["registered_operations"])for case in self.value["cases"]))
 def test_absent_and_disabled_are_authority_equivalent(self):self.assertEqual(self.value["absent_authority"],self.value["disabled_authority"]);self.assertTrue(self.value["strict_local_equivalent"])
if __name__=="__main__":unittest.main()
