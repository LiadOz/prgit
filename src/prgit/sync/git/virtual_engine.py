import hashlib
from datetime import datetime
from pathlib import Path

from prgit.sync.git.abstract_engine import GitEngine
from prgit.sync.git.types import (
    Author,
    Branch,
    Commit,
    FileStatus,
    FileStatusType,
    Repository,
)
from prgit.sync.virtual_registry import VirtualRegistry


class VirtualGitEngine(GitEngine):
    def __init__(self) -> None:
        self._commits: dict[str, Commit] = {}
        self._branches: dict[str, str] = {}
        self._head: str | None = None
        self._working_files: dict[Path, bytes] = {}
        self._commit_files: dict[str, dict[Path, bytes]] = {}
        self._initialized: bool = False

    def init_repo(self, path: Path) -> None:
        self._commits = {}
        self._branches = {}
        self._head = None
        self._working_files = {}
        self._commit_files = {}
        self._initialized = True

    def clone_repo(self, source: str, target_path: Path) -> None:
        registry = VirtualRegistry[Repository].instance()
        repository = registry.get(source)

        self._commits = dict(repository.commits)
        self._branches = dict(repository.branches)
        self._head = repository.head
        self._working_files = {}
        self._initialized = True

        if self._head:
            self._checkout_files(self._resolve_head_commit())

    def export_repository(self) -> Repository:
        return Repository(
            commits=dict(self._commits),
            branches=dict(self._branches),
            head=self._head or "",
        )

    def get_commits(self, branch: str | None = None) -> list[Commit]:
        if branch:
            if branch not in self._branches:
                raise ValueError(f"Branch '{branch}' not found")
            commit_hash = self._branches[branch]
        else:
            commit_hash = self._resolve_head_commit()

        commits: list[Commit] = []
        visited: set[str] = set()
        stack: list[str] = [commit_hash]

        while stack:
            current_hash = stack.pop()
            if current_hash in visited:
                continue  # pragma: no cover
            visited.add(current_hash)

            commit = self._commits[current_hash]
            commits.append(commit)

            for parent_hash in commit.parent_hashes:
                if parent_hash not in visited:
                    stack.append(parent_hash)

        return commits

    def get_commit(self, commit_hash: str) -> Commit:
        if commit_hash not in self._commits:
            raise ValueError(f"Commit '{commit_hash}' not found")
        return self._commits[commit_hash]

    def get_branches(self) -> list[Branch]:
        return [
            Branch(name=name, commit_hash=hash) for name, hash in self._branches.items()
        ]

    def get_current_branch(self) -> Branch | None:
        if self._head is None:
            return None
        if self._head in self._branches:
            return Branch(name=self._head, commit_hash=self._branches[self._head])
        return None

    def create_branch(self, name: str, from_commit: str | None = None) -> Branch:
        if name in self._branches:
            raise ValueError(f"Branch '{name}' already exists")

        commit_hash = from_commit if from_commit else self._resolve_head_commit()
        if commit_hash not in self._commits:
            raise ValueError(f"Commit '{commit_hash}' not found")

        self._branches[name] = commit_hash
        return Branch(name=name, commit_hash=commit_hash)

    def checkout(self, branch_or_commit: str) -> None:
        if branch_or_commit in self._branches:
            self._head = branch_or_commit
            commit_hash = self._branches[branch_or_commit]
        elif branch_or_commit in self._commits:
            self._head = branch_or_commit
            commit_hash = branch_or_commit
        else:
            raise ValueError(f"Branch or commit '{branch_or_commit}' not found")

        self._checkout_files(commit_hash)

    def delete_branch(self, name: str, force: bool = False) -> None:
        if name not in self._branches:
            raise ValueError(f"Branch '{name}' not found")
        if self._head == name:
            raise ValueError(f"Cannot delete checked out branch '{name}'")
        del self._branches[name]

    def get_file_status(self) -> list[FileStatus]:
        commit_hash = self._resolve_head_commit()
        commit_files = self._get_commit_files(commit_hash) if commit_hash else {}

        statuses: list[FileStatus] = []
        all_paths = set(self._working_files.keys()) | set(commit_files.keys())

        for path in all_paths:
            working_content = self._working_files.get(path)
            commit_content = commit_files.get(path)

            if working_content is None and commit_content is not None:
                statuses.append(FileStatus(path=path, status=FileStatusType.DELETED))
            elif working_content is not None and commit_content is None:
                statuses.append(FileStatus(path=path, status=FileStatusType.UNTRACKED))
            elif working_content != commit_content:
                statuses.append(FileStatus(path=path, status=FileStatusType.MODIFIED))

        return statuses

    def stage_and_commit(
        self,
        files: dict[Path, bytes],
        message: str,
        author: Author,
        timestamp: datetime | None = None,
    ) -> Commit:
        for path, content in files.items():
            self._working_files[path] = content

        parent_hash = self._resolve_head_commit() if self._head else None
        parent_hashes = [parent_hash] if parent_hash else []

        commit_hash = self._generate_commit_hash(
            files, message, author, timestamp or datetime.now(), parent_hashes
        )

        commit = Commit(
            hash=commit_hash,
            author=author,
            timestamp=timestamp or datetime.now(),
            message=message,
            parent_hashes=parent_hashes,
        )

        self._commits[commit_hash] = commit
        self._commit_files[commit_hash] = dict(self._working_files)

        if self._head and self._head in self._branches:
            self._branches[self._head] = commit_hash
        elif self._head:
            self._head = commit_hash
        else:
            self._branches["main"] = commit_hash
            self._head = "main"

        return commit

    def merge(self, branch: str, message: str | None = None) -> Commit:
        if branch not in self._branches:
            raise ValueError(f"Branch '{branch}' not found")

        current_hash = self._resolve_head_commit()
        merge_hash = self._branches[branch]

        merge_message = message or f"Merge branch '{branch}'"

        merge_files = self._get_commit_files(merge_hash)
        for path, content in merge_files.items():
            self._working_files[path] = content

        author = Author(name="VirtualGit", email="virtual@git.com")
        commit_hash = self._generate_commit_hash(
            self._working_files,
            merge_message,
            author,
            datetime.now(),
            [current_hash, merge_hash],
        )

        commit = Commit(
            hash=commit_hash,
            author=author,
            timestamp=datetime.now(),
            message=merge_message,
            parent_hashes=[current_hash, merge_hash],
        )

        self._commits[commit_hash] = commit
        self._commit_files[commit_hash] = dict(self._working_files)

        if self._head and self._head in self._branches:
            self._branches[self._head] = commit_hash
        elif self._head:
            self._head = commit_hash

        return commit

    def _resolve_head_commit(self) -> str:
        if not self._head:
            raise ValueError("HEAD is not set")
        if self._head in self._branches:
            return self._branches[self._head]
        return self._head

    def _checkout_files(self, commit_hash: str) -> None:
        self._working_files = self._get_commit_files(commit_hash)

    def _get_commit_files(self, commit_hash: str) -> dict[Path, bytes]:
        if not commit_hash or commit_hash not in self._commits:
            return {}

        if commit_hash in self._commit_files:
            return dict(self._commit_files[commit_hash])

        return {}

    def _generate_commit_hash(
        self,
        files: dict[Path, bytes],
        message: str,
        author: Author,
        timestamp: datetime,
        parent_hashes: list[str],
    ) -> str:
        content = f"{message}{author.name}{author.email}{timestamp.isoformat()}"
        content += "".join(parent_hashes)
        for path, data in sorted(files.items()):
            content += f"{path}{data.hex()}"
        return hashlib.sha256(content.encode()).hexdigest()[:40]
