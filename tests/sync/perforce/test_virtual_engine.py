from datetime import datetime
from pathlib import Path

import pytest

from prgit.sync.perforce import (
    Changelist,
    ChangelistStatus,
    Client,
    FileAction,
    FileActionType,
    VirtualPerforceEngine,
    VirtualPerforceRegistry,
)


@pytest.fixture(autouse=True)
def clear_registry():
    registry = VirtualPerforceRegistry.instance()
    registry.clear()
    yield
    registry.clear()


@pytest.fixture
def sample_changelist() -> Changelist:
    return Changelist(
        number=1,
        description="Initial commit",
        user="testuser",
        client="testclient",
        timestamp=datetime(2025, 1, 1, 12, 0, 0),
        status=ChangelistStatus.SUBMITTED,
        files=[
            FileAction(
                depot_path="//depot/project/file.py",
                action=FileActionType.ADD,
                revision=1,
            )
        ],
    )


@pytest.fixture
def sample_client(sample_changelist: Changelist) -> Client:
    return Client(
        changelists={1: sample_changelist},
        file_revisions={"//depot/project/file.py": {1: b"print('hello')"}},
    )


def test_virtual_engine_with_empty_state():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelists = engine.get_changelists()
    assert len(changelists) == 0


def test_virtual_engine_with_prepopulated_client(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelists = engine.get_changelists()
    assert len(changelists) == 1
    assert changelists[0].number == 1
    assert changelists[0].description == "Initial commit"


def test_create_changelist():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelist = engine.create_changelist("Test feature")

    assert changelist.number == 1
    assert changelist.description == "Test feature"
    assert changelist.status == ChangelistStatus.PENDING
    assert changelist.user == "virtualuser"
    assert changelist.client == "virtualclient"


def test_create_multiple_changelists():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    cl1 = engine.create_changelist("Feature 1")
    cl2 = engine.create_changelist("Feature 2")

    assert cl1.number == 1
    assert cl2.number == 2


def test_create_changelist_with_prepopulated_client(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelist = engine.create_changelist("New feature")
    assert changelist.number == 2


def test_get_changelist():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    created = engine.create_changelist("Test")
    retrieved = engine.get_changelist(created.number)

    assert retrieved.number == created.number
    assert retrieved.description == created.description


def test_get_nonexistent_changelist():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.get_changelist(999)


def test_get_changelists_by_status(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    engine.create_changelist("Pending 1")
    engine.create_changelist("Pending 2")

    pending_cls = engine.get_changelists(status=ChangelistStatus.PENDING)
    assert len(pending_cls) == 2
    assert all(cl.status == ChangelistStatus.PENDING for cl in pending_cls)

    submitted_cls = engine.get_changelists(status=ChangelistStatus.SUBMITTED)
    assert len(submitted_cls) == 1
    assert submitted_cls[0].number == 1


def test_get_changelists_with_max_results():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    for i in range(5):
        engine.create_changelist(f"Change {i}")

    changelists = engine.get_changelists(max_results=3)
    assert len(changelists) == 3


def test_get_changelists_sorted_descending():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    cl1 = engine.create_changelist("First")
    cl2 = engine.create_changelist("Second")
    cl3 = engine.create_changelist("Third")

    changelists = engine.get_changelists()
    assert changelists[0].number == cl3.number
    assert changelists[1].number == cl2.number
    assert changelists[2].number == cl1.number


def test_update_changelist_description():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    original = engine.create_changelist("Original description")
    updated = engine.update_changelist_description(
        original.number, "Updated description"
    )

    assert updated.number == original.number
    assert updated.description == "Updated description"
    assert updated.status == original.status


def test_update_nonexistent_changelist():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.update_changelist_description(999, "New description")


def test_shelve_files():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelist = engine.create_changelist("Feature")
    files = {
        "//depot/project/file1.py": b"content1",
        "//depot/project/file2.py": b"content2",
    }

    shelved = engine.shelve_files(changelist.number, files)

    assert shelved.changelist.number == changelist.number
    assert shelved.changelist.status == ChangelistStatus.SHELVED
    assert len(shelved.files) == 2
    assert shelved.files["//depot/project/file1.py"] == b"content1"


def test_shelve_files_updates_changelist_status():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    changelist = engine.create_changelist("Feature")
    files = {"//depot/project/file.py": b"content"}

    engine.shelve_files(changelist.number, files)

    updated_cl = engine.get_changelist(changelist.number)
    assert updated_cl.status == ChangelistStatus.SHELVED


def test_shelve_to_nonexistent_changelist():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    files = {"//depot/project/file.py": b"content"}

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.shelve_files(999, files)


def test_get_file_content(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    content = engine.get_changelist_file_content("//depot/project/file.py", 1)
    assert content == b"print('hello')"


def test_get_nonexistent_file_content():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    with pytest.raises(ValueError, match="File //depot/project/missing.py not found"):
        engine.get_changelist_file_content("//depot/project/missing.py", 1)


def test_get_nonexistent_file_revision(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    with pytest.raises(
        ValueError, match="Revision 99 of //depot/project/file.py not found"
    ):
        engine.get_changelist_file_content("//depot/project/file.py", 99)


def test_export_client():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    cl1 = engine.create_changelist("First")
    cl2 = engine.create_changelist("Second")

    exported = engine.export_client()

    assert len(exported.changelists) == 2
    assert cl1.number in exported.changelists
    assert cl2.number in exported.changelists


def test_export_client_with_prepopulated_state(sample_client: Client):
    registry = VirtualPerforceRegistry.instance()
    registry.register("//depot/project", sample_client)

    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    engine.create_changelist("New change")

    exported = engine.export_client()

    assert len(exported.changelists) == 2
    assert 1 in exported.changelists
    assert 2 in exported.changelists
    assert "//depot/project/file.py" in exported.file_revisions
    assert 1 in exported.file_revisions["//depot/project/file.py"]


def test_registry_isolation():
    registry = VirtualPerforceRegistry.instance()

    client1 = Client(changelists={}, file_revisions={})
    client2 = Client(changelists={}, file_revisions={})

    registry.register("//depot/project1", client1)
    registry.register("//depot/project2", client2)

    mappings1 = [("//depot/project1/...", Path("/workspace/project1"))]
    engine1 = VirtualPerforceEngine(mappings1)

    mappings2 = [("//depot/project2/...", Path("/workspace/project2"))]
    engine2 = VirtualPerforceEngine(mappings2)

    engine1.create_changelist("Project 1 change")
    engine2.create_changelist("Project 2 change")

    cls1 = engine1.get_changelists()
    cls2 = engine2.get_changelists()

    assert len(cls1) == 1
    assert len(cls2) == 1
    assert cls1[0].description == "Project 1 change"
    assert cls2[0].description == "Project 2 change"


def test_depot_root_extraction():
    registry = VirtualPerforceRegistry.instance()
    client = Client(changelists={}, file_revisions={})
    registry.register("//depot/project", client)

    mappings = [("//depot/project/subdir/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    engine.create_changelist("Test")
    assert len(engine.get_changelists()) == 1


def test_empty_mappings():
    engine = VirtualPerforceEngine([])
    changelists = engine.get_changelists()
    assert len(changelists) == 0


def test_depot_path_with_trailing_slash():
    registry = VirtualPerforceRegistry.instance()
    client = Client(changelists={}, file_revisions={})
    registry.register("//depot/project", client)

    mappings = [("//depot/project/", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    engine.create_changelist("Test")
    assert len(engine.get_changelists()) == 1


def test_depot_path_without_wildcard():
    registry = VirtualPerforceRegistry.instance()
    client = Client(changelists={}, file_revisions={})
    registry.register("//depot/project", client)

    mappings = [("//depot/project", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    engine.create_changelist("Test")
    assert len(engine.get_changelists()) == 1


def test_get_user_creates_new_user():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    user = engine.get_user("johndoe")

    assert user.username == "johndoe"
    assert user.email == "johndoe@example.com"
    assert user.full_name == "johndoe"


def test_get_user_returns_existing_user():
    mappings = [("//depot/project/...", Path("/workspace/project"))]
    engine = VirtualPerforceEngine(mappings)

    user1 = engine.get_user("janedoe")
    user2 = engine.get_user("janedoe")

    assert user1 is user2
    assert user1.username == "janedoe"
