from typing import Any

class SettingsManager:
    def __init__(self, name: str, settings_directory: str | None = None) -> None:
        ...

    def read(self) -> None:
        ...

    def commit(self) -> None:
        ...

    def getSetting(self, key: str, default: Any = None) -> Any:
        ...

    def setSetting(self, key: str, value: Any) -> None:
        ...
