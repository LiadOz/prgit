from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from pathlib import Path


class ChangelistStatus(StrEnum):
    PENDING = "pending"
    SHELVED = "shelved"
    SUBMITTED = "submitted"


class FileActionType(StrEnum):
    ADD = "add"
    EDIT = "edit"
    DELETE = "delete"
    BRANCH = "branch"
    INTEGRATE = "integrate"
    MOVE_ADD = "move/add"
    MOVE_DELETE = "move/delete"


@dataclass(frozen=True)
class FileAction:
    depot_path: str
    action: FileActionType
    revision: int | None


@dataclass(frozen=True)
class Changelist:
    number: int
    description: str
    user: str
    client: str
    timestamp: datetime
    status: ChangelistStatus
    files: list[FileAction]


@dataclass(frozen=True)
class ShelvedChange:
    changelist: Changelist
    files: dict[str, bytes]


@dataclass(frozen=True)
class Client:
    changelists: dict[int, Changelist]
    file_revisions: dict[str, dict[int, bytes]]
