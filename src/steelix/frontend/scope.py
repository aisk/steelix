"""Scope analysis based on the standard library `symtable` module.

`symtable` runs CPython's own scope analysis, so local / global / free
classification matches the interpreter exactly.
"""

from __future__ import annotations

import ast
import symtable
from dataclasses import dataclass

from steelix.diagnostics import SourceLocation, UnsupportedError


@dataclass(frozen=True)
class FunctionScope:
    name: str
    params: tuple[str, ...]
    locals: frozenset[str]
    """Local names including parameters."""
    globals: frozenset[str]
    """Names this function reads from module globals or builtins."""

    def is_local(self, name: str) -> bool:
        return name in self.locals


@dataclass
class ModuleScopes:
    functions: dict[ast.FunctionDef, FunctionScope]

    def of(self, fn: ast.FunctionDef) -> FunctionScope:
        return self.functions[fn]


def analyze_scopes(module: ast.Module, source: str, filename: str) -> ModuleScopes:
    table = symtable.symtable(source, filename, "exec")
    children: dict[tuple[str, int], symtable.SymbolTable] = {}
    for child in table.get_children():
        if child.get_type() == symtable.SymbolTableType.FUNCTION:
            children[(child.get_name(), child.get_lineno())] = child

    functions: dict[ast.FunctionDef, FunctionScope] = {}
    for stmt in module.body:
        if not isinstance(stmt, ast.FunctionDef):
            continue
        child = children[(stmt.name, stmt.lineno)]
        assert isinstance(child, symtable.Function)
        functions[stmt] = _function_scope(child, stmt, filename)
    return ModuleScopes(functions)


def _function_scope(
    table: symtable.Function, node: ast.FunctionDef, filename: str
) -> FunctionScope:
    if table.get_frees() or table.get_nonlocals():
        raise UnsupportedError("closure", SourceLocation.of(filename, node))
    if table.has_children():
        raise UnsupportedError("nested scope", SourceLocation.of(filename, node))
    locals_ = set(table.get_locals())
    globals_ = set()
    for symbol in table.get_symbols():
        if symbol.is_declared_global():
            raise UnsupportedError("global statement", SourceLocation.of(filename, node))
        if symbol.is_global():
            globals_.add(symbol.get_name())
    return FunctionScope(
        name=table.get_name(),
        params=tuple(table.get_parameters()),
        locals=frozenset(locals_),
        globals=frozenset(globals_),
    )
