"""Lower a whole Python module into one Rust module file.

The Rust module exposes `exec_module(m)`, which runs the Python module body
against an existing module object. It does not know whether it runs inside an
extension module or as `__main__` of an executable; the emitted `lib.rs` /
`main.rs` decides that.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import analyze_module_bindings, analyze_module_flow
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.desugar import desugar
from steelix.frontend.parse import ParsedModule
from steelix.frontend.scope import analyze_scopes, has_future_annotations
from steelix.frontend.subset import check_supported
from steelix.lower.context import Context, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.function import attach_comments, lower_function
from steelix.lower.names import MODULE_RESERVED, Renamer
from steelix.lower.stmt import StmtLowering
from steelix.rustast import nodes as rs


@dataclass
class LoweredModule:
    name: str
    file: rs.File


def lower_module(parsed: ParsedModule) -> LoweredModule:
    tree = parsed.tree
    desugar(tree)
    check_supported(tree, parsed.filename)
    scopes = analyze_scopes(tree, parsed.source, parsed.filename)
    bindings = analyze_module_bindings(tree)
    callgraph = build_call_graph(tree, bindings, scopes)
    defs = [s for s in tree.body if isinstance(s, ast.FunctionDef)]
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
    lowered_defs: set[str] = set()
    for fn in defs:
        if fn.name in lowered_defs:
            # Two Rust functions would need distinct names; not supported yet.
            raise module.unsupported("function redefinition", fn)
        lowered_defs.add(fn.name)
        comments = module.comments.get(fn, [])
        items.append(lower_function(module, scopes.of(fn), fn_names[fn.name], comments, True))
    items.append(_lower_exec_module(module, tree, parsed))

    uses = ["pyo3::prelude::*"]
    uses += sorted(module.uses)
    uses.append("steelix_runtime as rt")
    file = rs.File([rs.Use(u) for u in uses] + items)
    return LoweredModule(parsed.name, file)


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
        if isinstance(stmt, ast.FunctionDef):
            # The comments before a `def` went to its Rust fn.
            if stmt not in ctx.flow.unreachable:
                body += lowering.module_def(stmt, module.fn_names[stmt.name])
        else:
            body += lowering.stmt(stmt)
    body += [rs.Comment(c) for c in module.comments.get(tree, [])]  # type: ignore[call-overload]

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
