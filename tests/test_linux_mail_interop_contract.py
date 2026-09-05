import json,unittest
from scripts.linux_mail_interop_contract import ATTACKS,FAILURES,PROFILES,STATES,build
class LinuxMailInteropTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["attacks"]),ATTACKS);self.assertEqual(tuple(self.v["failures"]),FAILURES);self.assertEqual(tuple(self.v["states"]),STATES);self.assertEqual(self.v["case_count"],4500)
 def test_profiles_are_never_mutated(self):self.assertTrue(all(not c["source_mutation"] for c in self.v["cases"]))
 def test_private_inputs_never_extract_credentials(self):self.assertTrue(all(not c["credential_extraction"]for c in self.v["cases"]if c["attack"]!="none"))
 def test_no_escape_missing_provenance_or_residual_authority(self):self.assertEqual(sum(self.v[k]for k in ("path_escape_count","missing_provenance_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
