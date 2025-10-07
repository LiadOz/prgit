import pytest

from prgit.exceptions import PrgitError, create_context_key_for_testing


def test_prgit_error_with_message_only() -> None:
    error = PrgitError("Something went wrong")
    assert error.message == "Something went wrong"
    assert str(error) == "Something went wrong"


def test_prgit_error_with_message_and_kwargs() -> None:
    error = PrgitError("Operation failed", operation="fetch", ref="main")
    assert error.message == "Operation failed"
    assert str(error) == "Operation failed"


def test_get_context_info_returns_formatted_string() -> None:
    error = PrgitError("Division by zero", numerator=10, denominator=0)
    context_info = error.get_context_info()
    assert "numerator: 10" in context_info
    assert "denominator: 0" in context_info


def test_get_context_info_with_empty_context() -> None:
    error = PrgitError("Simple error")
    assert error.get_context_info() == ""


def test_get_context_with_key_returns_dict() -> None:
    error = PrgitError("Error", key1="value1", key2="value2")
    key = create_context_key_for_testing()
    context = error.get_context(key)
    assert context == {"key1": "value1", "key2": "value2"}


def test_get_context_without_key_raises_type_error() -> None:
    error = PrgitError("Error", key1="value1")
    with pytest.raises(TypeError):
        error.get_context()


def test_subclass_inherits_behavior() -> None:
    class CustomError(PrgitError):
        pass

    error = CustomError("Custom error", custom_field="custom_value")
    assert error.message == "Custom error"
    assert error.get_context_info() == "custom_field: custom_value"

    key = create_context_key_for_testing()
    context = error.get_context(key)
    assert context == {"custom_field": "custom_value"}


def test_exception_chaining_preserves_cause() -> None:
    original = ValueError("Original error")
    try:
        raise PrgitError("Wrapped error", detail="some detail") from original
    except PrgitError as e:
        assert e.__cause__ is original
        assert isinstance(e.__cause__, ValueError)


def test_context_preserves_types() -> None:
    error = PrgitError(
        "Error",
        string_val="text",
        int_val=42,
        list_val=[1, 2, 3],
        dict_val={"nested": "value"},
    )
    key = create_context_key_for_testing()
    context = error.get_context(key)
    assert context["string_val"] == "text"
    assert context["int_val"] == 42
    assert context["list_val"] == [1, 2, 3]
    assert context["dict_val"] == {"nested": "value"}


def test_context_with_none_values() -> None:
    error = PrgitError("Error", value=None)
    key = create_context_key_for_testing()
    context = error.get_context(key)
    assert context == {"value": None}
    assert error.get_context_info() == "value: None"
