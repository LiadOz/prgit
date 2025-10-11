from datetime import datetime
from pathlib import Path

import pytest

from prgit.sync.actions import ImportChangelist, SyncExecutionError
from prgit.sync.git import VirtualGitEngine
from prgit.sync.perforce import (
    Changelist,
    ChangelistStatus,
    Client,
    FileAction,
    FileActionType,
    User,
    VirtualPerforceEngine,
    VirtualPerforceRegistry,
)


@pytest.fixture(autouse=True)
def cleanup_registry() -> None:
    registry = VirtualPerforceRegistry.instance()
    registry.clear()


def test_import_changelist_basic() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=12345,
        description="Add new feature",
        user="testuser",
        client="testclient",
        timestamp=datetime(2024, 1, 15, 10, 30, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/main.py",
                action=FileActionType.ADD,
                revision=1,
            )
        ],
    )

    client = Client(
        changelists={12345: changelist},
        file_revisions={"//depot/project/main.py": {1: b"print('hello')"}},
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=12345)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1

    commit = commits[0]
    assert "Add new feature" in commit.message
    assert "[CL: 12345, user: testuser, date: 2024-01-15 10:30:00]" in commit.message
    assert commit.author.name == "testuser"
    assert commit.author.email == "testuser@example.com"
    assert commit.timestamp == datetime(2024, 1, 15, 10, 30, 0)


def test_import_changelist_with_user_info() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=100,
        description="Fix bug",
        user="jdoe",
        client="testclient",
        timestamp=datetime(2024, 2, 20, 14, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file.txt",
                action=FileActionType.EDIT,
                revision=2,
            )
        ],
    )

    client = Client(
        changelists={100: changelist},
        file_revisions={"//depot/project/file.txt": {2: b"updated content"}},
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    user = User(username="jdoe", email="john.doe@company.com", full_name="John Doe")
    perforce._users["jdoe"] = user

    action = ImportChangelist(git, perforce, changelist=100)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1

    commit = commits[0]
    assert commit.author.name == "John Doe"
    assert commit.author.email == "john.doe@company.com"


def test_import_changelist_nonexistent() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=99999)

    with pytest.raises(SyncExecutionError) as exc_info:
        action.perform()

    error = exc_info.value
    assert error.operation == "fetch_changelist"
    assert "99999" in error.message


def test_import_changelist_with_multiple_files() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=200,
        description="Add multiple files",
        user="developer",
        client="testclient",
        timestamp=datetime(2024, 3, 1, 9, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file1.py",
                action=FileActionType.ADD,
                revision=1,
            ),
            FileAction(
                depot_path="//depot/project/file2.py",
                action=FileActionType.ADD,
                revision=1,
            ),
            FileAction(
                depot_path="//depot/project/file3.py",
                action=FileActionType.ADD,
                revision=1,
            ),
        ],
    )

    client = Client(
        changelists={200: changelist},
        file_revisions={
            "//depot/project/file1.py": {1: b"content1"},
            "//depot/project/file2.py": {1: b"content2"},
            "//depot/project/file3.py": {1: b"content3"},
        },
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=200)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1


def test_import_changelist_with_delete() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=300,
        description="Delete old file",
        user="cleaner",
        client="testclient",
        timestamp=datetime(2024, 4, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/oldfile.py",
                action=FileActionType.DELETE,
                revision=None,
            )
        ],
    )

    client = Client(changelists={300: changelist}, file_revisions={})

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=300)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1


def test_import_changelist_file_no_revision() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=400,
        description="Bad changelist",
        user="testuser",
        client="testclient",
        timestamp=datetime(2024, 5, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file.py",
                action=FileActionType.ADD,
                revision=None,
            )
        ],
    )

    client = Client(changelists={400: changelist}, file_revisions={})

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=400)

    with pytest.raises(SyncExecutionError) as exc_info:
        action.perform()

    error = exc_info.value
    assert error.operation == "sync_changelist"
    assert "has no revision" in error.message


def test_import_changelist_file_not_found() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=500,
        description="Missing file",
        user="testuser",
        client="testclient",
        timestamp=datetime(2024, 6, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/missing.py",
                action=FileActionType.ADD,
                revision=1,
            )
        ],
    )

    client = Client(changelists={500: changelist}, file_revisions={})

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=500)

    with pytest.raises(SyncExecutionError) as exc_info:
        action.perform()

    error = exc_info.value
    assert error.operation == "sync_changelist"
    assert "Failed to sync file" in error.message


def test_import_changelist_user_auto_created() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=600,
        description="User not in system",
        user="newuser",
        client="testclient",
        timestamp=datetime(2024, 7, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file.py",
                action=FileActionType.ADD,
                revision=1,
            )
        ],
    )

    client = Client(
        changelists={600: changelist},
        file_revisions={"//depot/project/file.py": {1: b"content"}},
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=600)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1
    assert commits[0].author.name == "newuser"
    assert commits[0].author.email == "newuser@example.com"

    user = perforce.get_user("newuser")
    assert user.username == "newuser"
    assert user.email == "newuser@example.com"
    assert user.full_name == "newuser"


def test_import_changelist_user_get_error_fallback() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=700,
        description="User fetch fails",
        user="erroruser",
        client="testclient",
        timestamp=datetime(2024, 8, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file.py",
                action=FileActionType.ADD,
                revision=1,
            )
        ],
    )

    client = Client(
        changelists={700: changelist},
        file_revisions={"//depot/project/file.py": {1: b"content"}},
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", client)

    perforce = VirtualPerforceEngine([("//depot/project/...", Path("/workspace"))])

    def raise_error(username: str) -> User:
        raise ValueError(f"User {username} not found")

    original_get_user = perforce.get_user
    perforce.get_user = raise_error

    action = ImportChangelist(git, perforce, changelist=700)
    action.perform()

    perforce.get_user = original_get_user

    commits = git.get_commits()
    assert len(commits) == 1
    assert commits[0].author.name == "erroruser"
    assert commits[0].author.email == "erroruser@example.com"


def test_import_changelist_mapping_filters_files() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/workspace"))

    changelist = Changelist(
        number=1,
        description="Add files in different depots",
        user="testuser",
        client="testclient",
        timestamp=datetime(2024, 9, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/A/a",
                action=FileActionType.ADD,
                revision=1,
            ),
            FileAction(
                depot_path="//depot/B/b",
                action=FileActionType.ADD,
                revision=1,
            ),
        ],
    )

    client = Client(
        changelists={1: changelist},
        file_revisions={
            "//depot/A/a": {1: b"content a"},
            "//depot/B/b": {1: b"content b"},
        },
    )

    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/A", client)

    perforce = VirtualPerforceEngine([("//depot/A/...", Path("/workspace"))])

    action = ImportChangelist(git, perforce, changelist=1)
    action.perform()

    commits = git.get_commits()
    assert len(commits) == 1

    commit = commits[0]
    commit_files = git._commit_files[commit.hash]
    assert Path("depot/A/a") in commit_files
    assert Path("depot/B/b") not in commit_files
