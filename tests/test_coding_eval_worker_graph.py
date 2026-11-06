"""Finite worker codec assertions without frozen held-out test imports."""
from types import SimpleNamespace
import unittest
from unittest import mock

from scripts import coding_eval_worker_graph as g


class Peer:
    def __init__(self,prefix):
        self.failed=False
        self.graph=g.Graph(self,prefix,__file__ if prefix=='t' else None)
        self.other=None

    def load(self,frame):return self.graph.load(frame)
    def snapshot(self,value):return self.graph.snapshot(value)

    def request(self,operation,target,args,kwargs):
        decoded=self.other.graph.load(self.graph.snapshot((args,kwargs)))
        result=g.trusted_dispatch(self.other,operation,target,*decoded)
        return self.graph.load(self.other.graph.snapshot(result,copy_values=True))


class CodecTests(unittest.TestCase):
    def pair(self):
        parent,child=Peer('t'),Peer('c')
        parent.other=child;child.other=parent
        return parent,child

    def test_new_closed_return_values_copy_but_known_roots_and_aliases_share_identity(self):
        parent,child=self.pair()
        value={'a':[False,0,b'fixture']}
        frame=parent.graph.snapshot(value,copy_values=True)
        self.assertEqual(frame['nodes'],[])
        decoded=child.graph.load(frame)
        self.assertEqual(decoded,value)
        self.assertIs(type(decoded['a'][0]),bool)
        shared=[];value={'a':shared,'b':shared}
        frame=parent.graph.snapshot(value,copy_values=True)
        decoded=child.graph.load(frame)
        self.assertIs(decoded['a'],decoded['b'])
        shared.append('changed')
        updated=child.graph.load(parent.graph.snapshot(value))
        self.assertIs(updated,decoded)
        self.assertIs(updated['a'],updated['b'])
        self.assertEqual(updated['a'],['changed'])

    def test_fixture_class_and_instances_keep_type_relationships_and_constructor_keywords(self):
        parent,child=self.pair()
        class Factory:
            def __init__(self,*,_identifier):self.value=_identifier
            def read(self):return self.value
        Factory.__module__='hidden_eval'
        proxy=child.graph.load(parent.graph.snapshot(Factory))
        self.assertIs(type(proxy),type)
        instance=proxy(_identifier='ordinary input')
        self.assertIs(type(instance),proxy)
        self.assertIsInstance(instance,proxy)
        self.assertEqual(instance.read(),'ordinary input')
        actual=parent.graph.load(child.graph.snapshot(instance))
        self.assertIs(type(actual),Factory)
        self.assertEqual(actual.read(),'ordinary input')
        self.assertNotIn('CodecTests',proxy.__qualname__)

    def test_callback_reserved_looking_keyword_is_data(self):
        parent,child=self.pair()
        def callback(*,_identifier):return _identifier
        proxy=child.graph.load(parent.graph.snapshot(callback))
        self.assertEqual(proxy(_identifier='ordinary input'),'ordinary input')

    def test_namespace_invalid_names_reach_original_validation_as_data(self):
        parent,child=self.pair()
        value=SimpleNamespace(**{'__class__':'invalid-name','valid':False})
        decoded=child.graph.load(parent.graph.snapshot(value))
        self.assertIs(type(decoded),SimpleNamespace)
        self.assertEqual(vars(decoded),vars(value))
        self.assertEqual(vars(decoded)['__class__'],'invalid-name')

    def test_builtin_type_literals_and_list_derived_invalid_inputs_keep_categories(self):
        parent,child=self.pair()
        for value in (object,type,bool,int,float,str,bytes,list,tuple,dict,set,frozenset):
            self.assertIs(child.graph.load(parent.graph.snapshot(value)),value)
        class Derived(list):
            def __iter__(self):raise AssertionError('custom iteration must not execute')
            def __eq__(self,other):raise AssertionError('custom equality must not execute')
        value=Derived([False,3])
        decoded=child.graph.load(parent.graph.snapshot([value,value]))
        self.assertIsInstance(decoded[0],list)
        self.assertIsNot(type(decoded[0]),list)
        self.assertIs(decoded[0],decoded[1])
        self.assertEqual(list(decoded[0]),[False,3])
        decoded[0].append('changed')
        returned=parent.graph.load(child.graph.snapshot(decoded[0]))
        self.assertIs(returned,value)
        self.assertEqual(list.__getitem__(value,2),'changed')

    def test_plain_fixture_exception_identity_survives_and_custom_constructors_refuse(self):
        parent,child=self.pair()
        constructed=[]
        class FixtureError(RuntimeError):
            def __init__(self,*args):
                constructed.append(args)
                super().__init__(*args)
        FixtureError.__module__='hidden_eval'
        value=FixtureError('synthetic')
        with self.assertRaisesRegex(g.BoundaryError,'fixture-exception-constructor'):
            parent.graph.snapshot(value)
        self.assertEqual(constructed,[('synthetic',)])

        class PlainError(RuntimeError):pass
        PlainError.__module__='hidden_eval'
        value=PlainError('synthetic')
        decoded=child.graph.load(parent.graph.snapshot(value))
        self.assertIsInstance(decoded,RuntimeError)
        returned=parent.graph.load(child.graph.snapshot(decoded))
        self.assertIs(type(returned),PlainError)
        self.assertEqual(returned.args,('synthetic',))
        self.assertEqual(constructed,[('synthetic',)])

    def test_inert_non_string_mapping_keys_reach_subject_validation(self):
        parent,child=self.pair()
        sentinel=object()
        value={sentinel:'object-key',object:'type-key',None:'none-key'}
        decoded=child.graph.load(parent.graph.snapshot(value))
        returned=parent.graph.load(child.graph.snapshot(decoded))
        self.assertIs(returned,value)
        self.assertEqual(returned[sentinel],'object-key')
        self.assertEqual(returned[object],'type-key')

    def test_invalid_json_literals_and_native_exception_fields_survive_transport(self):
        parent,child=self.pair()
        for value in (3+7j,print,10**10000,-10**10000):
            frame=parent.graph.snapshot(value)
            frame=g.receive(__import__('io').BytesIO(g.encode(frame)))
            result=child.graph.load(frame)
            self.assertIs(type(result),type(value))
            self.assertEqual(result,value)
        for value,field in ((SystemExit(2),'code'),(StopIteration('synthetic'),'value')):
            result=child.graph.load(parent.graph.snapshot(value))
            self.assertIs(type(result),type(value))
            self.assertEqual(getattr(result,field),getattr(value,field))

    def test_equal_valued_boolean_and_integer_mutations_never_disappear_in_delta_cache(self):
        parent,child=self.pair()
        root={'field':1}
        received=child.graph.load(parent.graph.snapshot(root))
        for value in (True,1,False,0):
            root['field']=value
            child.graph.load(parent.graph.snapshot(root))
            self.assertIs(type(received['field']),type(value))

    def test_mock_public_data_and_call_tuple_arities_preserved_without_overriding_assertions(self):
        parent,child=self.pair()
        original=mock.MagicMock(stdout='fixture',returncode=0)
        original('argument',cache_prompt=False)
        decoded=child.graph.load(parent.graph.snapshot(original))
        self.assertIsInstance(decoded,mock.MagicMock)
        self.assertEqual(decoded.stdout,'fixture')
        self.assertEqual(decoded.returncode,0)
        self.assertEqual(len(tuple(decoded.call_args)),2)
        self.assertEqual(len(tuple(decoded.mock_calls[0])),3)
        decoded.assert_called_once_with('argument',cache_prompt=False)
        decoded.assert_called_once=lambda:None
        with self.assertRaises(g.BoundaryError):child.graph.snapshot(decoded)

    def test_verdict_objects_and_forged_exception_hierarchies_refuse(self):
        parent,child=self.pair()
        for value in (unittest.TestCase(),unittest.TestResult(),unittest.TestCase,unittest.TestResult):
            with self.subTest(value=value),self.assertRaises(g.BoundaryError):parent.graph.token(value)
        class ProtocolError(RuntimeError):pass
        child.graph.exceptions['ProtocolError']=ProtocolError
        class Expected(ValueError):pass
        parent.graph.exceptions['ProtocolError']=Expected
        with self.assertRaises(g.BoundaryError):parent.graph.load(child.graph.snapshot(ProtocolError))


if __name__=='__main__':unittest.main()
