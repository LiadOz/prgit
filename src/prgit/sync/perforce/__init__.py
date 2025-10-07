from prgit.sync.perforce.abstract_engine import PerforceEngine
from prgit.sync.perforce.real_engine import RealPerforceEngine
from prgit.sync.perforce.types import (
    Changelist,
    ChangelistStatus,
    FileAction,
    FileActionType,
    ShelvedChange,
)
from prgit.sync.perforce.virtual_engine import VirtualPerforceEngine
from prgit.sync.virtual_registry import VirtualRegistry

PerforceState = tuple[dict[int, Changelist], dict[str, dict[int, bytes]]]
VirtualPerforceRegistry = VirtualRegistry[PerforceState]

__all__ = [
    "PerforceEngine",
    "RealPerforceEngine",
    "VirtualPerforceEngine",
    "VirtualPerforceRegistry",
    "Changelist",
    "ChangelistStatus",
    "FileAction",
    "FileActionType",
    "ShelvedChange",
]
