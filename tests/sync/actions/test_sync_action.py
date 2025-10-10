from pathlib import Path

import pytest

from prgit.sync.actions import (
    SyncAction,
    SyncActionError,
    SyncConfigurationError,
    SyncExecutionError,
)
from prgit.sync.git import VirtualGitEngine
from prgit.sync.perforce import VirtualPerforceEngine


class DummyAction(SyncAction):
    def __init__(
        self, git: VirtualGitEngine, perforce: VirtualPerforceEngine, value: int
    ) -> None:
        super().__init__(git, perforce)
        self._value = value

    def perform(self) -> None:
        pass


class FailingAction(SyncAction):
    def __init__(self, git: VirtualGitEngine, perforce: VirtualPerforceEngine) -> None:
        super().__init__(git, perforce)

    def perform(self) -> None:
        raise SyncExecutionError(
            "Test execution error", action=self, operation="test_operation"
        )


class ConfigErrorAction(SyncAction):
    def __init__(
        self, git: VirtualGitEngine, perforce: VirtualPerforceEngine, value: int
    ) -> None:
        super().__init__(git, perforce)
        if value < 0:
            raise SyncConfigurationError(
                "Value must be non-negative",
                action=self,
                parameter="value",
                value=value,
            )
        self._value = value

    def perform(self) -> None:
        pass


def test_sync_action_initialization() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/fake/repo"))
    perforce = VirtualPerforceEngine([("//depot/test/...", Path("/fake/workspace"))])

    action = DummyAction(git, perforce, value=42)

    assert action._git is git
    assert action._perforce is perforce
    assert action._value == 42


def test_sync_action_perform() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/fake/repo"))
    perforce = VirtualPerforceEngine([("//depot/test/...", Path("/fake/workspace"))])

    action = DummyAction(git, perforce, value=42)
    action.perform()


def test_sync_configuration_error() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/fake/repo"))
    perforce = VirtualPerforceEngine([("//depot/test/...", Path("/fake/workspace"))])

    with pytest.raises(SyncConfigurationError) as exc_info:
        ConfigErrorAction(git, perforce, value=-1)

    error = exc_info.value
    assert error.message == "Value must be non-negative"
    assert error.parameter == "value"
    assert isinstance(error.action, ConfigErrorAction)
    assert (
        error.get_context_info()
        == "action_name: ConfigErrorAction, parameter: value, value: -1"
    )


def test_sync_execution_error() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/fake/repo"))
    perforce = VirtualPerforceEngine([("//depot/test/...", Path("/fake/workspace"))])

    action = FailingAction(git, perforce)

    with pytest.raises(SyncExecutionError) as exc_info:
        action.perform()

    error = exc_info.value
    assert error.message == "Test execution error"
    assert error.operation == "test_operation"
    assert error.action is action
    assert (
        error.get_context_info()
        == "action_name: FailingAction, operation: test_operation"
    )


def test_sync_action_error_captures_action() -> None:
    git = VirtualGitEngine()
    git.init_repo(Path("/fake/repo"))
    perforce = VirtualPerforceEngine([("//depot/test/...", Path("/fake/workspace"))])

    action = DummyAction(git, perforce, value=42)

    error = SyncActionError("Test error", action=action, extra="data")

    assert error.action is action
    assert error.message == "Test error"
    assert "action_name: DummyAction" in error.get_context_info()
    assert "extra: data" in error.get_context_info()
