from typing import TYPE_CHECKING, Any

from prgit.exceptions import PrgitError

if TYPE_CHECKING:
    from prgit.sync.actions.base import SyncAction


class SyncActionError(PrgitError):
    def __init__(self, message: str, action: "SyncAction", **kwargs: Any) -> None:
        super().__init__(message, action_name=action.__class__.__name__, **kwargs)
        self.action = action


class SyncConfigurationError(SyncActionError):
    def __init__(
        self, message: str, action: "SyncAction", parameter: str, **kwargs: Any
    ) -> None:
        super().__init__(message, action=action, parameter=parameter, **kwargs)
        self.parameter = parameter


class SyncExecutionError(SyncActionError):
    def __init__(
        self, message: str, action: "SyncAction", operation: str, **kwargs: Any
    ) -> None:
        super().__init__(message, action=action, operation=operation, **kwargs)
        self.operation = operation
