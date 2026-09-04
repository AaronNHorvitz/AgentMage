"""Sprint 71 provider artifact tests."""
import unittest
from scripts.github_provider_contract import validate
class GithubProviderContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
if __name__=="__main__": unittest.main()
