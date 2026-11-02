"""Lower one `def` or `lambda` into a Rust fn.

Module level functions become `pub fn` items of the module. Nested functions
and lambdas become `fn` items declared inside the body of the enclosing Rust
fn, right where the Python code defines them (design §6). Every fn takes the
module context `m: &rt::Module<'py>` first. Variables captured from
enclosing functions are extra parameters after it: a `&rt::Cell` when the
variable may still be rebound, else its value.
"""

from __future__ import annotations

import ast

from steelix.frontend.bindings import analyze_locals
from steelix.frontend.scope import FunctionNode, FunctionScope, annotations_of
from steelix.lower.context import Context, FunctionInfo, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.names import FUNCTION_RESERVED, Renamer, context_name, rust_ident
from steelix.lower.rustflow import fallible, statement_line
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal
from steelix.rustast.rewrite import count_path, replace_path

BOUND = "Bound<'py, PyAny>"
RESULT = f"PyResult<{BOUND}>"
MODULE = "&rt::Module<'py>"


def bind_py(body: rs.Block, context: str) -> rs.Block:
    """Provide the `py` token that the lowered `body` refers to.

    A body that uses it more than once starts with `let py = m.py();`, a
    single use is spelled `m.py()` in place.
    """
    py = rs.MethodCall(rs.Path(context), "py")
    uses = count_path(body, "py")
    if uses == 0:
        return body
    if uses == 1:
        return replace_path(body, "py", py)
    return rs.Block((rs.Let("py", init=py), *body.stmts), body.tail)


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
    ctx = Context(module, locals_info, info, context_name=context_name(names.mapping.values()))
    first_line = _first_line(fn)
    lowering = StmtLowering(ctx, line=first_line)

    guard: list[rs.Stmt] = []
    if fn in module.callgraph.recursive:
        enter = rs.Call(rs.Path("rt::RecursionGuard::enter"), (rs.Path("py"),))
        guard.append(rs.Let("_guard", init=rs.Try(enter)))
    body: list[rs.Stmt] = []
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
        # Its body is on the line of the `lambda`, where the frame starts.
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
            body += lowering.flush_items()
            line = statement_line(stmts[-1])
            body += lowering.mark(line, [*prelude, rs.Return(tail)]) + prelude
        else:
            for stmt in stmts:
                body += lowering.stmt(stmt)
            if locals_info.end_reachable:
                tail = rs.Call(rs.Path("Ok"), (rs.Call(rs.Path("rt::none"), (rs.Path("py"),)),))

    params = [rs.Param(ctx.context_name, MODULE)]
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
    block = in_frame(ctx.context_name, scope.qualname, first_line, rs.Block(tuple(body), tail))
    return rs.Fn(
        name=rust_name,
        params=tuple(params),
        ret=RESULT,
        body=bind_py(rs.Block((*guard, *block.stmts), block.tail), ctx.context_name),
        lifetimes=("'py",),
        public=public,
        doc=tuple(doc.splitlines()) if doc else (),
        comments=(*comments, f"{module.filename}:{fn.lineno}"),
    )


def _first_line(fn: FunctionNode) -> int:
    """The first line of a function's code object: its first decorator."""
    if isinstance(fn, ast.FunctionDef):
        return min([fn.lineno, *(d.lineno for d in fn.decorator_list)])
    return fn.lineno


def in_frame(context: str, qualname: str, line: int, body: rs.Block) -> rs.Block:
    """`body` run in a traceback frame: `rt::frame!(m, "name", line, |frame| ..)`
    (design §9). A body that can not raise needs none."""
    tail = [] if body.tail is None else [rs.Return(body.tail)]
    if not fallible([*body.stmts, *tail]):
        return body
    param = "frame" if count_path(body, "frame") else "_"
    args = (rs.Path(context), rs.StrLit(qualname), rs.IntLit(line), rs.Closure((param,), body))
    return rs.Block((), rs.MacroCall("rt::frame", args))


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
    return {
        stmt: [comments[line] for line in lines]
        for stmt, lines in attach_comment_lines(module, comments).items()
    }


def attach_comment_lines(module: ast.Module, comments: dict[int, str]) -> dict[ast.stmt, list[int]]:
    """`attach_comments`, giving the line numbers of the comments."""
    lines = sorted(line for line in comments if not (line == 1 and comments[line].startswith("!")))
    result: dict[ast.stmt, list[int]] = {}
    position = 0

    def take(until: int) -> list[int]:
        nonlocal position
        taken = []
        while position < len(lines) and lines[position] <= until:
            taken.append(lines[position])
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
    pending = [
        c for c in ast.iter_child_nodes(stmt) if not isinstance(c, ast.stmt | ast.excepthandler)
    ]
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
        case ast.FunctionDef(body=body) | ast.With(body=body):
            return [body]
        case ast.Try(body, handlers, orelse, finalbody):
            return [body, *(h.body for h in handlers), orelse, finalbody]
    return []
