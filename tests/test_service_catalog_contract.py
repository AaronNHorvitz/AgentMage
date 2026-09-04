import json,unittest
from scripts.service_catalog_contract import ATTACKS,EXTENSIONS,FAULTS,KINDS,LINKS,STATES,build
class ServiceCatalogContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_entities(self):self.assertEqual(tuple(self.value["kinds"]),KINDS);self.assertEqual(tuple(self.value["links"]),LINKS);self.assertEqual(tuple(self.value["extensions"]),EXTENSIONS);self.assertEqual(tuple(self.value["states"]),STATES);self.assertTrue(all(c["provider_resolved_independently"]and c["unresolved_visible"]for c in self.value["entity_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["identity_guess_count"]and not c["authority_grant_count"]and not c["registration_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["drift_visible"]and c["prior_evidence_preserved"]and not c["automatic_rebind_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_provider_count"],0)
if __name__=="__main__":unittest.main()
