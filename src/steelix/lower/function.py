"""Lower one `def` or `lambda` into a Rust fn.

Module level functions become `pub fn` items of the module. Nested functions
and lambdas become `fn` items declared inside the body of the enclosing Rust
fn, right where the Python code defines them (design §6). Variables captured
from enclosing functions are extra parameters after `py` / `globals`: a
`&rt::Cell` when the variable may still be rebound, else its value.
"""

from __future__ import annotations

import ast

from steelix.frontend.bindings import analyze_locals
from steelix.frontend.scope import FunctionNode, FunctionScope, annotations_of
from steelix.lower.context import Context, FunctionInfo, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.names import FUNCTION_RESERVED, Renamer, rust_ident
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal

BOUND = "Bound<'py, PyAny>"
RESULT = f"PyResult<{BOUND}>"


def lower_function(
    module: ModuleContext,
    scope: FunctionScope,
    rust_name: str,
    comments: list[str],
    public: bool,
) -> rs.Fn:
    # Imported here: the statement lowering lowers nested functions with this.
    from steelix.lower.stmt import StmtLowering

    fn = scope.node
    assert fn is not None
    locals_info = analyze_locals(scope, module.scopes)
    module.frozen[scope] = locals_info.frozen
    # A local must not shadow the Rust name of a renamed function it calls.
    renamed_fns = {rust for py, rust in module.fn_names.mapping.items() if rust != py}
    names = Renamer(
        list(scope.params) + list(scope.frees) + locals_info.order,
        FUNCTION_RESERVED | renamed_fns,
    )
    module.names[scope] = names
    by_value = {name for name in scope.frees if name in module.frozen[scope.owner(name)]}
    info = FunctionInfo(fn, scope, locals_info, names, by_value)
    ctx = Context(module, locals_info, info)
    lowering = StmtLowering(ctx)

    body: list[rs.Stmt] = []
    if fn in module.callgraph.recursive:
        enter = rs.Call(rs.Path("rt::RecursionGuard::enter"), (rs.Path("py"),))
        body.append(rs.Let("_guard", init=rs.Try(enter)))
    typed = {info.varargs, info.varkw} - {None}
    for param in scope.params:
        rust = names[param]
        clone: rs.Expr = rs.MethodCall(rs.Path(rust), "clone")
        if param in typed:
            clone = rs.MethodCall(clone, "into_any")
        if info.is_cell(param):
            cell = rs.Call(
                rs.Path("rt::Cell::new"), (rs.Path("py"), rs.StrLit(param), rs.Path(rust))
            )
            body.append(rs.Let(rust, init=rs.Try(cell)))
        elif param in locals_info.optional:
            # A deleted parameter can be unassigned later on.
            some = rs.Call(rs.Path("Some"), (clone,))
            body.append(rs.Let(rust, True, f"Option<{BOUND}>", some))
        elif param in locals_info.assigned_params:
            body.append(rs.Let(rust, mutable=True, init=clone))
    for name in locals_info.order:
        rust = names[name]
        if info.is_cell(name):
            if not _cell_declared_at_assignment(info, name):
                empty = rs.Call(rs.Path("rt::Cell::empty"), (rs.Path("py"), rs.StrLit(name)))
                body.append(rs.Let(rust, init=rs.Try(empty)))
        elif name not in locals_info.declared_at_top:
            continue
        elif name in locals_info.optional:
            body.append(rs.Let(rust, True, f"Option<{BOUND}>", rs.Path("None")))
        else:
            body.append(rs.Let(rust, name in locals_info.mutable, BOUND))

    if isinstance(fn, ast.Lambda):
        with lowering.exprs.fresh() as prelude:
            tail: rs.Expr | None = lowering.exprs.value(fn.body).result()
        body += lowering.flush_items() + prelude
    else:
        stmts = lowering.reachable(fn.body)
        tail = None
        if stmts and isinstance(stmts[-1], ast.Return) and fn.body[-1] is stmts[-1]:
            # A final `return` becomes the tail expression.
            for stmt in stmts[:-1]:
                body += lowering.stmt(stmt)
            body += lowering.comments(stmts[-1])
            with lowering.exprs.fresh() as prelude:
                tail = lowering.return_value(stmts[-1].value)
            body += lowering.flush_items() + prelude
        else:
            for stmt in stmts:
                body += lowering.stmt(stmt)
            if locals_info.end_reachable:
                tail = rs.Call(rs.Path("Ok"), (rs.Call(rs.Path("rt::none"), (rs.Path("py"),)),))

    params = [rs.Param("py", "Python<'py>")]
    if fn in module.callgraph.needs_globals:
        params.append(rs.Param("globals", "&Bound<'py, PyDict>"))
        module.uses.add("pyo3::types::PyDict")
    for name in scope.frees:
        ty = f"&{BOUND}" if name in by_value else "&rt::Cell<'py>"
        params.append(rs.Param(names[name], ty))
    for param in scope.params:
        if param == info.varargs:
            module.uses.add("pyo3::types::PyTuple")
            ty = "&Bound<'py, PyTuple>"
        elif param == info.varkw:
            module.uses.add("pyo3::types::PyDict")
            ty = "&Bound<'py, PyDict>"
        else:
            ty = f"&{BOUND}"
        params.append(rs.Param(names[param], ty))

    doc = docstring(fn, module.filename) if isinstance(fn, ast.FunctionDef) else None
    return rs.Fn(
        name=rust_name,
        params=tuple(params),
        ret=RESULT,
        body=rs.Block(tuple(body), tail),
        lifetimes=("'py",),
        public=public,
        doc=tuple(doc.splitlines()) if doc else (),
        comments=(*comments, f"{module.filename}:{fn.lineno}"),
    )


def _cell_declared_at_assignment(info: FunctionInfo, name: str) -> bool:
    """`x = value` declares the cell with `let x = rt::Cell::new(..)`."""
    stmt = info.locals.declared_at.get(name)
    return (
        isinstance(stmt, ast.Assign)
        and len(stmt.targets) == 1
        and isinstance(stmt.targets[0], ast.Name)
        and stmt.targets[0].id == name
    )


def cell_declared_at(info: FunctionInfo, name: str, stmt: ast.stmt) -> bool:
    return _cell_declared_at_assignment(info, name) and info.locals.declared_at[name] is stmt


def lower_nested(ctx: Context, node: FunctionNode) -> str:
    """Lower a nested function into an item of the current Rust fn and
    return the item's name."""
    module = ctx.module
    scope = module.scopes.of(node)
    if isinstance(node, ast.Lambda):
        rust = ctx.item_name("lambda")
        comments: list[str] = []
    else:
        fn = ctx.function
        allowed = None
        if fn is not None and fn.locals.declared_at.get(node.name) is node:
            allowed = fn.names[node.name]
        rust = ctx.item_name(rust_ident(node.name), allowed)
        comments = module.comments.get(node, [])
    ctx.items.append(lower_function(module, scope, rust, comments, public=False))
    return rust


def signature_text(
    ctx: Context, scope: FunctionScope, rust_name: str, defaults: list[rs.Expr]
) -> rs.Tokens:
    """`name[captures](params) -> "ret"` as written in `rt::function!`.

    The Rust fn, the captured variables and the defaults are expressions of
    the result; parameter names only name the macro's own bindings.
    """
    module = ctx.module
    node = scope.node
    assert node is not None
    names = module.names[scope]
    annotations: dict[str, str] = {}
    if isinstance(node, ast.FunctionDef):
        for param, annotation in annotations_of(node):
            source = ast.unparse(annotation)
            if module.future_annotations:
                source = repr(source)
            annotations[param] = string_literal(source)

    out: list[str | rs.Expr] = [rs.Path(rust_name)]

    def separate(index: int) -> None:
        if index:
            out.append(", ")

    parent = ctx.function
    if scope.frees:
        out.append("[")
        for i, free in enumerate(scope.frees):
            assert parent is not None
            outer = parent.names[free]
            separate(i)
            if free in module.frozen[scope.owner(free)]:
                out.append(rs.Path(outer))
            else:
                out.append(rs.Ref(rs.Path(outer)))
            out.append(_as(outer, free))
        out.append("]")

    args = node.args
    defaults_iter = iter(defaults)
    params: list[list[str | rs.Expr]] = []
    if node in module.callgraph.needs_globals:
        params.append(["globals"])

    def param(arg: ast.arg, prefix: str = "", default: bool = False) -> list[str | rs.Expr]:
        rust = names[arg.arg]
        text = prefix + rust + _as(rust, arg.arg)
        if arg.arg in annotations:
            text += f": {annotations[arg.arg]}"
        if default:
            return [text + " = ", next(defaults_iter)]
        return [text]

    positional = args.posonlyargs + args.args
    first_default = len(positional) - len(args.defaults)
    for i, arg in enumerate(positional):
        params.append(param(arg, default=i >= first_default))
        if args.posonlyargs and i == len(args.posonlyargs) - 1:
            params.append(["/"])
    if args.vararg is not None:
        params.append(param(args.vararg, "*"))
    elif args.kwonlyargs:
        params.append(["*"])
    for arg, default in zip(args.kwonlyargs, args.kw_defaults, strict=True):
        params.append(param(arg, default=default is not None))
    if args.kwarg is not None:
        params.append(param(args.kwarg, "**"))

    out.append("(")
    for i, part in enumerate(params):
        separate(i)
        out += part
    out.append(")")
    if "return" in annotations:
        out.append(f" -> {annotations['return']}")
    # Join adjacent text for a compact node.
    merged: list[str | rs.Expr] = []
    for part in out:
        if isinstance(part, str) and merged and isinstance(merged[-1], str):
            merged[-1] += part
        elif part != "":
            merged.append(part)
    return rs.Tokens(tuple(merged))


def _as(rust: str, python: str) -> str:
    """` as "name"` when the Python name is not the Rust one minus `r#`."""
    return "" if rust.removeprefix("r#") == python else f" as {string_literal(python)}"


def function_options(ctx: Context, scope: FunctionScope, rust_name: str) -> list[str]:
    """`qualname = ".."`, `name = ".."` and `doc = ".."` for `rt::function!`."""
    node = scope.node
    assert node is not None
    options = []
    if scope.qualname != scope.name:
        options.append(f"qualname = {string_literal(scope.qualname)}")
    elif rust_name.removeprefix("r#") != scope.name:
        key = "name" if isinstance(node, ast.FunctionDef) else "qualname"
        options.append(f"{key} = {string_literal(scope.name)}")
    if isinstance(node, ast.FunctionDef):
        doc = docstring(node, ctx.module.filename)
        if doc is not None:
            options.append(f"doc = {string_literal(doc)}")
    return options


def attach_comments(module: ast.Module, comments: dict[int, str]) -> dict[ast.stmt, list[str]]:
    """Assign each Python comment to the statement it precedes or ends.

    A comment belongs to the first statement (at any nesting depth) that ends
    on or after its line. Comments on the header line of a compound statement
    belong to the statement itself. Comments after the last statement of the
    module are keyed by the module node.
    """
    lines = sorted(line for line in comments if not (line == 1 and comments[line].startswith("!")))
    result: dict[ast.stmt, list[str]] = {}
    position = 0

    def take(until: int) -> list[str]:
        nonlocal position
        taken = []
        while position < len(lines) and lines[position] <= until:
            taken.append(comments[lines[position]])
            position += 1
        return taken

    def block(stmts: list[ast.stmt]) -> None:
        for stmt in stmts:
            children = [b for b in _child_blocks(stmt) if b]
            head_end = _header_end(stmt) if children else stmt.end_lineno or stmt.lineno
            found = take(head_end)
            if found:
                result[stmt] = found
            for child in children:
                block(child)

    block(module.body)
    rest = take(10**9)
    if rest:
        result[module] = rest  # type: ignore[index]
    return result


def _header_end(stmt: ast.stmt) -> int:
    """The last line of the header of a compound statement, before its body."""
    end = stmt.lineno
    pending = [c for c in ast.iter_child_nodes(stmt) if not isinstance(c, ast.stmt)]
    while pending:
        node = pending.pop()
        if isinstance(node, ast.expr | ast.arg):
            end = max(end, node.end_lineno or node.lineno)
        pending.extend(ast.iter_child_nodes(node))
    return end


def _child_blocks(stmt: ast.stmt) -> list[list[ast.stmt]]:
    match stmt:
        case ast.If(_, body, orelse) | ast.While(_, body, orelse) | ast.For(_, _, body, orelse):
            return [body, orelse]
        case ast.FunctionDef(body=body):
            return [body]
    return []
