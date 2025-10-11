from abc import ABC, abstractmethod
from pathlib import Path

from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    Client,
    ShelvedChange,
    User,
)


class PerforceEngine(ABC):
    def __init__(self, client_mappings: list[tuple[str, Path]]) -> None:
        self._mappings = client_mappings

    @abstractmethod
    def export_client(self) -> Client:
        pass

    @abstractmethod
    def get_changelist(self, number: int) -> Changelist:
        pass

    @abstractmethod
    def get_changelists(
        self, status: ChangelistStatus | None = None, max_results: int | None = None
    ) -> list[Changelist]:
        pass

    @abstractmethod
    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        pass

    @abstractmethod
    def get_user(self, username: str) -> User:
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

    def is_path_in_client_view(self, depot_path: str) -> bool:
        for depot_pattern, _ in self._mappings:
            depot_root = depot_pattern.rstrip("/...").rstrip("/")
            if depot_path.startswith(depot_root):
                return True
        return False
