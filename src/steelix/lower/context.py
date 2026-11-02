"""Lowering state shared by the expression and statement lowerings."""

from __future__ import annotations

import ast
from dataclasses import dataclass, field

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.bindings import LocalsInfo, ModuleBindings
from steelix.frontend.callgraph import CallGraph
from steelix.frontend.scope import FunctionNode, FunctionScope, ModuleScopes
from steelix.lower.names import FUNCTION_RESERVED, RUST_KEYWORDS, Renamer
from steelix.rustast import nodes as rs


@dataclass
class ModuleContext:
    filename: str
    bindings: ModuleBindings
    callgraph: CallGraph
    fn_names: Renamer
    """Rust names of the module level functions."""
    scopes: ModuleScopes
    comments: dict[ast.stmt, list[str]] = field(default_factory=dict)
    """Python comments to emit before each statement."""
    future_annotations: bool = False
    """`from __future__ import annotations`: annotations are strings."""
    frozen: dict[FunctionScope, set[str]] = field(default_factory=dict)
    names: dict[FunctionScope, Renamer] = field(default_factory=dict)
    """Rust names of each lowered function's parameters and locals."""
    """Cells of each function that nested functions receive by value."""
    uses: set[str] = field(default_factory=set)
    """`use` paths the generated module needs besides the fixed ones."""

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return UnsupportedError(feature, SourceLocation.of(self.filename, node))


@dataclass
class FunctionInfo:
    """What lowering needs to know about the function being lowered."""

    node: FunctionNode
    scope: FunctionScope
    locals: LocalsInfo
    names: Renamer
    """Rust names of parameters, captured variables and locals."""
    by_value: set[str] = field(default_factory=set)
    """Captured variables received by value rather than as `rt::Cell`."""

    def is_cell(self, name: str) -> bool:
        """Whether `name` is a `rt::Cell` here: an owned cell that nested
        functions may rebind, or a captured one."""
        if self.scope.is_free(name):
            return name not in self.by_value
        return name in self.scope.cells and name not in self.locals.frozen

    @property
    def varargs(self) -> str | None:
        args = self.node.args
        return args.vararg.arg if args.vararg is not None else None

    @property
    def varkw(self) -> str | None:
        args = self.node.args
        return args.kwarg.arg if args.kwarg is not None else None


@dataclass
class Context:
    """Lowering state for one Rust function body: a `def` or `exec_module`."""

    module: ModuleContext
    flow: LocalsInfo
    """Reachability (and, in functions, the declarations of locals)."""
    function: FunctionInfo | None = None
    """`None` while lowering module level code."""
    statement_index: int | None = None
    """Index of the current top-level statement, for module level code."""
    uses_py: bool = False
    uses_globals: bool = False
    items: list[rs.Fn] = field(default_factory=list)
    """Nested functions waiting to be declared before the current statement."""
    item_names: set[str] = field(default_factory=set)
    _temps: set[str] = field(default_factory=set)
    _labels: set[str] = field(default_factory=set)

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
        """A fresh Rust name for a temporary that can not clash with locals,
        functions or names the generated code relies on."""
        taken = set(self.module.fn_names.mapping.values()) | FUNCTION_RESERVED | self.item_names
        if self.function is not None:
            taken.update(self.function.names.mapping.values())
        if not base.isidentifier() or not base.isascii():
            base = "value"
        if base in RUST_KEYWORDS or base in ("_", "self", "Self", "crate", "super"):
            base += "_"
        name = base
        n = 1
        while name in taken or name in self._temps:
            n += 1
            name = f"{base}{n}"
        self._temps.add(name)
        return name

    def item_name(self, base: str, allowed: str | None = None) -> str:
        """A fresh name for a nested `fn` item, which must not be shadowed by
        locals or temporaries of the enclosing function (`allowed` is a local
        declared by the very statement that defines the item)."""
        taken = set(self.module.fn_names.mapping.values()) | FUNCTION_RESERVED | self._temps
        taken |= self.item_names
        if self.function is not None:
            taken.update(v for v in self.function.names.mapping.values() if v != allowed)
        name = base
        n = 1
        while name in taken:
            n += 1
            name = f"{base}{n}" if base == "lambda" else name + "_"
        self.item_names.add(name)
        return name

    def label(self, base: str) -> str:
        """A fresh loop or block label such as `'for_else`."""
        name = base
        n = 1
        while name in self._labels:
            n += 1
            name = f"{base}{n}"
        self._labels.add(name)
        return "'" + name

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return self.module.unsupported(feature, node)
