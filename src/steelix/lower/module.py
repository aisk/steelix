"""Lower a whole Python module into one Rust module file.

The Rust module exposes `exec_module(m)`, which runs the Python module body
against an existing module object. It does not know whether it runs inside an
extension module or as `__main__` of an executable; the emitted `lib.rs` /
`main.rs` decides that.

In mixed mode (the default) top-level statements that can not be lowered are
kept as Python source and run in place by `rt::exec_python!` (design §13).
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.bindings import analyze_module_bindings, analyze_module_flow
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.desugar import desugar
from steelix.frontend.fallback import (
    FallbackPlan,
    first_line,
    last_line,
    python_source,
    source_lines,
)
from steelix.frontend.parse import ParsedModule
from steelix.frontend.scope import analyze_scopes, has_future_annotations
from steelix.frontend.subset import Issue, find_unsupported
from steelix.lower.context import Context, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.function import attach_comments, bind_py, lower_function
from steelix.lower.names import CONTEXT, MODULE_RESERVED, Renamer
from steelix.lower.stmt import StmtLowering
from steelix.rustast import nodes as rs


@dataclass
class LoweredModule:
    name: str
    file: rs.File
    plan: FallbackPlan
    """The statements kept as Python source."""


def lower_module(parsed: ParsedModule, *, fallback: bool = True) -> LoweredModule:
    """Lower `parsed`. Without `fallback`, the first unsupported construct
    raises `UnsupportedError` instead of being kept as Python."""
    tree = parsed.tree
    desugar(tree)
    plan = FallbackPlan(find_unsupported(tree, parsed.filename))
    _keep_redefinitions(tree, plan, parsed.filename)
    if not fallback:
        for stmt in tree.body:
            if plan.is_python(stmt):
                raise plan.python[stmt][0].error()
    while True:
        try:
            return _lower(parsed, plan)
        except UnsupportedError as err:
            # Found while lowering rather than by the subset check: keep the
            # statement as Python and start over, since keeping it changes
            # what other statements can call directly.
            if not fallback or not plan.keep(tree, err):
                raise


def _keep_redefinitions(tree: ast.Module, plan: FallbackPlan, filename: str) -> None:
    """Two native `def`s of one name would need two Rust names; the later
    ones stay Python."""
    seen: set[str] = set()
    for stmt in tree.body:
        if not isinstance(stmt, ast.FunctionDef) or plan.is_python(stmt):
            continue
        if stmt.name in seen:
            location = SourceLocation.of(filename, stmt)
            plan.python[stmt] = [Issue("function redefinition", location, stmt)]
        seen.add(stmt.name)


def _lower(parsed: ParsedModule, plan: FallbackPlan) -> LoweredModule:
    tree = parsed.tree
    python = plan.python
    scopes = analyze_scopes(tree, parsed.source, parsed.filename, python)
    bindings = analyze_module_bindings(tree, python)
    callgraph = build_call_graph(tree, bindings, scopes)
    defs = [s for s in tree.body if isinstance(s, ast.FunctionDef) and s not in python]
    fn_names = Renamer([d.name for d in defs], MODULE_RESERVED)
    module = ModuleContext(
        parsed.filename,
        bindings,
        callgraph,
        fn_names,
        scopes,
        comments=attach_comments(tree, parsed.comments),
        future_annotations=has_future_annotations(tree),
    )

    items: list[rs.Item] = []
    for fn in defs:
        comments = module.comments.get(fn, [])
        items.append(lower_function(module, scopes.of(fn), fn_names[fn.name], comments, True))
    items.append(_lower_exec_module(module, tree, parsed, plan))

    uses = ["pyo3::prelude::*"]
    uses += sorted(module.uses)
    uses.append("steelix_runtime as rt")
    file = rs.File([rs.Use(u) for u in uses] + items, inner_doc=_python_summary(parsed, tree, plan))
    return LoweredModule(parsed.name, file, plan)


def _lower_exec_module(
    module: ModuleContext, tree: ast.Module, parsed: ParsedModule, plan: FallbackPlan
) -> rs.Fn:
    ctx = Context(module, analyze_module_flow(tree, plan.python))
    lowering = StmtLowering(ctx)
    body: list[rs.Stmt] = []

    doc = docstring(tree, parsed.filename)
    if doc is not None:
        setter = rs.MethodCall(
            rs.Path(ctx.context()), "set_global", (rs.StrLit("__doc__"), rs.StrLit(doc))
        )
        body.append(rs.ExprStmt(rs.Try(setter)))

    lines = source_lines(parsed.source)
    previous_end = 0
    chunk: list[ast.stmt] = []
    for index, stmt in enumerate(tree.body):
        ctx.statement_index = index
        if plan.is_python(stmt) and stmt not in ctx.flow.unreachable:
            if not chunk:
                body += _leading_comments(parsed, previous_end, first_line(stmt))
            chunk.append(stmt)
            previous_end = last_line(stmt)
            continue
        if chunk:
            body += _exec_python(module, lines, chunk, plan)
            chunk = []
        previous_end = last_line(stmt)
        if isinstance(stmt, ast.FunctionDef):
            # The comments before a `def` went to its Rust fn.
            if stmt not in ctx.flow.unreachable:
                body += lowering.module_def(stmt, module.fn_names[stmt.name])
        elif not plan.is_python(stmt):
            body += lowering.stmt(stmt)
    if chunk:
        body += _exec_python(module, lines, chunk, plan)
    body += [rs.Comment(c) for c in module.comments.get(tree, [])]  # type: ignore[call-overload]

    return rs.Fn(
        name="exec_module",
        params=(rs.Param(CONTEXT, "&rt::Module<'_>"),),
        ret="PyResult<()>",
        body=bind_py(rs.Block(tuple(body), rs.Call(rs.Path("Ok"), (rs.Tuple(()),))), CONTEXT),
        doc=(f"Executes the body of `{parsed.filename}` in the module `{CONTEXT}`.",),
    )


def _leading_comments(parsed: ParsedModule, after: int, before: int) -> list[rs.Stmt]:
    """Comments on the lines between two statements, which belong to the
    second one. Comments inside a statement kept as Python stay in its source."""
    return [
        rs.Comment(text)
        for line, text in sorted(parsed.comments.items())
        if after < line < before and not (line == 1 and text.startswith("!"))
    ]


def _exec_python(
    module: ModuleContext, lines: list[str], stmts: list[ast.stmt], plan: FallbackPlan
) -> list[rs.Stmt]:
    """`rt::exec_python!(m, line = N, r#"..."#)?;` for consecutive statements
    kept as Python, with a comment saying why."""
    features: dict[str, str] = {}
    for stmt in stmts:
        for issue in plan.python[stmt]:
            location = issue.location
            features.setdefault(issue.feature, f"{location.line}:{location.col + 1}")
    reasons = ", ".join(f"{feature} at {where}" for feature, where in features.items())
    args: list[rs.Expr] = [rs.Path(CONTEXT), rs.Tokens((f"line = {first_line(stmts[0])}",))]
    if module.future_annotations:
        args.append(rs.Tokens(("future_annotations",)))
    args.append(rs.RawStrLit("\n" + python_source(lines, stmts)))
    call = rs.Try(rs.MacroCall("rt::exec_python", tuple(args)))
    return [rs.Comment(f"Kept as Python: {reasons}"), rs.ExprStmt(call)]


def _python_summary(parsed: ParsedModule, tree: ast.Module, plan: FallbackPlan) -> tuple[str, ...]:
    """Module documentation listing what is still Python, if anything."""
    if not plan.python:
        return ()
    out = [
        f"Parts of `{parsed.filename}` kept as Python source and run by",
        "`rt::exec_python!` in `exec_module` (steelix does not lower them yet):",
        "",
    ]
    for stmt in tree.body:
        issues = plan.python.get(stmt)
        if issues is None:
            continue
        features = ", ".join(dict.fromkeys(issue.feature for issue in issues))
        out.append(f"- line {first_line(stmt)}, {_describe(stmt)}: {features}")
    return tuple(out)


def _describe(stmt: ast.stmt) -> str:
    match stmt:
        case ast.FunctionDef(name) | ast.AsyncFunctionDef(name):
            return f"`def {name}`"
        case ast.ClassDef(name):
            return f"`class {name}`"
    return f"`{type(stmt).__name__.lower()}` statement"
