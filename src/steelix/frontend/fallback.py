"""Mixed mode: which statements stay Python source (design §13).

A module level statement that steelix can not lower is kept as Python source
and executed in place, in the module's globals, by `rt::exec_python!`. A `def`
containing an unsupported construct is kept whole, nested functions included.
A class is split like a compound statement (method level mixed mode): only
the methods and other statements of its body that contain unsupported
constructs are kept, and run in the class namespace.

Module level `if`, `for` and `while` statements are split: when only some of
the statements in their bodies are unsupported, only those are kept and the
compound statement itself stays native. Module scope is the globals dict, so
the kept code needs no other state. A kept statement must not `break` or
`continue` a native loop, though (that is a `SyntaxError` on its own), so a
loop with such a statement is kept whole.

The plan is keyed by statement node. Statements inside a native function are
not kept separately yet: that also needs the function's locals to be
synchronised with the Python code.
"""

from __future__ import annotations

import ast
import re
from collections.abc import Iterator
from dataclasses import dataclass, field

from steelix.diagnostics import UnsupportedError
from steelix.frontend.subset import WHOLE_CLASS_FEATURES, FunctionLike, Issue


@dataclass
class FallbackPlan:
    python: dict[ast.stmt, list[Issue]] = field(default_factory=dict)
    """Statements kept as Python source, with the constructs that made them
    so. They are module level statements or, when a compound statement is
    split, statements in its body; never one inside another."""

    def is_python(self, stmt: ast.stmt) -> bool:
        return stmt in self.python

    def add(self, stmt: ast.stmt, issues: list[Issue]) -> None:
        """Keep the parts of the module level statement `stmt` that contain
        `issues` (`split` decides which)."""
        self.python.update(split(stmt, issues))

    def within(self, stmt: ast.stmt) -> list[ast.stmt]:
        """The kept statements in the module level statement `stmt`, in
        source order."""
        nodes = {id(node) for node in ast.walk(stmt)}
        kept = [s for s in self.python if id(s) in nodes]
        return sorted(kept, key=first_line)

    def keep(self, module: ast.Module, error: UnsupportedError) -> bool:
        """Keep the statement where lowering raised `error`.

        Most unsupported constructs are found by the subset check before any
        analysis; a few are only noticed while lowering. Returns `False` if
        the error is not inside a statement that can still be kept.
        """
        stmt = statement_at(module, error.location.line)
        if stmt is None:
            return False
        kept = self.within(stmt)
        if any(_contains(s, error.location.line, error.location.col) for s in kept):
            return False
        owner = outermost_function_at(stmt, error.location.line)
        issues = [Issue(error.feature, error.location, owner)]
        for s in kept:
            issues += self.python.pop(s)
        self.add(stmt, issues)
        return True


_SPLIT = (ast.If, ast.For, ast.While)


def split(stmt: ast.stmt, issues: list[Issue]) -> dict[ast.stmt, list[Issue]]:
    """The statements to keep as Python for the unsupported constructs
    `issues` in `stmt`: `stmt` itself, or for an `if`, `for` or `while`
    whose header is supported, the statements of its bodies that contain
    them, split further the same way."""
    if isinstance(stmt, ast.ClassDef):
        return _split_class(stmt, issues)
    if not isinstance(stmt, _SPLIT):
        return {stmt: issues}
    children = [*stmt.body, *stmt.orelse]
    by_child: dict[ast.stmt, list[Issue]] = {}
    for issue in issues:
        line, col = issue.location.line, issue.location.col
        child = next((c for c in children if _contains(c, line, col)), None)
        if child is None:
            return {stmt: issues}  # in the test, the target or the iterable
        by_child.setdefault(child, []).append(issue)
    kept: dict[ast.stmt, list[Issue]] = {}
    for child, found in by_child.items():
        kept.update(split(child, found))
    if isinstance(stmt, ast.For | ast.While) and any(_jumps_out(s) for s in kept):
        return {stmt: issues}
    return kept


def _split_class(cls: ast.ClassDef, issues: list[Issue]) -> dict[ast.stmt, list[Issue]]:
    """Method level mixed mode: the class is created natively and only the
    statements of its body that contain `issues` (methods, or other
    statements, split further) are kept, and run in the class namespace.

    The whole class is kept when an issue is in its header (decorators,
    bases, keywords), changes how the rest of the body is compiled
    (`WHOLE_CLASS_FEATURES`), or when a kept statement has annotations, which
    belong to the class's `__annotate__`."""
    by_child: dict[ast.stmt, list[Issue]] = {}
    for issue in issues:
        if issue.feature in WHOLE_CLASS_FEATURES or issue.feature == "type parameters":
            return {cls: issues}
        line, col = issue.location.line, issue.location.col
        child = next((c for c in cls.body if _contains(c, line, col)), None)
        if child is None:
            return {cls: issues}
        by_child.setdefault(child, []).append(issue)
    kept: dict[ast.stmt, list[Issue]] = {}
    for child, found in by_child.items():
        kept.update(split(child, found))
    for stmt in kept:
        if any(isinstance(node, ast.AnnAssign) for node in _class_body_nodes(stmt)):
            return {cls: issues}
    return kept


def _class_body_nodes(stmt: ast.AST) -> Iterator[ast.AST]:
    """The nodes of a class body statement outside of nested functions and
    classes (where annotations belong to the class)."""
    yield stmt
    if isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef | ast.Lambda):
        return
    for child in ast.iter_child_nodes(stmt):
        yield from _class_body_nodes(child)  # type: ignore[arg-type]


def _contains(stmt: ast.stmt, line: int, col: int) -> bool:
    """Whether the position `line:col` is inside `stmt` (or its decorators)."""
    start = (first_line(stmt), 0 if first_line(stmt) < stmt.lineno else stmt.col_offset)
    end = (last_line(stmt), stmt.end_col_offset if stmt.end_col_offset is not None else 10**9)
    return start <= (line, col) <= end


def _jumps_out(stmt: ast.AST) -> bool:
    """Whether `stmt` contains a `break` or `continue` of a loop around it."""
    return next(loop_jumps(stmt), None) is not None


def loop_jumps(node: ast.AST) -> Iterator[ast.Break | ast.Continue]:
    """The `break` and `continue` statements in `node` that leave a loop
    around it, in source order."""
    match node:
        case ast.Break() | ast.Continue():
            yield node
            return
        case ast.For(orelse=orelse) | ast.AsyncFor(orelse=orelse) | ast.While(orelse=orelse):
            # Jumps in the body belong to this loop, those in `else` to an outer one.
            for stmt in orelse:
                yield from loop_jumps(stmt)
            return
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
            return
    for child in ast.iter_child_nodes(node):
        if isinstance(child, ast.stmt | ast.excepthandler | ast.match_case):
            yield from loop_jumps(child)


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


def python_runs(stmts: list[ast.stmt], plan: FallbackPlan) -> Iterator[ast.stmt | list[ast.stmt]]:
    """`stmts` with the statements kept as Python grouped into the runs that
    one `rt::exec_python!` executes: a compound statement (`def`, `class`,
    `try`, ...) on its own, consecutive simple statements together."""
    run: list[ast.stmt] = []
    for stmt in stmts:
        if not plan.is_python(stmt):
            if run:
                yield run
                run = []
            yield stmt
            continue
        if run and (_is_compound(stmt) or _is_compound(run[-1])):
            yield run
            run = []
        run.append(stmt)
    if run:
        yield run


COMPOUND = (
    ast.FunctionDef,
    ast.AsyncFunctionDef,
    ast.ClassDef,
    ast.If,
    ast.For,
    ast.AsyncFor,
    ast.While,
    ast.With,
    ast.AsyncWith,
    ast.Try,
    ast.TryStar,
    ast.Match,
)


def _is_compound(stmt: ast.stmt) -> bool:
    return isinstance(stmt, COMPOUND)


def python_source(lines: list[str], stmts: list[ast.stmt], pad: bool = False) -> str:
    """The source text of consecutive statements, ending with a newline. A
    comment after the last statement on its line is included.

    Statements in the body of a compound statement keep their indentation
    (`rt::exec_python` compiles them in an `if True:` block, so that column
    offsets stay those of the file). Statements of a class body are always
    indented (`pad`)."""
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
    if text[0][:start_col].strip():
        # After `;`: only the statement itself, or with `pad` spaces in
        # place of what comes before it, which keeps its column.
        text[0] = (b" " * start_col if pad else b"") + text[0][start_col:]
    return "\n".join(line.decode() for line in text) + "\n"


def source_lines(source: str) -> list[str]:
    """The lines of `source` as the parser counts them."""
    return re.split(r"\r\n|\r|\n", source)


def module_annotations(module: ast.Module) -> list[ast.AnnAssign]:
    """The annotated assignments in module scope, in source order: at the
    top level and in the bodies of compound statements, not in functions or
    classes."""
    found: list[ast.AnnAssign] = []

    def visit(stmts: list[ast.stmt]) -> None:
        for stmt in stmts:
            if isinstance(stmt, ast.AnnAssign):
                found.append(stmt)
            elif not isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                for child in ast.iter_child_nodes(stmt):
                    if isinstance(child, ast.stmt):
                        visit([child])
                    elif isinstance(child, ast.excepthandler | ast.match_case):
                        visit(child.body)

    visit(module.body)
    return found


def annotate_source(annotations: list[tuple[int, str]]) -> str:
    """The module source `rt::module_annotate` compiles: each `name:
    annotation` on its line (mirrors the runtime)."""
    out = ""
    current = 1
    for line, text in annotations:
        if line > current:
            out += "\n" * (line - current)
            current = line
        elif out:
            out += "; "
        out += text
        current += text.count("\n")
    return out + "\n"
