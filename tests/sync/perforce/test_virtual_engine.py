from datetime import datetime

import pytest

from prgit.sync.perforce import (
    Changelist,
    ChangelistStatus,
    FileActionType,
    VirtualPerforceEngine,
    VirtualPerforceRegistry,
)


@pytest.fixture(autouse=True)
def clear_registry():
    VirtualPerforceRegistry.instance().clear()
    yield
    VirtualPerforceRegistry.instance().clear()


def test_create_changelist():
    engine = VirtualPerforceEngine()

    changelist = engine.create_changelist("Test feature")

    assert changelist.number == 1
    assert changelist.description == "Test feature"
    assert changelist.user == "virtualuser"
    assert changelist.client == "virtualclient"
    assert changelist.status == ChangelistStatus.PENDING
    assert len(changelist.files) == 0


def test_create_multiple_changelists():
    engine = VirtualPerforceEngine()

    cl1 = engine.create_changelist("First feature")
    cl2 = engine.create_changelist("Second feature")

    assert cl1.number == 1
    assert cl2.number == 2
    assert cl1.description == "First feature"
    assert cl2.description == "Second feature"


def test_get_changelist():
    engine = VirtualPerforceEngine()
    created = engine.create_changelist("Test feature")

    retrieved = engine.get_changelist(created.number)

    assert retrieved.number == created.number
    assert retrieved.description == created.description
    assert retrieved.status == created.status


def test_get_changelist_not_found():
    engine = VirtualPerforceEngine()

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.get_changelist(999)


def test_get_changelists_all():
    engine = VirtualPerforceEngine()

    cl1 = engine.create_changelist("Feature 1")
    cl2 = engine.create_changelist("Feature 2")
    cl3 = engine.create_changelist("Feature 3")

    changelists = engine.get_changelists()

    assert len(changelists) == 3
    assert changelists[0].number == cl1.number
    assert changelists[1].number == cl2.number
    assert changelists[2].number == cl3.number


def test_get_changelists_filter_by_status():
    engine = VirtualPerforceEngine()

    cl1 = engine.create_changelist("Feature 1")
    cl2 = engine.create_changelist("Feature 2")
    engine.shelve_files(cl2.number, {"//depot/file.txt": b"content"})

    pending = engine.get_changelists(status=ChangelistStatus.PENDING)
    shelved = engine.get_changelists(status=ChangelistStatus.SHELVED)

    assert len(pending) == 1
    assert pending[0].number == cl1.number
    assert len(shelved) == 1
    assert shelved[0].number == cl2.number


def test_get_changelists_max_results():
    engine = VirtualPerforceEngine()

    engine.create_changelist("Feature 1")
    engine.create_changelist("Feature 2")
    engine.create_changelist("Feature 3")
    engine.create_changelist("Feature 4")

    changelists = engine.get_changelists(max_results=2)

    assert len(changelists) == 2
    assert changelists[0].number == 1
    assert changelists[1].number == 2


def test_get_changelists_filter_and_limit():
    engine = VirtualPerforceEngine()

    cl1 = engine.create_changelist("Feature 1")
    cl2 = engine.create_changelist("Feature 2")
    cl3 = engine.create_changelist("Feature 3")

    engine.shelve_files(cl1.number, {"//depot/file1.txt": b"content1"})
    engine.shelve_files(cl2.number, {"//depot/file2.txt": b"content2"})
    engine.shelve_files(cl3.number, {"//depot/file3.txt": b"content3"})

    changelists = engine.get_changelists(status=ChangelistStatus.SHELVED, max_results=2)

    assert len(changelists) == 2
    assert changelists[0].number == 1
    assert changelists[1].number == 2


def test_update_changelist_description():
    engine = VirtualPerforceEngine()

    original = engine.create_changelist("Original description")
    updated = engine.update_changelist_description(
        original.number, "Updated description"
    )

    assert updated.number == original.number
    assert updated.description == "Updated description"
    assert updated.user == original.user
    assert updated.client == original.client
    assert updated.status == original.status

    retrieved = engine.get_changelist(original.number)
    assert retrieved.description == "Updated description"


def test_update_changelist_description_not_found():
    engine = VirtualPerforceEngine()

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.update_changelist_description(999, "New description")


def test_shelve_files_new_files():
    engine = VirtualPerforceEngine()

    cl = engine.create_changelist("Test shelve")
    files = {
        "//depot/project/file1.py": b"print('hello')",
        "//depot/project/file2.py": b"print('world')",
    }

    shelved = engine.shelve_files(cl.number, files)

    assert shelved.changelist.number == cl.number
    assert shelved.changelist.status == ChangelistStatus.SHELVED
    assert len(shelved.changelist.files) == 2
    assert shelved.files == files

    for file_action in shelved.changelist.files:
        assert file_action.action == FileActionType.ADD


def test_shelve_files_changelist_not_found():
    engine = VirtualPerforceEngine()

    with pytest.raises(ValueError, match="Changelist 999 not found"):
        engine.shelve_files(999, {"//depot/file.txt": b"content"})


def test_shelve_files_updates_status():
    engine = VirtualPerforceEngine()

    cl = engine.create_changelist("Test")
    assert cl.status == ChangelistStatus.PENDING

    engine.shelve_files(cl.number, {"//depot/file.txt": b"content"})

    updated = engine.get_changelist(cl.number)
    assert updated.status == ChangelistStatus.SHELVED


def test_get_changelist_file_content():
    engine = VirtualPerforceEngine()

    cl = engine.create_changelist("Test")
    files = {"//depot/file.py": b"print('test')"}
    engine.shelve_files(cl.number, files)

    engine._file_revisions["//depot/file.py"] = {1: b"print('test')"}

    content = engine.get_changelist_file_content("//depot/file.py", 1)

    assert content == b"print('test')"


def test_get_changelist_file_content_file_not_found():
    engine = VirtualPerforceEngine()

    with pytest.raises(ValueError, match="File //depot/missing.txt not found"):
        engine.get_changelist_file_content("//depot/missing.txt", 1)


def test_get_changelist_file_content_revision_not_found():
    engine = VirtualPerforceEngine()

    engine._file_revisions["//depot/file.py"] = {1: b"content"}

    with pytest.raises(ValueError, match="Revision 2 of //depot/file.py not found"):
        engine.get_changelist_file_content("//depot/file.py", 2)


def test_virtual_perforce_registry_register_and_get():
    registry = VirtualPerforceRegistry.instance()

    changelists = {
        1: Changelist(
            number=1,
            description="Test",
            user="user",
            client="client",
            timestamp=datetime.now(),
            status=ChangelistStatus.SUBMITTED,
            files=[],
        )
    }
    file_revisions = {"//depot/file.py": {1: b"content"}}

    registry.register("test-state", (changelists, file_revisions))

    retrieved_cl, retrieved_files = registry.get("test-state")

    assert retrieved_cl == changelists
    assert retrieved_files == file_revisions


def test_virtual_perforce_registry_not_found():
    registry = VirtualPerforceRegistry.instance()

    with pytest.raises(ValueError, match="Identifier 'missing' not found in registry"):
        registry.get("missing")


def test_virtual_perforce_registry_unregister():
    registry = VirtualPerforceRegistry.instance()

    changelists = {}
    file_revisions = {}

    registry.register("test-state", (changelists, file_revisions))
    registry.unregister("test-state")

    with pytest.raises(ValueError):
        registry.get("test-state")


def test_virtual_perforce_registry_singleton():
    registry1 = VirtualPerforceRegistry.instance()
    registry2 = VirtualPerforceRegistry.instance()

    assert registry1 is registry2


def test_empty_get_changelists():
    engine = VirtualPerforceEngine()

    changelists = engine.get_changelists()

    assert len(changelists) == 0


def test_shelve_files_preserves_changelist_metadata():
    engine = VirtualPerforceEngine()

    original = engine.create_changelist("Original description")
    original_time = original.timestamp

    shelved = engine.shelve_files(original.number, {"//depot/file.txt": b"content"})

    assert shelved.changelist.description == "Original description"
    assert shelved.changelist.timestamp == original_time
    assert shelved.changelist.user == "virtualuser"
    assert shelved.changelist.client == "virtualclient"


def test_shelve_files_edit_existing():
    engine = VirtualPerforceEngine()

    engine._file_revisions["//depot/file.py"] = {1: b"old content"}

    cl = engine.create_changelist("Update file")
    shelved = engine.shelve_files(cl.number, {"//depot/file.py": b"new content"})

    edit_actions = [
        f for f in shelved.changelist.files if f.action == FileActionType.EDIT
    ]
    assert len(edit_actions) == 1
    assert edit_actions[0].depot_path == "//depot/file.py"
