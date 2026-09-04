import json,unittest
from scripts.cross_interface_assurance_contract import ATTACKS,BOUNDARIES,ISOLATION,build
class CrossInterfaceAssuranceContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_boundary_attack_and_isolation_sets(self):self.assertEqual(tuple(self.value["boundaries"]),BOUNDARIES);self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertEqual(tuple(self.value["isolation_dimensions"]),ISOLATION)
 def test_full_cross_product_and_no_effects(self):self.assertEqual(self.value["case_count"],len(BOUNDARIES)*len(ATTACKS));self.assertEqual({(c["boundary"],c["attack"]) for c in self.value["cases"]},{(b,a) for b in BOUNDARIES for a in ATTACKS});self.assertTrue(all(c["decision"]=="deny" and not c["unauthorized_effect_count"] and not c["network_count"] and c["receipt_count"]==1 for c in self.value["cases"]))
 def test_native_claim_absent(self):self.assertEqual(self.value["native_interface_count"],0)
if __name__=="__main__":unittest.main()
