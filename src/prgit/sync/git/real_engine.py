from datetime import datetime, timezone
from pathlib import Path

import git

from prgit.sync.git.abstract_engine import GitEngine
from prgit.sync.git.types import Author, Branch, Commit, FileStatus, FileStatusType, Repository


class RealGitEngine(GitEngine):
    def __init__(self, repo_path: Path) -> None:
        self.repo_path = repo_path
        try:
            self.repo = git.Repo(repo_path)
        except git.exc.InvalidGitRepositoryError:
            raise ValueError(f"'{repo_path}' is not a valid git repository")

    def init_repo(self, path: Path) -> None:
        self.repo_path = path
        self.repo = git.Repo.init(path)

    def clone_repo(self, source: str, target_path: Path) -> None:
        self.repo_path = target_path
        self.repo = git.Repo.clone_from(source, target_path)

    def export_repository(self) -> Repository:
        commits: dict[str, Commit] = {}
        for commit_obj in self.repo.iter_commits("--all"):
            commit = self._convert_commit(commit_obj)
            commits[commit.hash] = commit

        branches: dict[str, str] = {}
        for ref in self.repo.heads:
            branches[ref.name] = ref.commit.hexsha

        head = ""
        if not self.repo.head.is_detached and self.repo.head.ref:
            head = self.repo.head.ref.name
        elif self.repo.head.commit:
            head = self.repo.head.commit.hexsha

        return Repository(commits=commits, branches=branches, head=head)

    def get_commits(self, branch: str | None = None) -> list[Commit]:
        try:
            if branch:
                commits_iter = self.repo.iter_commits(branch)
            else:
                commits_iter = self.repo.iter_commits()
            return [self._convert_commit(c) for c in commits_iter]
        except git.exc.GitCommandError as e:
            raise ValueError(f"Failed to get commits: {e}")

    def get_commit(self, commit_hash: str) -> Commit:
        try:
            commit_obj = self.repo.commit(commit_hash)
            return self._convert_commit(commit_obj)
        except (git.exc.BadName, ValueError) as e:
            raise ValueError(f"Commit '{commit_hash}' not found: {e}")

    def get_branches(self) -> list[Branch]:
        return [Branch(name=ref.name, commit_hash=ref.commit.hexsha) for ref in self.repo.heads]

    def get_current_branch(self) -> Branch | None:
        if self.repo.head.is_detached:
            return None
        return Branch(name=self.repo.active_branch.name, commit_hash=self.repo.active_branch.commit.hexsha)

    def create_branch(self, name: str, from_commit: str | None = None) -> Branch:
        try:
            if from_commit:
                commit = self.repo.commit(from_commit)
                new_branch = self.repo.create_head(name, commit)
            else:
                new_branch = self.repo.create_head(name)
            return Branch(name=new_branch.name, commit_hash=new_branch.commit.hexsha)
        except (git.exc.GitCommandError, ValueError) as e:
            raise ValueError(f"Failed to create branch '{name}': {e}")

    def checkout(self, branch_or_commit: str) -> None:
        try:
            self.repo.git.checkout(branch_or_commit)
        except git.exc.GitCommandError as e:
            raise ValueError(f"Failed to checkout '{branch_or_commit}': {e}")

    def delete_branch(self, name: str, force: bool = False) -> None:
        try:
            self.repo.delete_head(name, force=force)
        except git.exc.GitCommandError as e:
            raise ValueError(f"Failed to delete branch '{name}': {e}")

    def get_file_status(self) -> list[FileStatus]:
        statuses: list[FileStatus] = []
        
        for item in self.repo.index.diff(None):
            path = Path(item.a_path)
            if item.deleted_file:
                statuses.append(FileStatus(path=path, status=FileStatusType.DELETED))
            elif item.new_file:
                statuses.append(FileStatus(path=path, status=FileStatusType.ADDED))
            else:
                statuses.append(FileStatus(path=path, status=FileStatusType.MODIFIED))
        
        for item in self.repo.index.diff("HEAD"):
            path = Path(item.a_path)
            if item.new_file:
                statuses.append(FileStatus(path=path, status=FileStatusType.ADDED))
        
        for path_str in self.repo.untracked_files:
            statuses.append(FileStatus(path=Path(path_str), status=FileStatusType.UNTRACKED))
        
        return statuses

    def stage_and_commit(
        self,
        files: dict[Path, bytes],
        message: str,
        author: Author,
        timestamp: datetime | None = None,
    ) -> Commit:
        for path, content in files.items():
            full_path = self.repo_path / path
            full_path.parent.mkdir(parents=True, exist_ok=True)
            full_path.write_bytes(content)
            self.repo.index.add([str(path)])
        
        author_str = f"{author.name} <{author.email}>"
        
        if timestamp:
            env = {"GIT_AUTHOR_DATE": timestamp.isoformat(), "GIT_COMMITTER_DATE": timestamp.isoformat()}
            commit_obj = self.repo.index.commit(message, author=git.Actor._from_string(author_str), committer=git.Actor._from_string(author_str), author_date=timestamp.isoformat(), commit_date=timestamp.isoformat())
        else:
            commit_obj = self.repo.index.commit(message, author=git.Actor._from_string(author_str))
        
        return self._convert_commit(commit_obj)

    def merge(self, branch: str, message: str | None = None) -> Commit:
        try:
            base = self.repo.merge_base(self.repo.head.commit, self.repo.heads[branch].commit)
            if not base:
                raise ValueError(f"No merge base found for branch '{branch}'")
            
            self.repo.index.merge_tree(self.repo.head.commit, base_commit=base[0])
            
            merge_msg = message or f"Merge branch '{branch}'"
            commit_obj = self.repo.index.commit(
                merge_msg,
                parent_commits=(self.repo.head.commit, self.repo.heads[branch].commit),
            )
            
            return self._convert_commit(commit_obj)
        except (git.exc.GitCommandError, KeyError) as e:
            raise ValueError(f"Failed to merge branch '{branch}': {e}")

    def _convert_commit(self, commit_obj: git.Commit) -> Commit:
        author = Author(name=commit_obj.author.name, email=commit_obj.author.email)
        timestamp = datetime.fromtimestamp(commit_obj.authored_date, tz=timezone.utc)
        parent_hashes = [p.hexsha for p in commit_obj.parents]
        
        return Commit(
            hash=commit_obj.hexsha,
            author=author,
            timestamp=timestamp,
            message=commit_obj.message.strip(),
            parent_hashes=parent_hashes,
        )

