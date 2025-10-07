from datetime import datetime
from pathlib import Path
from threading import Lock

from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    Client,
    FileAction,
    FileActionType,
    ShelvedChange,
)
from prgit.sync.virtual_registry import VirtualRegistry

VirtualPerforceRegistry = VirtualRegistry[Client]


class VirtualPerforceEngine(PerforceEngine):
    def __init__(self, client_mappings: list[tuple[str, Path]]) -> None:
        self._mappings = client_mappings
        self._changelists: dict[int, Changelist] = {}
        self._shelved_files: dict[int, dict[str, bytes]] = {}
        self._file_revisions: dict[str, dict[int, bytes]] = {}
        self._next_changelist_number = 1
        self._lock = Lock()

        self._load_from_registry()

    def _load_from_registry(self) -> None:
        if not self._mappings:
            return

        first_depot_path = self._mappings[0][0]
        depot_root = self._extract_depot_root(first_depot_path)

        registry = VirtualPerforceRegistry.instance()
        try:
            client = registry.get(depot_root)
            self._changelists = dict(client.changelists)
            self._file_revisions = {
                path: dict(revisions)
                for path, revisions in client.file_revisions.items()
            }
            if self._changelists:
                self._next_changelist_number = max(self._changelists.keys()) + 1
        except ValueError:
            pass

    def _extract_depot_root(self, depot_path: str) -> str:
        if depot_path.endswith("/..."):
            return depot_path[:-4]
        if depot_path.endswith("/"):
            return depot_path[:-1]
        return depot_path

    def export_client(self) -> Client:
        with self._lock:
            return Client(
                changelists=dict(self._changelists),
                file_revisions={
                    path: dict(revisions)
                    for path, revisions in self._file_revisions.items()
                },
            )

    def get_changelist(self, number: int) -> Changelist:
        with self._lock:
            if number not in self._changelists:
                raise ValueError(f"Changelist {number} not found")
            return self._changelists[number]

    def get_changelists(
        self, status: ChangelistStatus | None = None, max_results: int | None = None
    ) -> list[Changelist]:
        with self._lock:
            changelists = list(self._changelists.values())

            if status is not None:
                changelists = [cl for cl in changelists if cl.status == status]

            changelists.sort(key=lambda cl: cl.number, reverse=True)

            if max_results is not None:
                changelists = changelists[:max_results]

            return changelists

    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        with self._lock:
            if depot_path not in self._file_revisions:
                raise ValueError(f"File {depot_path} not found")
            if revision not in self._file_revisions[depot_path]:
                raise ValueError(f"Revision {revision} of {depot_path} not found")
            return self._file_revisions[depot_path][revision]

    def create_changelist(self, description: str) -> Changelist:
        with self._lock:
            changelist = Changelist(
                number=self._next_changelist_number,
                description=description,
                user="virtualuser",
                client="virtualclient",
                timestamp=datetime.now(),
                status=ChangelistStatus.PENDING,
                files=[],
            )
            self._changelists[changelist.number] = changelist
            self._next_changelist_number += 1
            return changelist

    def update_changelist_description(
        self, number: int, description: str
    ) -> Changelist:
        with self._lock:
            if number not in self._changelists:
                raise ValueError(f"Changelist {number} not found")

            old_changelist = self._changelists[number]
            updated_changelist = Changelist(
                number=old_changelist.number,
                description=description,
                user=old_changelist.user,
                client=old_changelist.client,
                timestamp=old_changelist.timestamp,
                status=old_changelist.status,
                files=old_changelist.files,
            )
            self._changelists[number] = updated_changelist
            return updated_changelist

    def shelve_files(
        self, changelist_number: int, files: dict[str, bytes]
    ) -> ShelvedChange:
        with self._lock:
            if changelist_number not in self._changelists:
                raise ValueError(f"Changelist {changelist_number} not found")

            self._shelved_files[changelist_number] = dict(files)

            old_changelist = self._changelists[changelist_number]
            file_actions = [
                FileAction(depot_path=path, action=FileActionType.EDIT, revision=None)
                for path in files.keys()
            ]

            shelved_changelist = Changelist(
                number=old_changelist.number,
                description=old_changelist.description,
                user=old_changelist.user,
                client=old_changelist.client,
                timestamp=old_changelist.timestamp,
                status=ChangelistStatus.SHELVED,
                files=file_actions,
            )
            self._changelists[changelist_number] = shelved_changelist

            return ShelvedChange(changelist=shelved_changelist, files=dict(files))
