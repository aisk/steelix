"""Lower a whole Python module into one Rust module file.

The Rust module exposes `exec_module(m)`, which runs the Python module body
against an existing module object. It does not know whether it runs inside an
extension module or as `__main__` of an executable; the emitted `lib.rs` /
`main.rs` decides that.

In mixed mode (the default) module level statements that can not be lowered
are kept as Python source and run in place by `rt::exec_python!` (design §13).
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.bindings import analyze_module_bindings, analyze_module_flow
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.classinfo import static_attributes
from steelix.frontend.desugar import desugar
from steelix.frontend.fallback import (
    FallbackPlan,
    first_line,
    module_annotations,
    python_runs,
    source_lines,
)
from steelix.frontend.parse import ParsedModule
from steelix.frontend.scope import analyze_scopes, has_future_annotations
from steelix.frontend.subset import Issue, find_unsupported
from steelix.lower.context import Context, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.function import (
    attach_comment_lines,
    attach_comments,
    bind_py,
    in_frame,
    lower_function,
)
from steelix.lower.names import CONTEXT, MODULE_RESERVED, Renamer, module_ident
from steelix.lower.stmt import StmtLowering
from steelix.rustast import nodes as rs
from steelix.rustast.cleanup import move_returned_locals


@dataclass
class LoweredModule:
    name: str
    file: rs.File
    plan: FallbackPlan
    """The statements kept as Python source."""


@dataclass(frozen=True)
class ModuleRole:
    """How a module is used in its program, which the analysis of a single
    module can not see."""

    own_names: tuple[str, ...] = ("__main__",)
    """The names the module runs under: `__main__` for the script, its
    import name for a module of a project (both for a script that is also
    imported by name)."""
    patched: frozenset[str] | None = frozenset()
    """Functions other modules of the project may replace, `None` if they
    may bind any of its globals (design §7)."""
    submodules: tuple[str, ...] = ()
    """The Rust modules of a package's submodules, declared in its file."""
    rust_name: str | None = None
    """The module's own Rust name, which its class modules avoid (clippy's
    `module_inception`). Defaults to the one of its Python name."""


def lower_module(
    parsed: ParsedModule,
    *,
    fallback: bool = True,
    role: ModuleRole | None = None,
    traceback_lines: bool = False,
) -> LoweredModule:
    """Lower `parsed`. Without `fallback`, the first unsupported construct
    raises `UnsupportedError` instead of being kept as Python. `role`
    defaults to a script run as `__main__`. `traceback_lines` emits the
    `frame.line(n)` markers (design §9)."""
    role = role or ModuleRole()
    tree = parsed.tree
    desugar(tree)
    plan = FallbackPlan()
    for stmt, issues in find_unsupported(tree, parsed.filename).items():
        plan.add(stmt, issues)
    _keep_redefinitions(tree, plan, parsed.filename)
    if not fallback:
        for stmt in tree.body:
            for kept in plan.within(stmt):
                raise plan.python[kept][0].error()
    attributes = static_attributes(parsed.source, parsed.filename)
    while True:
        try:
            return _lower(parsed, plan, attributes, role, traceback_lines)
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


def _lower(
    parsed: ParsedModule,
    plan: FallbackPlan,
    attributes: dict[tuple[str, int], tuple[str, ...]],
    role: ModuleRole,
    traceback_lines: bool = False,
) -> LoweredModule:
    tree = parsed.tree
    python = plan.python
    scopes = analyze_scopes(tree, parsed.source, parsed.filename, python)
    bindings = analyze_module_bindings(tree, python, own_names=role.own_names, patched=role.patched)
    callgraph = build_call_graph(tree, bindings, scopes)
    defs = [s for s in tree.body if isinstance(s, ast.FunctionDef) and s not in python]
    fn_names = Renamer([d.name for d in defs], MODULE_RESERVED)
    future_annotations = has_future_annotations(tree)
    annotations = [] if future_annotations else _simple_annotations(tree)
    module = ModuleContext(
        parsed.filename,
        bindings,
        callgraph,
        fn_names,
        scopes,
        comments=attach_comments(tree, parsed.comments),
        future_annotations=future_annotations,
        plan=plan,
        annotation_index={a: i for i, a in enumerate(annotations)},
        source_lines=source_lines(parsed.source),
        leading_comments=_leading_comments(tree, parsed, plan),
        static_attributes=attributes,
        traceback_lines=traceback_lines,
        # Class modules avoid the submodules and the module's own name.
        mod_names={
            *role.submodules,
            (role.rust_name or module_ident(parsed.name)).removeprefix("r#"),
        },
    )

    items: list[rs.Item] = []
    for fn in defs:
        comments = module.comments.get(fn, [])
        items.append(lower_function(module, scopes.of(fn), fn_names[fn.name], comments, True))
    exec_module = _lower_exec_module(module, tree, parsed, plan)
    items += module.class_mods
    items.append(exec_module)

    uses = ["pyo3::prelude::*"]
    uses += sorted(module.uses)
    uses.append("steelix_runtime as rt")
    submodules: list[rs.Item] = [rs.ModDecl(name, public=True) for name in role.submodules]
    file = rs.File(
        submodules + [rs.Use(u) for u in uses] + items,
        inner_doc=_python_summary(parsed, tree, plan),
    )
    return LoweredModule(parsed.name, move_returned_locals(file), plan)


def _lower_exec_module(
    module: ModuleContext, tree: ast.Module, parsed: ParsedModule, plan: FallbackPlan
) -> rs.Fn:
    ctx = Context(module, analyze_module_flow(tree, plan.python))
    lowering = StmtLowering(ctx, line=1)
    body: list[rs.Stmt] = []

    doc = docstring(tree, parsed.filename)
    if doc is not None:
        setter = rs.MethodCall(
            rs.Path(ctx.context()), "set_global", (rs.StrLit("__doc__"), rs.StrLit(doc))
        )
        body.append(rs.ExprStmt(rs.Try(setter)))
    body += _module_annotations(ctx, tree, parsed)

    index = {id(stmt): i for i, stmt in enumerate(tree.body)}
    unreachable = ctx.flow.unreachable
    stmts = [s for s in tree.body if not (plan.is_python(s) and s in unreachable)]
    for run in python_runs(stmts, plan):
        if isinstance(run, list):
            body += lowering.exec_python(run)
            continue
        ctx.statement_index = index[id(run)]
        if isinstance(run, ast.FunctionDef):
            # The comments before a `def` went to its Rust fn.
            if run not in unreachable:
                body += lowering.module_def(run, module.fn_names[run.name])
        else:
            body += lowering.stmt(run)
    body += [rs.Comment(c) for c in module.comments.get(tree, [])]  # type: ignore[call-overload]
    # Not after a loop that never ends.
    ok = rs.Call(rs.Path("Ok"), (rs.Tuple(()),))

    block = rs.Block(tuple(body), ok if ctx.flow.end_reachable else None)
    return rs.Fn(
        name="exec_module",
        params=(rs.Param(CONTEXT, "&rt::Module<'_>"),),
        ret="PyResult<()>",
        body=bind_py(in_frame(CONTEXT, "<module>", 1, block), CONTEXT),
        doc=(f"Executes the body of `{parsed.filename}` in the module `{CONTEXT}`.",),
    )


def _simple_annotations(tree: ast.Module) -> list[ast.AnnAssign]:
    """The module level annotations that go into `__annotate__`: those of a
    plain name (not `(x): int` or `x.y: int`)."""
    return [a for a in module_annotations(tree) if a.simple and isinstance(a.target, ast.Name)]


def _module_annotations(ctx: Context, tree: ast.Module, parsed: ParsedModule) -> list[rs.Stmt]:
    """What CPython sets up for module level annotations before the first
    statement runs. The annotated assignments are kept as Python, possibly in
    several `rt::exec_python!` blocks, but annotations are per module.

    * `__annotate__` evaluating all annotations (they are only returned once
      their assignment has run), or with `from __future__ import annotations`
    * the `__annotations__` dict that each assignment adds its annotation to.
    """
    module = ctx.module
    if module.future_annotations:
        if not module_annotations(tree):
            return []
        ctx.module.uses.add("pyo3::types::PyDict")
        create = rs.MethodCall(
            rs.Path(ctx.context()),
            "set_global",
            (rs.StrLit("__annotations__"), rs.Call(rs.Path("PyDict::new"), (rs.Path("py"),))),
        )
        return [
            rs.Comment("The module's `__annotations__`, which CPython creates before running it"),
            rs.ExprStmt(rs.Try(create)),
        ]
    if not module.annotation_index:
        return []
    entries = []
    for node in module.annotation_index:
        assert isinstance(node.target, ast.Name)
        text = ast.get_source_segment(parsed.source, node.annotation)
        assert text is not None
        entry = (rs.IntLit(node.annotation.lineno), rs.StrLit(f"{node.target.id}: {text}"))
        entries.append(rs.Tuple(entry))
    call = rs.Call(
        rs.Path("rt::module_annotate"), (rs.Path(ctx.context()), rs.Ref(rs.Array(tuple(entries))))
    )
    return [
        rs.Comment("The module's `__annotate__`, which CPython defines before running it"),
        rs.ExprStmt(rs.Try(call)),
    ]


def _leading_comments(
    tree: ast.Module, parsed: ParsedModule, plan: FallbackPlan
) -> dict[ast.stmt, list[str]]:
    """The comments before each statement kept as Python. Those inside it
    stay in its source."""
    attached = attach_comment_lines(tree, parsed.comments)
    return {
        stmt: [parsed.comments[line] for line in attached.get(stmt, []) if line < first_line(stmt)]
        for stmt in plan.python
    }


def _python_summary(parsed: ParsedModule, tree: ast.Module, plan: FallbackPlan) -> tuple[str, ...]:
    """Module documentation listing what is still Python, if anything."""
    if not plan.python:
        return ()
    out = [
        f"Parts of `{parsed.filename}` kept as Python source and run in place by",
        "`rt::exec_python!` (steelix does not lower them yet):",
        "",
    ]
    for top in tree.body:
        for stmt in plan.within(top):
            features = ", ".join(dict.fromkeys(issue.feature for issue in plan.python[stmt]))
            out.append(f"- line {first_line(stmt)}, {_describe(stmt)}: {features}")
    return tuple(out)


def _describe(stmt: ast.stmt) -> str:
    match stmt:
        case ast.FunctionDef(name) | ast.AsyncFunctionDef(name):
            return f"`def {name}`"
        case ast.ClassDef(name):
            return f"`class {name}`"
    return f"`{type(stmt).__name__.lower()}` statement"
