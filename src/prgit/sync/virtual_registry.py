import threading
from typing import Any, Generic, TypeVar, cast

T = TypeVar("T")

_instances: dict[int, "VirtualRegistry[Any]"] = {}
_instances_lock: threading.Lock = threading.Lock()
_type_cache: dict[int, type["VirtualRegistry[Any]"]] = {}


class VirtualRegistry(Generic[T]):
    _type_id: int = 0

    def __init__(self) -> None:
        self._data: dict[str, T] = {}
        self._data_lock: threading.Lock = threading.Lock()

    @classmethod
    def __class_getitem__(cls, item: Any) -> type["VirtualRegistry[T]"]:
        type_id = id(item)

        if type_id in _type_cache:
            return cast(type["VirtualRegistry[T]"], _type_cache[type_id])

        class SpecificRegistry(VirtualRegistry):
            _type_id = type_id

        _type_cache[type_id] = SpecificRegistry
        return cast(type["VirtualRegistry[T]"], SpecificRegistry)

    @classmethod
    def instance(cls) -> "VirtualRegistry[T]":
        if cls._type_id not in _instances:
            with _instances_lock:
                if cls._type_id not in _instances:
                    _instances[cls._type_id] = cls()
        return cast("VirtualRegistry[T]", _instances[cls._type_id])

    def register(self, identifier: str, data: T) -> None:
        with self._data_lock:
            self._data[identifier] = data

    def unregister(self, identifier: str) -> None:
        with self._data_lock:
            self._data.pop(identifier, None)

    def get(self, identifier: str) -> T:
        with self._data_lock:
            if identifier not in self._data:
                raise ValueError(f"Identifier '{identifier}' not found in registry")
            return self._data[identifier]

    def clear(self) -> None:
        with self._data_lock:
            self._data.clear()
