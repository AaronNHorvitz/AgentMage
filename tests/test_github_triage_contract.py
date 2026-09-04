import unittest
from scripts.github_triage_contract import validate
class GithubTriageContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
if __name__=="__main__": unittest.main()
