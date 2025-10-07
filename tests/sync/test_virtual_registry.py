from concurrent.futures import ThreadPoolExecutor

import pytest

from prgit.sync.virtual_registry import VirtualRegistry


def test_singleton_instance():
    registry1 = VirtualRegistry[str].instance()
    registry2 = VirtualRegistry[str].instance()

    assert registry1 is registry2


def test_type_specific_singleton_isolation():
    str_registry = VirtualRegistry[str].instance()
    int_registry = VirtualRegistry[int].instance()

    assert str_registry is not int_registry


def test_register_and_get():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    registry.register("key1", "value1")
    registry.register("key2", "value2")

    assert registry.get("key1") == "value1"
    assert registry.get("key2") == "value2"


def test_get_nonexistent():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    with pytest.raises(ValueError, match="Identifier 'missing' not found in registry"):
        registry.get("missing")


def test_unregister():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    registry.register("key1", "value1")
    registry.unregister("key1")

    with pytest.raises(ValueError):
        registry.get("key1")


def test_unregister_nonexistent():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    registry.unregister("nonexistent")


def test_clear():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    registry.register("key1", "value1")
    registry.register("key2", "value2")

    registry.clear()

    with pytest.raises(ValueError):
        registry.get("key1")
    with pytest.raises(ValueError):
        registry.get("key2")


def test_complex_type():
    ComplexType = tuple[dict[int, str], list[str]]
    registry = VirtualRegistry[ComplexType].instance()
    registry.clear()

    data = ({1: "one", 2: "two"}, ["a", "b", "c"])
    registry.register("complex", data)

    retrieved = registry.get("complex")
    assert retrieved == data


def test_thread_safety():
    registry = VirtualRegistry[int].instance()
    registry.clear()

    def register_data(thread_id: int) -> None:
        for i in range(100):
            key = f"thread_{thread_id}_item_{i}"
            registry.register(key, thread_id * 1000 + i)

    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(register_data, i) for i in range(10)]
        for future in futures:
            future.result()

    for thread_id in range(10):
        for i in range(100):
            key = f"thread_{thread_id}_item_{i}"
            assert registry.get(key) == thread_id * 1000 + i


def test_concurrent_clear():
    registry = VirtualRegistry[str].instance()
    registry.clear()

    registry.register("key1", "value1")

    def clear_registry() -> None:
        registry.clear()

    def try_get() -> str | None:
        try:
            return registry.get("key1")
        except ValueError:
            return None

    with ThreadPoolExecutor(max_workers=5) as executor:
        futures = [
            executor.submit(clear_registry if i % 2 == 0 else try_get)
            for i in range(100)
        ]
        for future in futures:
            future.result()


def test_multiple_types_coexist():
    str_registry = VirtualRegistry[str].instance()
    int_registry = VirtualRegistry[int].instance()
    list_registry = VirtualRegistry[list[str]].instance()

    str_registry.clear()
    int_registry.clear()
    list_registry.clear()

    str_registry.register("str_key", "string_value")
    int_registry.register("int_key", 42)
    list_registry.register("list_key", ["a", "b", "c"])

    assert str_registry.get("str_key") == "string_value"
    assert int_registry.get("int_key") == 42
    assert list_registry.get("list_key") == ["a", "b", "c"]

    str_registry.clear()
    assert int_registry.get("int_key") == 42
    assert list_registry.get("list_key") == ["a", "b", "c"]
