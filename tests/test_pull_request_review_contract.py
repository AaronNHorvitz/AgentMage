import unittest
from scripts.pull_request_review_contract import validate
class PullRequestReviewContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
if __name__=="__main__": unittest.main()
