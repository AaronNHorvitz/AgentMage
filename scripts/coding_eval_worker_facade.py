"""Module facade transporting explicit worker API attributes."""
from types import ModuleType


class Facade(ModuleType):
    def __init__(self, peer, source_path):
        super().__init__('scripts.coding_eval_oracle_worker')
        self.__dict__['_bridge'] = peer
        self.__dict__['__file__'] = source_path

    def __getattr__(self, name):
        if name.startswith('_'):
            raise AttributeError(name)
        return self.__dict__['_bridge'].request('get-subject', name, (), {})

    def __setattr__(self, name, value):
        if name.startswith('_') or name in ('__name__', '__doc__', '__package__', '__loader__', '__spec__'):
            return super().__setattr__(name, value)
        self.__dict__['_bridge'].request('set-subject', name, (value,), {})

    def __delattr__(self, name):
        if name.startswith('_'):
            return super().__delattr__(name)
        self.__dict__['_bridge'].request('delete-subject', name, (), {})
