from typing import Any


class _ContextKey:
    pass


class PrgitError(Exception):
    def __init__(self, message: str, **kwargs: Any) -> None:
        super().__init__(message)
        self.message = message
        self.__context = kwargs

    def get_context(self, key: _ContextKey) -> dict[str, Any]:
        return self.__context

    def get_context_info(self) -> str:
        return ", ".join(f"{k}: {v}" for k, v in self.__context.items())


def create_context_key_for_testing() -> _ContextKey:
    return _ContextKey()


__all__ = ["PrgitError", "create_context_key_for_testing"]
