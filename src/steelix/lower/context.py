"""Lowering state shared by the expression and statement lowerings."""

from __future__ import annotations

import ast
from dataclasses import dataclass, field

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.bindings import LocalsInfo, ModuleBindings
from steelix.frontend.callgraph import CallGraph
from steelix.frontend.scope import FunctionScope
from steelix.lower.names import Renamer


@dataclass
class ModuleContext:
    filename: str
    bindings: ModuleBindings
    callgraph: CallGraph
    fn_names: Renamer
    """Rust names of the module level functions."""
    param_names: dict[ast.FunctionDef, list[str]] = field(default_factory=dict)
    """Rust names of each function's parameters."""
    uses: set[str] = field(default_factory=set)
    """`use` paths the generated module needs besides the fixed ones."""

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return UnsupportedError(feature, SourceLocation.of(self.filename, node))


@dataclass
class FunctionInfo:
    """What lowering needs to know about the function being lowered."""

    node: ast.FunctionDef
    scope: FunctionScope
    locals: LocalsInfo
    names: Renamer
    """Rust names of parameters and locals."""


@dataclass
class Context:
    """Lowering state for one Rust function body: a `def` or `exec_module`."""

    module: ModuleContext
    function: FunctionInfo | None = None
    """`None` while lowering module level code."""
    statement_index: int | None = None
    """Index of the current top-level statement, for module level code."""
    uses_py: bool = False
    uses_globals: bool = False
    _temps: set[str] = field(default_factory=set)

    @property
    def at_module_level(self) -> bool:
        return self.function is None

    def py(self) -> str:
        self.uses_py = True
        return "py"

    def globals(self) -> str:
        self.uses_globals = True
        return "globals"

    def temp(self, base: str) -> str:
        """A fresh Rust name for a temporary that can not clash with locals."""
        taken = set(self.module.fn_names.mapping.values())
        if self.function is not None:
            taken.update(self.function.names.mapping.values())
        name = base
        n = 1
        while name in taken or name in self._temps:
            n += 1
            name = f"{base}{n}"
        self._temps.add(name)
        return name

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return self.module.unsupported(feature, node)
