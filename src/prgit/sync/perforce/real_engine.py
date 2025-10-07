from datetime import datetime

from P4 import P4, P4Exception  # type: ignore[import-untyped]

from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    FileAction,
    FileActionType,
    ShelvedChange,
)


class RealPerforceEngine(PerforceEngine):
    def __init__(self) -> None:
        self.p4 = P4()
        try:
            self.p4.connect()
        except P4Exception as e:
            raise ValueError(f"Failed to connect to Perforce: {e}")

    def __del__(self) -> None:
        if hasattr(self, "p4") and self.p4.connected():
            self.p4.disconnect()

    def get_changelist(self, number: int) -> Changelist:
        try:
            result = self.p4.run_describe("-s", str(number))
            if not result:
                raise ValueError(f"Changelist {number} not found")

            desc = result[0]
            status = self._parse_status(desc.get("status", ""))
            files = self._parse_files(desc)

            return Changelist(
                number=int(desc["change"]),
                description=desc.get("desc", ""),
                user=desc.get("user", ""),
                client=desc.get("client", ""),
                timestamp=datetime.fromtimestamp(int(desc["time"])),
                status=status,
                files=files,
            )
        except P4Exception as e:
            raise ValueError(f"Failed to get changelist {number}: {e}")

    def get_changelists(
        self,
        status: ChangelistStatus | None = None,
        max_results: int | None = None,
    ) -> list[Changelist]:
        try:
            args = []

            if status == ChangelistStatus.PENDING:
                args.extend(["-s", "pending"])
            elif status == ChangelistStatus.SHELVED:
                args.extend(["-s", "shelved"])
            elif status == ChangelistStatus.SUBMITTED:
                args.extend(["-s", "submitted"])

            if max_results is not None:
                args.extend(["-m", str(max_results)])

            changes = self.p4.run_changes(*args)

            changelists = []
            for change in changes:
                cl_number = int(change["change"])
                changelists.append(self.get_changelist(cl_number))

            return changelists
        except P4Exception as e:
            raise ValueError(f"Failed to get changelists: {e}")

    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        try:
            file_spec = f"{depot_path}#{revision}"
            result = self.p4.run_print("-q", file_spec)

            if not result or len(result) < 2:
                raise ValueError(f"File {depot_path} revision {revision} not found")

            content = result[1]
            if isinstance(content, str):
                return content.encode("utf-8")
            return content
        except P4Exception as e:
            raise ValueError(f"Failed to get content for {depot_path}#{revision}: {e}")

    def create_changelist(self, description: str) -> Changelist:
        try:
            change = self.p4.fetch_change()
            change["Description"] = description

            result = self.p4.save_change(change)

            if not result or len(result) < 1:
                raise ValueError("Failed to create changelist")

            change_number = int(result[0].split()[1])
            return self.get_changelist(change_number)
        except P4Exception as e:
            raise ValueError(f"Failed to create changelist: {e}")

    def update_changelist_description(
        self, number: int, description: str
    ) -> Changelist:
        try:
            change = self.p4.fetch_change(str(number))
            change["Description"] = description
            self.p4.save_change(change)

            return self.get_changelist(number)
        except P4Exception as e:
            raise ValueError(f"Failed to update changelist {number}: {e}")

    def shelve_files(
        self, changelist_number: int, files: dict[str, bytes]
    ) -> ShelvedChange:
        try:
            for depot_path, content in files.items():
                local_path = self._depot_to_local(depot_path)

                with open(local_path, "wb") as f:
                    f.write(content)

                try:
                    self.p4.run_edit("-c", str(changelist_number), depot_path)
                except P4Exception:
                    self.p4.run_add("-c", str(changelist_number), depot_path)

            self.p4.run_shelve("-c", str(changelist_number))

            changelist = self.get_changelist(changelist_number)
            return ShelvedChange(changelist=changelist, files=files)
        except P4Exception as e:
            raise ValueError(f"Failed to shelve files: {e}")

    def _parse_status(self, status_str: str) -> ChangelistStatus:
        status_lower = status_str.lower()
        if status_lower == "pending":
            return ChangelistStatus.PENDING
        elif status_lower == "shelved":
            return ChangelistStatus.SHELVED
        elif status_lower == "submitted":
            return ChangelistStatus.SUBMITTED
        return ChangelistStatus.PENDING

    def _parse_files(self, desc: dict) -> list[FileAction]:
        files = []
        depot_files = desc.get("depotFile", [])
        actions = desc.get("action", [])
        revisions = desc.get("rev", [])

        if not isinstance(depot_files, list):
            depot_files = [depot_files]
        if not isinstance(actions, list):
            actions = [actions]
        if not isinstance(revisions, list):
            revisions = [revisions]

        for depot_file, action, rev in zip(depot_files, actions, revisions):
            files.append(
                FileAction(
                    depot_path=depot_file,
                    action=self._parse_action(action),
                    revision=int(rev) if rev else None,
                )
            )

        return files

    def _parse_action(self, action_str: str) -> FileActionType:
        action_lower = action_str.lower()
        if action_lower == "add":
            return FileActionType.ADD
        elif action_lower == "edit":
            return FileActionType.EDIT
        elif action_lower == "delete":
            return FileActionType.DELETE
        elif action_lower == "branch":
            return FileActionType.BRANCH
        elif action_lower == "integrate":
            return FileActionType.INTEGRATE
        elif action_lower == "move/add":
            return FileActionType.MOVE_ADD
        elif action_lower == "move/delete":
            return FileActionType.MOVE_DELETE
        return FileActionType.EDIT

    def _depot_to_local(self, depot_path: str) -> str:
        try:
            result = self.p4.run_where(depot_path)
            if result and len(result) > 0:
                return result[0]["path"]
            raise ValueError(f"Could not resolve depot path: {depot_path}")
        except P4Exception as e:
            raise ValueError(f"Failed to resolve depot path {depot_path}: {e}")
