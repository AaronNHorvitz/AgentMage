import copy,json,unittest
from pathlib import Path
from unittest.mock import patch
from scripts.v07_read_only_contract import validate
class V07ReadOnlyContractTests(unittest.TestCase):
    def test_contract(self): self.assertEqual(validate(),[])
    def test_bundle_truth_is_closed(self):
        source=json.loads(Path("docs/verification/sprint-75-v0.7-acceptance-bundle.json").read_text())
        original_read_text=Path.read_text
        for mutate in (lambda v:v.update({"release_status":"PASS"}),lambda v:v.update({"native_acceptance_complete":True}),lambda v:v.update({"hosted_mutation_operation_count":1}),lambda v:v.update({"substitution_set":["local"]})):
            value=copy.deepcopy(source); mutate(value)
            def read_text(path,*args,**kwargs):
                if "acceptance-bundle" in str(path): return json.dumps(value)
                return original_read_text(path,*args,**kwargs)
            with patch("pathlib.Path.read_text",read_text): self.assertTrue(validate())
if __name__=="__main__": unittest.main()
