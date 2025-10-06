from datetime import datetime
from pathlib import Path

import pytest

from prgit.sync.git import (
    Author,
    Commit,
    FileStatusType,
    Repository,
    VirtualGitEngine,
    VirtualGitRegistry,
)


@pytest.fixture(autouse=True)
def clear_registry():
    VirtualGitRegistry.instance().clear()
    yield
    VirtualGitRegistry.instance().clear()


def test_init_repo():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    assert engine._initialized is True
    assert len(engine._commits) == 0
    assert len(engine._branches) == 0


def test_manual_repository_construction():
    author = Author("Test User", "test@example.com")
    commit = Commit(
        hash="abc123",
        author=author,
        timestamp=datetime(2025, 1, 1, 12, 0, 0),
        message="Initial commit",
        parent_hashes=[],
    )
    
    repository = Repository(
        commits={"abc123": commit},
        branches={"main": "abc123"},
        head="main",
    )
    
    assert repository.commits["abc123"].message == "Initial commit"
    assert repository.branches["main"] == "abc123"
    assert repository.head == "main"


def test_repository_cloning_with_registry():
    author = Author("Test User", "test@example.com")
    commit = Commit(
        hash="abc123",
        author=author,
        timestamp=datetime(2025, 1, 1, 12, 0, 0),
        message="Initial commit",
        parent_hashes=[],
    )
    
    repository = Repository(
        commits={"abc123": commit},
        branches={"main": "abc123"},
        head="main",
    )
    
    registry = VirtualGitRegistry.instance()
    registry.register("virtual://test-repo", repository)
    
    engine = VirtualGitEngine()
    engine.clone_repo("virtual://test-repo", Path("/fake/clone"))
    
    assert len(engine._commits) == 1
    assert "abc123" in engine._commits
    assert engine._branches["main"] == "abc123"
    assert engine._head == "main"


def test_stage_and_commit():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    files = {Path("test.txt"): b"Hello, World!"}
    timestamp = datetime(2025, 1, 1, 12, 0, 0)
    
    commit = engine.stage_and_commit(files, "Initial commit", author, timestamp)
    
    assert commit.message == "Initial commit"
    assert commit.author == author
    assert commit.timestamp == timestamp
    assert len(commit.parent_hashes) == 0
    assert commit.hash in engine._commits
    assert engine._branches["main"] == commit.hash


def test_get_commits():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit1 = engine.stage_and_commit(
        {Path("file1.txt"): b"Content 1"},
        "First commit",
        author,
        datetime(2025, 1, 1, 12, 0, 0),
    )
    
    commit2 = engine.stage_and_commit(
        {Path("file2.txt"): b"Content 2"},
        "Second commit",
        author,
        datetime(2025, 1, 1, 13, 0, 0),
    )
    
    commits = engine.get_commits()
    
    assert len(commits) == 2
    assert commits[0].hash == commit2.hash
    assert commits[1].hash == commit1.hash


def test_get_commit():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit = engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Test commit",
        author,
    )
    
    retrieved = engine.get_commit(commit.hash)
    
    assert retrieved.hash == commit.hash
    assert retrieved.message == "Test commit"


def test_create_branch():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit = engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    branch = engine.create_branch("feature")
    
    assert branch.name == "feature"
    assert branch.commit_hash == commit.hash
    assert "feature" in engine._branches


def test_get_branches():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    engine.create_branch("feature")
    engine.create_branch("develop")
    
    branches = engine.get_branches()
    
    assert len(branches) == 3
    branch_names = {b.name for b in branches}
    assert branch_names == {"main", "feature", "develop"}


def test_checkout_branch():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    engine.create_branch("feature")
    engine.checkout("feature")
    
    current = engine.get_current_branch()
    assert current is not None
    assert current.name == "feature"


def test_delete_branch():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    engine.create_branch("feature")
    engine.delete_branch("feature")
    
    branches = engine.get_branches()
    branch_names = {b.name for b in branches}
    assert "feature" not in branch_names


def test_get_file_status():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    engine.stage_and_commit(
        {Path("committed.txt"): b"Committed content"},
        "Initial commit",
        author,
    )
    
    engine._working_files[Path("new.txt")] = b"New content"
    engine._working_files[Path("committed.txt")] = b"Modified content"
    
    statuses = engine.get_file_status()
    
    status_dict = {s.path: s.status for s in statuses}
    assert status_dict[Path("new.txt")] == FileStatusType.UNTRACKED
    assert status_dict[Path("committed.txt")] == FileStatusType.MODIFIED


def test_merge():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit1 = engine.stage_and_commit(
        {Path("base.txt"): b"Base content"},
        "Initial commit",
        author,
    )
    
    engine.create_branch("feature")
    engine.checkout("feature")
    
    commit2 = engine.stage_and_commit(
        {Path("feature.txt"): b"Feature content"},
        "Feature commit",
        author,
    )
    
    engine.checkout("main")
    
    merge_commit = engine.merge("feature", "Merge feature branch")
    
    assert merge_commit.message == "Merge feature branch"
    assert len(merge_commit.parent_hashes) == 2
    assert commit1.hash in merge_commit.parent_hashes
    assert commit2.hash in merge_commit.parent_hashes


def test_export_repository():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit = engine.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    engine.create_branch("feature")
    
    repository = engine.export_repository()
    
    assert len(repository.commits) == 1
    assert commit.hash in repository.commits
    assert len(repository.branches) == 2
    assert repository.branches["main"] == commit.hash
    assert repository.head == "main"


def test_virtual_engine_state_consistency():
    engine = VirtualGitEngine()
    engine.init_repo(Path("/fake/repo"))
    
    author = Author("Test User", "test@example.com")
    
    commit1 = engine.stage_and_commit(
        {Path("file1.txt"): b"Content 1"},
        "First commit",
        author,
    )
    
    engine.create_branch("branch1")
    engine.checkout("branch1")
    
    commit2 = engine.stage_and_commit(
        {Path("file2.txt"): b"Content 2"},
        "Second commit",
        author,
    )
    
    assert engine._branches["main"] == commit1.hash
    assert engine._branches["branch1"] == commit2.hash
    assert engine._head == "branch1"
    assert commit2.parent_hashes[0] == commit1.hash


def test_cloned_repository_identical():
    engine1 = VirtualGitEngine()
    engine1.init_repo(Path("/fake/repo1"))
    
    author = Author("Test User", "test@example.com")
    
    commit = engine1.stage_and_commit(
        {Path("test.txt"): b"Content"},
        "Initial commit",
        author,
    )
    
    engine1.create_branch("feature")
    
    repository = engine1.export_repository()
    
    registry = VirtualGitRegistry.instance()
    registry.register("virtual://clone-source", repository)
    
    engine2 = VirtualGitEngine()
    engine2.clone_repo("virtual://clone-source", Path("/fake/repo2"))
    
    assert engine1._commits.keys() == engine2._commits.keys()
    assert engine1._branches == engine2._branches
    assert engine1._head == engine2._head


def test_registry_operations():
    registry = VirtualGitRegistry.instance()
    
    author = Author("Test User", "test@example.com")
    commit = Commit(
        hash="abc123",
        author=author,
        timestamp=datetime(2025, 1, 1, 12, 0, 0),
        message="Test commit",
        parent_hashes=[],
    )
    
    repository = Repository(
        commits={"abc123": commit},
        branches={"main": "abc123"},
        head="main",
    )
    
    registry.register("virtual://test", repository)
    
    retrieved = registry.get("virtual://test")
    assert retrieved.commits["abc123"].message == "Test commit"
    
    registry.unregister("virtual://test")
    
    with pytest.raises(ValueError):
        registry.get("virtual://test")
    
    registry.clear()
    assert len(registry._repositories) == 0

