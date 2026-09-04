import unittest
from scripts import github_mutation_contract as contract
class GithubMutationContractTests(unittest.TestCase):
 def test_current(self): self.assertEqual(contract.validate(),[])
 def test_missing(self):
  original=contract.REQUIRED
  try:
   contract.REQUIRED=(*original,"MissingGithubMutationBoundary"); self.assertIn("GitHub mutation boundary absent: MissingGithubMutationBoundary",contract.validate())
  finally: contract.REQUIRED=original
if __name__=="__main__": unittest.main()
