from abc import ABC, abstractmethod

from prgit.sync.perforce.types import Changelist, ChangelistStatus, ShelvedChange


class PerforceEngine(ABC):
    @abstractmethod
    def get_changelist(self, number: int) -> Changelist:
        pass

    @abstractmethod
    def get_changelists(
        self,
        status: ChangelistStatus | None = None,
        max_results: int | None = None,
    ) -> list[Changelist]:
        pass

    @abstractmethod
    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        pass

    @abstractmethod
    def create_changelist(self, description: str) -> Changelist:
        pass

    @abstractmethod
    def update_changelist_description(
        self, number: int, description: str
    ) -> Changelist:
        pass

    @abstractmethod
    def shelve_files(
        self, changelist_number: int, files: dict[str, bytes]
    ) -> ShelvedChange:
        pass
