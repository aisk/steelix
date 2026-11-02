"""Diagnostics reported while transpiling, carrying Python source locations."""

import ast
from dataclasses import dataclass
from enum import Enum


class Severity(Enum):
    ERROR = "error"
    WARNING = "warning"


@dataclass(frozen=True)
class SourceLocation:
    path: str
    line: int
    col: int

    @classmethod
    def of(cls, path: str, node: ast.AST) -> SourceLocation:
        return cls(path, getattr(node, "lineno", 0), getattr(node, "col_offset", 0))

    def __str__(self) -> str:
        return f"{self.path}:{self.line}:{self.col + 1}"


@dataclass(frozen=True)
class Diagnostic:
    severity: Severity
    message: str
    location: SourceLocation

    def __str__(self) -> str:
        return f"{self.location}: {self.severity.value}: {self.message}"


class UnsupportedError(Exception):
    """Raised when a Python construct has no lowering yet."""

    def __init__(self, feature: str, location: SourceLocation) -> None:
        super().__init__(f"{location}: unsupported: {feature}")
        self.feature = feature
        self.location = location
