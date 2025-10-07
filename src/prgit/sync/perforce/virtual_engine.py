from datetime import datetime

from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    FileAction,
    FileActionType,
    ShelvedChange,
)


class VirtualPerforceEngine(PerforceEngine):
    def __init__(self) -> None:
        self._changelists: dict[int, Changelist] = {}
        self._shelved_files: dict[int, dict[str, bytes]] = {}
        self._file_revisions: dict[str, dict[int, bytes]] = {}
        self._next_changelist_number: int = 1

    def get_changelist(self, number: int) -> Changelist:
        if number not in self._changelists:
            raise ValueError(f"Changelist {number} not found")
        return self._changelists[number]

    def get_changelists(
        self,
        status: ChangelistStatus | None = None,
        max_results: int | None = None,
    ) -> list[Changelist]:
        changelists = list(self._changelists.values())

        if status is not None:
            changelists = [cl for cl in changelists if cl.status == status]

        changelists.sort(key=lambda cl: cl.number)

        if max_results is not None:
            changelists = changelists[:max_results]

        return changelists

    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        if depot_path not in self._file_revisions:
            raise ValueError(f"File {depot_path} not found")
        if revision not in self._file_revisions[depot_path]:
            raise ValueError(f"Revision {revision} of {depot_path} not found")
        return self._file_revisions[depot_path][revision]

    def create_changelist(self, description: str) -> Changelist:
        changelist_number = self._next_changelist_number
        self._next_changelist_number += 1

        changelist = Changelist(
            number=changelist_number,
            description=description,
            user="virtualuser",
            client="virtualclient",
            timestamp=datetime.now(),
            status=ChangelistStatus.PENDING,
            files=[],
        )

        self._changelists[changelist_number] = changelist
        return changelist

    def update_changelist_description(
        self, number: int, description: str
    ) -> Changelist:
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
        if changelist_number not in self._changelists:
            raise ValueError(f"Changelist {changelist_number} not found")

        old_changelist = self._changelists[changelist_number]

        file_actions: list[FileAction] = []
        for depot_path, content in files.items():
            if depot_path in self._file_revisions:
                action = FileActionType.EDIT
            else:
                action = FileActionType.ADD
                self._file_revisions[depot_path] = {}

            file_actions.append(
                FileAction(depot_path=depot_path, action=action, revision=None)
            )

        updated_changelist = Changelist(
            number=old_changelist.number,
            description=old_changelist.description,
            user=old_changelist.user,
            client=old_changelist.client,
            timestamp=old_changelist.timestamp,
            status=ChangelistStatus.SHELVED,
            files=file_actions,
        )

        self._changelists[changelist_number] = updated_changelist
        self._shelved_files[changelist_number] = dict(files)

        return ShelvedChange(changelist=updated_changelist, files=dict(files))
