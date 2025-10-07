from abc import ABC, abstractmethod
from datetime import datetime
from pathlib import Path

from prgit.sync.git.types import Author, Branch, Commit, FileStatus, Repository


class GitEngine(ABC):
    @abstractmethod
    def init_repo(self, path: Path) -> None:
        pass

    @abstractmethod
    def clone_repo(self, source: str, target_path: Path) -> None:
        pass

    @abstractmethod
    def export_repository(self) -> Repository:
        pass

    @abstractmethod
    def get_commits(self, branch: str | None = None) -> list[Commit]:
        pass

    @abstractmethod
    def get_commit(self, commit_hash: str) -> Commit:
        pass

    @abstractmethod
    def get_branches(self) -> list[Branch]:
        pass

    @abstractmethod
    def get_current_branch(self) -> Branch | None:
        pass

    @abstractmethod
    def create_branch(self, name: str, from_commit: str | None = None) -> Branch:
        pass

    @abstractmethod
    def checkout(self, branch_or_commit: str) -> None:
        pass

    @abstractmethod
    def delete_branch(self, name: str, force: bool = False) -> None:
        pass

    @abstractmethod
    def get_file_status(self) -> list[FileStatus]:
        pass

    @abstractmethod
    def stage_and_commit(
        self,
        files: dict[Path, bytes],
        message: str,
        author: Author,
        timestamp: datetime | None = None,
    ) -> Commit:
        pass

    @abstractmethod
    def merge(self, branch: str, message: str | None = None) -> Commit:
        pass
