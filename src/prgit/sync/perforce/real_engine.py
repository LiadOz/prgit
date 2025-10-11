from datetime import datetime
from pathlib import Path

from P4 import P4, P4Exception  # type: ignore[import-untyped]

from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    Client,
    FileAction,
    FileActionType,
    ShelvedChange,
    User,
)


class RealPerforceEngine(PerforceEngine):
    def __init__(self, client_mappings: list[tuple[str, Path]]) -> None:
        super().__init__(client_mappings)
        self._p4 = P4()
        self._P4Exception = P4Exception
        self._setup_client()

    def _setup_client(self) -> None:
        try:
            self._p4.connect()
            client_name = f"prgit_client_{id(self)}"
            self._p4.client = client_name

            client_spec = self._p4.fetch_client(client_name)
            client_spec["Root"] = str(self._mappings[0][1].parent)
            client_spec["View"] = [
                f"{depot_path} //{client_name}/{depot_path.split('/')[-1]}"
                for depot_path, _ in self._mappings
            ]
            self._p4.save_client(client_spec)
        except Exception as e:
            raise ValueError(f"Failed to setup Perforce client: {e}") from e

    def export_client(self) -> Client:
        try:
            changelists_dict: dict[int, Changelist] = {}
            file_revisions: dict[str, dict[int, bytes]] = {}

            changes = self._p4.run_changes("-s", "submitted")
            for change in changes:
                changelist = self._parse_changelist(change)
                changelists_dict[changelist.number] = changelist

                for file_action in changelist.files:
                    if file_action.revision is not None:
                        depot_path = file_action.depot_path
                        revision = file_action.revision
                        content = self._fetch_file_content(depot_path, revision)
                        if depot_path not in file_revisions:
                            file_revisions[depot_path] = {}
                        file_revisions[depot_path][revision] = content

            return Client(changelists=changelists_dict, file_revisions=file_revisions)
        except Exception as e:
            raise ValueError(f"Failed to export client: {e}") from e

    def _parse_changelist(self, change_data: dict) -> Changelist:
        number = int(change_data["change"])
        description = change_data.get("desc", "")
        user = change_data.get("user", "")
        client = change_data.get("client", "")
        timestamp = datetime.fromtimestamp(int(change_data.get("time", 0)))
        status_str = change_data.get("status", "submitted")
        status = ChangelistStatus(status_str)

        files = self._get_changelist_files(number)

        return Changelist(
            number=number,
            description=description,
            user=user,
            client=client,
            timestamp=timestamp,
            status=status,
            files=files,
        )

    def _get_changelist_files(self, changelist_number: int) -> list[FileAction]:
        try:
            describe = self._p4.run_describe(str(changelist_number))
            if not describe:
                return []

            desc = describe[0]
            depot_files = desc.get("depotFile", [])
            actions = desc.get("action", [])
            revisions = desc.get("rev", [])

            file_actions = []
            for depot_file, action, rev in zip(depot_files, actions, revisions):
                file_action = FileAction(
                    depot_path=depot_file,
                    action=FileActionType(action),
                    revision=int(rev),
                )
                file_actions.append(file_action)

            return file_actions
        except Exception:
            return []

    def _fetch_file_content(self, depot_path: str, revision: int) -> bytes:
        try:
            file_spec = f"{depot_path}#{revision}"
            content = self._p4.run_print(file_spec)
            if content and len(content) > 1:
                return content[1].encode("utf-8")
            return b""
        except Exception:
            return b""

    def get_changelist(self, number: int) -> Changelist:
        try:
            changes = self._p4.run_changes("-s", "submitted", f"@={number}")
            if not changes:
                raise ValueError(f"Changelist {number} not found")
            return self._parse_changelist(changes[0])
        except self._P4Exception as e:
            raise ValueError(f"Failed to get changelist: {e}") from e

    def get_changelists(
        self, status: ChangelistStatus | None = None, max_results: int | None = None
    ) -> list[Changelist]:
        try:
            args = []
            if status:
                args.extend(["-s", status.value])
            if max_results:
                args.extend(["-m", str(max_results)])

            changes = self._p4.run_changes(*args)
            return [self._parse_changelist(change) for change in changes]
        except self._P4Exception as e:
            raise ValueError(f"Failed to get changelists: {e}") from e

    def get_changelist_file_content(self, depot_path: str, revision: int) -> bytes:
        try:
            content = self._fetch_file_content(depot_path, revision)
            if not content:
                raise ValueError(f"File {depot_path}#{revision} not found")
            return content
        except self._P4Exception as e:
            raise ValueError(f"Failed to get file content: {e}") from e

    def get_user(self, username: str) -> User:
        try:
            users = self._p4.run_user("-o", username)
            if not users:
                raise ValueError(f"User {username} not found")
            user_data = users[0]
            return User(
                username=username,
                email=user_data.get("Email", f"{username}@example.com"),
                full_name=user_data.get("FullName", username),
            )
        except self._P4Exception as e:
            raise ValueError(f"Failed to get user information: {e}") from e

    def create_changelist(self, description: str) -> Changelist:
        try:
            change_spec = self._p4.fetch_change()
            change_spec["Description"] = description
            result = self._p4.save_change(change_spec)

            changelist_number = int(result[0].split()[1])
            return self.get_changelist(changelist_number)
        except self._P4Exception as e:
            raise ValueError(f"Failed to create changelist: {e}") from e

    def update_changelist_description(
        self, number: int, description: str
    ) -> Changelist:
        try:
            change_spec = self._p4.fetch_change(str(number))
            change_spec["Description"] = description
            self._p4.save_change(change_spec)
            return self.get_changelist(number)
        except self._P4Exception as e:
            raise ValueError(f"Failed to update changelist: {e}") from e

    def shelve_files(
        self, changelist_number: int, files: dict[str, bytes]
    ) -> ShelvedChange:
        try:
            for depot_path, content in files.items():
                local_path = self._depot_to_local(depot_path)
                local_path.parent.mkdir(parents=True, exist_ok=True)
                local_path.write_bytes(content)
                self._p4.run_edit("-c", str(changelist_number), str(local_path))

            self._p4.run_shelve("-c", str(changelist_number))

            changelist = self.get_changelist(changelist_number)
            return ShelvedChange(changelist=changelist, files=files)
        except self._P4Exception as e:
            raise ValueError(f"Failed to shelve files: {e}") from e

    def _depot_to_local(self, depot_path: str) -> Path:
        for depot_pattern, local_base in self._mappings:
            depot_root = depot_pattern.rstrip("/...").rstrip("/")
            if depot_path.startswith(depot_root):
                relative_path = depot_path[len(depot_root) :].lstrip("/")
                return local_base / relative_path

        return Path(depot_path.split("/")[-1])

    def __del__(self) -> None:
        if hasattr(self, "_p4"):
            try:
                self._p4.disconnect()
            except Exception:  # nosec
                pass
