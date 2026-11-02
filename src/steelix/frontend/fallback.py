"""Mixed mode: which statements stay Python source (design §13).

A top-level statement that steelix can not lower is kept as Python source and
executed in place, in the module's globals, by `rt::exec_python!`. A `def`
containing an unsupported construct is kept whole, nested functions included.

The plan is keyed by statement node rather than by module index, so that a
statement inside a native function could be kept the same way later (that
also needs the function's locals to be synchronised with the Python code,
which is why it is not done yet).
"""

from __future__ import annotations

import ast
import re
from dataclasses import dataclass, field

from steelix.diagnostics import UnsupportedError
from steelix.frontend.subset import FunctionLike, Issue


@dataclass
class FallbackPlan:
    python: dict[ast.stmt, list[Issue]] = field(default_factory=dict)
    """Statements kept as Python source, with the constructs that made them so."""

    def is_python(self, stmt: ast.stmt) -> bool:
        return stmt in self.python

    def keep(self, module: ast.Module, error: UnsupportedError) -> bool:
        """Keep the top-level statement where lowering raised `error`.

        Most unsupported constructs are found by the subset check before any
        analysis; a few are only noticed while lowering. Returns `False` if
        the error is not inside a statement that can still be kept.
        """
        stmt = statement_at(module, error.location.line)
        if stmt is None or stmt in self.python:
            return False
        owner = outermost_function_at(stmt, error.location.line)
        self.python[stmt] = [Issue(error.feature, error.location, owner)]
        return True


def first_line(stmt: ast.stmt) -> int:
    """The first line of `stmt`, including its decorators."""
    decorators = getattr(stmt, "decorator_list", [])
    return min([stmt.lineno, *(d.lineno for d in decorators)])


def last_line(stmt: ast.stmt) -> int:
    return stmt.end_lineno or stmt.lineno


def statement_at(module: ast.Module, line: int) -> ast.stmt | None:
    """The top-level statement spanning `line`."""
    for stmt in module.body:
        if first_line(stmt) <= line <= last_line(stmt):
            return stmt
    return None


def outermost_function_at(stmt: ast.stmt, line: int) -> FunctionLike | None:
    """The outermost `def` in `stmt` (the statement itself, or a method of a
    class) spanning `line`."""
    for node in outermost_functions(stmt):
        if first_line(node) <= line <= last_line(node):
            return node
    return None


def outermost_functions(node: ast.AST) -> list[FunctionLike]:
    """The functions in `node` that are not nested in another function:
    `node` itself if it is a `def`, else module level functions in compound
    statements and the methods of classes."""
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
        return [node]
    found: list[FunctionLike] = []
    for child in ast.iter_child_nodes(node):
        if isinstance(child, ast.stmt | ast.excepthandler | ast.match_case):
            found += outermost_functions(child)
    return found


def python_source(lines: list[str], stmts: list[ast.stmt]) -> str:
    """The source text of consecutive top-level statements, ending with a
    newline. A comment after the last statement on its line is included."""
    start, end = first_line(stmts[0]), last_line(stmts[-1])
    # Compound and decorated statements always start a line; a simple
    # statement may follow another one after `;`.
    start_col = stmts[0].col_offset if first_line(stmts[0]) == stmts[0].lineno else 0
    end_col = stmts[-1].end_col_offset
    # Column offsets count UTF-8 bytes.
    text = [line.encode() for line in lines[start - 1 : end]]
    rest = text[-1][end_col:] if end_col is not None else b""
    if rest.strip() and not rest.lstrip().startswith(b"#"):
        # Another statement follows on the same line.
        text[-1] = text[-1][:end_col]
    text[0] = text[0][start_col:]
    return "\n".join(line.decode() for line in text) + "\n"


def source_lines(source: str) -> list[str]:
    """The lines of `source` as the parser counts them."""
    return re.split(r"\r\n|\r|\n", source)
