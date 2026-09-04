import json,unittest
from scripts.artifact_promotion_contract import ATTACKS,PROVIDERS,READS,build
class ArtifactPromotionContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_reads_bind_immutable_identity(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["reads"]),READS);self.assertTrue(all(c["digest_bound"] and c["mapping_time_bound"] and not c["provider_request_count"] for c in self.value["read_cases"]))
 def test_transfers_are_verified_and_inert(self):self.assertTrue(all(c["digest_verified"] and c["hash_verified"] and c["size_verified"] and c["media_verified"] and c["source_verified"] and not c["corrupt_promotion_count"] for c in self.value["transfer_cases"]))
 def test_attacks_fail_closed(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["mutable_label_authority_count"] and not c["corrupt_promotion_count"] and not c["unsafe_retry_count"] and not c["residual_partial_count"] for c in self.value["attack_cases"]));self.assertEqual(self.value["live_provider_count"],0)
if __name__=="__main__":unittest.main()
