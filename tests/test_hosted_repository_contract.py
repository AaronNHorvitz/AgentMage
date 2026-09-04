import unittest
from scripts.hosted_repository_contract import validate
class HostedRepositoryContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
if __name__=="__main__": unittest.main()
