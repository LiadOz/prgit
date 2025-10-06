from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum
from pathlib import Path


class FileStatusType(StrEnum):
    ADDED = "added"
    MODIFIED = "modified"
    DELETED = "deleted"
    UNTRACKED = "untracked"


@dataclass(frozen=True)
class Author:
    name: str
    email: str


@dataclass(frozen=True)
class Commit:
    hash: str
    author: Author
    timestamp: datetime
    message: str
    parent_hashes: list[str]


@dataclass(frozen=True)
class Branch:
    name: str
    commit_hash: str


@dataclass(frozen=True)
class FileStatus:
    path: Path
    status: FileStatusType


@dataclass(frozen=True)
class Repository:
    commits: dict[str, Commit]
    branches: dict[str, str]
    head: str

