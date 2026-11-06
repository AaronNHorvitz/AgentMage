"""Complete frozen methods in a sealed independent worker oracle."""
import importlib.util
import json
import os
from pathlib import Path
import selectors
import sys
import unittest


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


graph = load('graph','/adapter/graph.py')
sealed = load('sealed','/adapter/sealed.py')
facade_module = load('facade','/adapter/facade.py')


class ProtocolError(ValueError):pass
class AdapterPolicyError(ValueError):pass
class CandidateExecutionError(RuntimeError):pass


ERRORS = {name: globals()[name] for name in graph.API_EXCEPTIONS}


class Manager:
    def __init__(self, boundary, transcript):
        self.boundary,self.transcript=boundary,transcript
        self.facade=facade_module.Facade(None,'/candidate/scripts/coding_eval_oracle_worker.py')
        self.process=None
        self.peer=None
        self.records=[]
        self.calls=0
        self.failed=False
        self.ready=False
        self.found=None
        self.fixture_assertions=[]

    def start(self, name):
        if self.process is not None:raise RuntimeError('runtime.overlapping-methods')
        self.active_name=name
        self.process=self.boundary.launch_bridge()
        self.peer=graph.Peer(self.process.stdout,self.process.stdin,'t','/adapter/test.py',self.transcript)
        self.peer.graph.exceptions.update(ERRORS)
        self.peer.dispatch=lambda *args:graph.trusted_dispatch(self.peer,*args)
        self.facade.__dict__['_bridge']=self.peer
        if self.peer.read()!={'event':'ready'}:raise graph.BoundaryError('runtime.startup')
        self.ready=True
        self.found=callable(self.peer.request('get-subject','RemoteFunction',(),{}))

    def finish(self):
        process,self.process=self.process,None
        if process is None:return
        peer,self.peer=self.peer,None
        # A local reference keeps callback dispatch stable during shutdown.
        self.peer=peer
        clean=False
        try:
            peer.send({'event':'stop'})
            with selectors.DefaultSelector() as events:
                events.register(process.stdout,selectors.EVENT_READ)
                if events.select(3):
                    clean=process.stdout.read(1)==b'' and process.wait(timeout=3)==0
        except (OSError,ValueError,TimeoutError):
            clean=False
        finally:
            if not clean:self.boundary.stop_bridge(process)
            else:
                process.stdin.close();process.stdout.close()
            self.calls+=peer.sequence
            self.failed|=peer.failed or not clean
            self.fixture_assertions.extend(peer.fixture_assertions)
            self.records.append({'method':self.active_name,'calls':peer.sequence,'events':peer.events,
                                 'bytes':peer.bytes,'clean_exit':clean,'boundary_failed':peer.failed,
                                 'boundary_errors':peer.boundary_errors,
                                 'fixture_assertions':len(peer.fixture_assertions)})
            self.peer=None


def main():
    config=json.loads(Path('/adapter/config.json').read_bytes())
    if (config.keys()!={'test_names','expected_tests','source'}
            or config['source']!='scripts/coding_eval_oracle_worker.py'):
        raise ValueError('runtime.config')
    index=0 if sys.argv[1:]==['oracle'] else int(sys.argv[1])
    if (type(config['test_names']) is not list or not 0<=index<len(config['test_names'])
            or len(config['test_names'])!=config['expected_tests']):
        raise ValueError('runtime.method-selection')
    selected=config['test_names'][index]
    prefix=f'method-{index:03d}'
    spec=importlib.util.spec_from_file_location('hidden_eval','/adapter/test.py')
    hidden=importlib.util.module_from_spec(spec)
    code=spec.loader.get_code('hidden_eval')
    boundary=sealed.SealedOracle()
    with (Path('/output/'+prefix+'-tests.json').open('xb') as summary,
          Path('/output/'+prefix+'-wire.log').open('xb') as transcript,
          Path('/output/'+prefix+'-details.json').open('xb') as details):
        boundary.install_launch_boundary()
        manager=Manager(boundary,transcript)
        sys.path.insert(0,'/workspace')
        import scripts
        sys.modules['scripts.coding_eval_oracle_worker']=manager.facade
        scripts.coding_eval_oracle_worker=manager.facade
        boundary.close_private_paths()
        graph.enable_sealed_files(boundary)
        sys.addaudithook(graph.audit)

        class Result(unittest.TextTestResult):
            def printErrors(self):
                # Original complete exception reports remain in the private
                # details descriptor. Compact diagnostics respect log bounds.
                self.stream.writeln('Original exception details retained separately.')

        try:
            manager.start(selected)
            # Execute the complete unchanged module after sealing. Even
            # module-level API aliases use this same method-owned bridge.
            exec(code,hidden.__dict__)
            suite=unittest.defaultTestLoader.loadTestsFromName(selected,hidden)
            result=unittest.TextTestRunner(stream=sys.stderr,verbosity=0,resultclass=Result).run(suite)
        finally:
            manager.finish()
        counts={'tests':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),
                'skipped':len(result.skipped),'expected_failures':len(result.expectedFailures),
                'unexpected_successes':len(result.unexpectedSuccesses)}
        record={'tests':counts,'calls':manager.calls,'protocol_failed':manager.failed,
                'adapter_policy_failed':False,'candidate_exception_failed':False,
                'candidate_ready':manager.ready,'function_found':manager.found,
                'methods':manager.records,'private_fs_sealed':boundary.sealed,
                'proc_windows':boundary.windows,'proc_window_open':bool(boundary.proc_depth),
                'module_global_continuity':False}
        detail={'failures':[(test.id(),text) for test,text in result.failures],
                'errors':[(test.id(),text) for test,text in result.errors],
                'fixture_assertions':[str(error) for error in manager.fixture_assertions]}
        encoded=(json.dumps(detail,sort_keys=True)+'\n').encode()
        if len(encoded)>1024*1024:raise ValueError('runtime.detail-limit')
        details.write(encoded);details.flush()
        summary.write((json.dumps(record,sort_keys=True)+'\n').encode());summary.flush()
        passed=(result.wasSuccessful() and counts['tests']==1
                and not any(counts[key] for key in ('skipped','expected_failures','unexpected_successes'))
                and not manager.failed and manager.calls>0 and boundary.proc_depth==0)
        return int(not passed)


if __name__=='__main__':raise SystemExit(main())
