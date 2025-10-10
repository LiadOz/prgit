from abc import ABC, abstractmethod

from prgit.sync.git.abstract_engine import GitEngine
from prgit.sync.perforce.abstract_engine import PerforceEngine


class SyncAction(ABC):
    def __init__(self, git: GitEngine, perforce: PerforceEngine) -> None:
        self._git = git
        self._perforce = perforce

    @abstractmethod
    def perform(self) -> None:
        pass
