from pathlib import Path

from prgit.sync.actions.base import SyncAction
from prgit.sync.actions.exceptions import SyncExecutionError
from prgit.sync.git.abstract_engine import GitEngine
from prgit.sync.git.types import Author
from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.types import Changelist, FileActionType


class ImportChangelist(SyncAction):
    def __init__(
        self, git: GitEngine, perforce: PerforceEngine, changelist: int
    ) -> None:
        super().__init__(git, perforce)
        self._changelist = changelist

    def perform(self) -> None:
        changelist = self._fetch_changelist()
        files = self._sync_changelist_files(changelist)
        message = self._format_commit_message(changelist)
        author = self._create_author(changelist)
        self._git.stage_and_commit(
            files=files,
            message=message,
            author=author,
            timestamp=changelist.timestamp,
        )

    def _fetch_changelist(self) -> Changelist:
        try:
            return self._perforce.get_changelist(self._changelist)
        except ValueError as e:
            raise SyncExecutionError(
                message=f"Failed to fetch changelist {self._changelist}: {e}",
                action=self,
                operation="fetch_changelist",
            ) from e

    def _sync_changelist_files(self, changelist: Changelist) -> dict[Path, bytes]:
        files: dict[Path, bytes] = {}
        for file_action in changelist.files:
            if not self._perforce.is_path_in_client_view(file_action.depot_path):
                continue

            if file_action.action == FileActionType.DELETE:
                continue

            if file_action.revision is None:
                raise SyncExecutionError(
                    message=f"File {file_action.depot_path} has no revision",
                    action=self,
                    operation="sync_changelist",
                )

            try:
                content = self._perforce.get_changelist_file_content(
                    file_action.depot_path, file_action.revision
                )
                local_path = self._depot_to_local_path(file_action.depot_path)
                files[local_path] = content
            except ValueError as e:
                raise SyncExecutionError(
                    message=f"Failed to sync file {file_action.depot_path}@{file_action.revision}: {e}",
                    action=self,
                    operation="sync_changelist",
                ) from e

        return files

    def _depot_to_local_path(self, depot_path: str) -> Path:
        parts = depot_path.split("/")
        return Path(*parts[2:]) if len(parts) > 2 else Path(depot_path)

    def _format_commit_message(self, changelist: Changelist) -> str:
        date_str = changelist.timestamp.strftime("%Y-%m-%d %H:%M:%S")
        return f"{changelist.description}\n\n[CL: {changelist.number}, user: {changelist.user}, date: {date_str}]"

    def _create_author(self, changelist: Changelist) -> Author:
        try:
            user = self._perforce.get_user(changelist.user)
            return Author(name=user.full_name, email=user.email)
        except ValueError:
            return Author(name=changelist.user, email=f"{changelist.user}@example.com")
