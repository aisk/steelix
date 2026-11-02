"""Lower a whole Python module into one Rust module file.

The Rust module exposes `exec_module(m)`, which runs the Python module body
against an existing module object. It does not know whether it runs inside an
extension module or as `__main__` of an executable; the emitted `lib.rs` /
`main.rs` decides that.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import (
    analyze_locals,
    analyze_module_bindings,
    analyze_module_flow,
)
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.parse import ParsedModule
from steelix.frontend.scope import FunctionScope, analyze_scopes
from steelix.frontend.subset import check_supported
from steelix.lower.context import Context, FunctionInfo, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.names import FUNCTION_RESERVED, MODULE_RESERVED, Renamer
from steelix.lower.stmt import StmtLowering
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal

BOUND = "Bound<'py, PyAny>"
RESULT = f"PyResult<{BOUND}>"


@dataclass
class LoweredModule:
    name: str
    file: rs.File


def lower_module(parsed: ParsedModule) -> LoweredModule:
    tree = parsed.tree
    check_supported(tree, parsed.filename)
    scopes = analyze_scopes(tree, parsed.source, parsed.filename)
    bindings = analyze_module_bindings(tree)
    callgraph = build_call_graph(tree, bindings, scopes)
    defs = [s for s in tree.body if isinstance(s, ast.FunctionDef)]
    fn_names = Renamer([d.name for d in defs], MODULE_RESERVED)
    module = ModuleContext(parsed.filename, bindings, callgraph, fn_names)

    items: list[rs.Item] = []
    lowered_defs: set[str] = set()
    for fn in defs:
        if fn.name in lowered_defs:
            # Two Rust functions would need distinct names; not needed for M1.
            raise module.unsupported("function redefinition", fn)
        lowered_defs.add(fn.name)
        items.append(_lower_function(module, fn, scopes.of(fn), parsed))
    items.append(_lower_exec_module(module, tree, parsed))

    uses = ["pyo3::prelude::*"]
    if callgraph.needs_globals:
        module.uses.add("pyo3::types::PyDict")
    uses += sorted(module.uses)
    uses.append("steelix_runtime as rt")
    file = rs.File([rs.Use(u) for u in uses] + items)
    return LoweredModule(parsed.name, file)


def _lower_function(
    module: ModuleContext, fn: ast.FunctionDef, scope: FunctionScope, parsed: ParsedModule
) -> rs.Fn:
    locals_info = analyze_locals(fn, scope)
    # A local must not shadow the Rust name of a renamed function it calls.
    renamed_fns = {rust for py, rust in module.fn_names.mapping.items() if rust != py}
    names = Renamer(
        list(scope.params) + sorted(scope.locals - set(scope.params)),
        FUNCTION_RESERVED | renamed_fns,
    )
    module.param_names[fn] = [names[p] for p in scope.params]
    info = FunctionInfo(fn, scope, locals_info, names)
    ctx = Context(module, locals_info, info)
    lowering = StmtLowering(ctx)

    body: list[rs.Stmt] = []
    if fn in module.callgraph.recursive:
        body.append(rs.Let("_guard", init=rs.Path("rt::RecursionGuard::enter(py)?")))
    for param in scope.params:
        rust = names[param]
        clone = rs.MethodCall(rs.Path(rust), "clone")
        if param in locals_info.optional:
            # A deleted parameter can be unassigned later on.
            some = rs.Call(rs.Path("Some"), (clone,))
            body.append(rs.Let(rust, True, f"Option<{BOUND}>", some))
        elif param in locals_info.assigned_params:
            body.append(rs.Let(rust, mutable=True, init=clone))
    for name in locals_info.declared_at_top:
        rust = names[name]
        if name in locals_info.optional:
            body.append(rs.Let(rust, True, f"Option<{BOUND}>", rs.Path("None")))
        else:
            body.append(rs.Let(rust, name in locals_info.mutable, BOUND))

    stmts = lowering.reachable(fn.body)
    tail: rs.Expr | None = None
    if stmts and isinstance(stmts[-1], ast.Return) and fn.body[-1] is stmts[-1]:
        # A final `return` becomes the tail expression.
        for stmt in stmts[:-1]:
            body += lowering.stmt(stmt)
        with lowering.exprs.fresh() as prelude:
            tail = lowering.return_value(stmts[-1].value)
        body += prelude
    else:
        for stmt in stmts:
            body += lowering.stmt(stmt)
        if locals_info.end_reachable:
            tail = rs.Call(rs.Path("Ok"), (rs.Call(rs.Path("rt::none"), (rs.Path("py"),)),))

    params = [rs.Param("py", "Python<'py>")]
    if fn in module.callgraph.needs_globals:
        params.append(rs.Param("globals", "&Bound<'py, PyDict>"))
    params += [rs.Param(names[p], f"&{BOUND}") for p in scope.params]

    doc = docstring(fn, parsed.filename)
    return rs.Fn(
        name=module.fn_names[fn.name],
        params=tuple(params),
        ret=RESULT,
        body=rs.Block(tuple(body), tail),
        lifetimes=("'py",),
        doc=tuple(doc.splitlines()) if doc else (),
        comments=(f"{parsed.filename}:{fn.lineno}",),
    )


def _lower_exec_module(module: ModuleContext, tree: ast.Module, parsed: ParsedModule) -> rs.Fn:
    ctx = Context(module, analyze_module_flow(tree))
    lowering = StmtLowering(ctx)
    body: list[rs.Stmt] = []

    doc = docstring(tree, parsed.filename)
    if doc is not None:
        setter = rs.MethodCall(
            rs.Path(ctx.globals()), "set_item", (rs.StrLit("__doc__"), rs.StrLit(doc))
        )
        body.append(rs.ExprStmt(rs.Try(setter)))

    for index, stmt in enumerate(tree.body):
        ctx.statement_index = index
        if stmt in ctx.flow.unreachable:
            continue
        if isinstance(stmt, ast.FunctionDef):
            body.append(rs.ExprStmt(rs.Try(_def_macro(module, ctx, stmt, parsed))))
        else:
            body += lowering.stmt(stmt)

    prologue: list[rs.Stmt] = []
    if ctx.uses_py:
        prologue.append(rs.Let("py", init=rs.MethodCall(rs.Path("m"), "py")))
    if ctx.uses_globals:
        prologue.append(rs.Let("globals", init=rs.Ref(rs.MethodCall(rs.Path("m"), "dict"))))
    return rs.Fn(
        name="exec_module",
        params=(rs.Param("m", "&Bound<'_, PyModule>"),),
        ret="PyResult<()>",
        body=rs.Block(tuple(prologue + body), rs.Path("Ok(())")),
        doc=(f"Executes the body of `{parsed.filename}` in the module `m`.",),
    )


def _def_macro(
    module: ModuleContext, ctx: Context, fn: ast.FunctionDef, parsed: ParsedModule
) -> rs.MacroCall:
    rust = module.fn_names[fn.name]
    param_text = ", ".join(module.param_names[fn])
    if fn in module.callgraph.needs_globals:
        param_text = "globals" + (", " + param_text if param_text else "")
    args: list[rs.Expr] = [
        rs.Path(ctx.globals()),
        rs.Raw(f"{rust}({param_text})"),
    ]
    if rust.removeprefix("r#") != fn.name:
        args.append(rs.Raw(f"name = {_quote(fn.name)}"))
    doc = docstring(fn, parsed.filename)
    if doc is not None:
        args.append(rs.Raw(f"doc = {_quote(doc)}"))
    return rs.MacroCall("rt::def", tuple(args))


def _quote(text: str) -> str:
    return string_literal(text)
