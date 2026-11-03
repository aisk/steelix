"""Lower `class` statements (design §10).

Every class becomes a Rust module named after it, holding a `body` fn for
the class body and one fn per method:

    pub mod point {
        use super::*;

        pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> { .. }
        pub fn __init__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>, ..) -> .. { .. }
    }

The class statement itself is a short `rt::class!` call where it runs, which
creates the class exactly like `__build_class__` (metaclass, `__prepare__`,
`__init_subclass__`, ...) and runs `point::body` in the class namespace.
Methods are ordinary function objects stored in the namespace; methods that
use `super()` or `__class__` capture the class's `__class__` cell.

Statements of the body that can not be lowered are kept as Python and run
in the class namespace (method level mixed mode, design §13).
"""

from __future__ import annotations

import ast
import re
from typing import TYPE_CHECKING

from steelix.frontend.bindings import analyze_class_flow
from steelix.frontend.desugar import binding_name
from steelix.frontend.fallback import first_line
from steelix.frontend.scope import ClassScope
from steelix.lower.context import NAMESPACE, ClassBody, Context, ModuleContext
from steelix.lower.docstrings import docstring
from steelix.lower.expr import Kind, Value, keyword_arg
from steelix.lower.function import bind_py, in_frame, lower_function
from steelix.lower.names import FUNCTION_RESERVED, MODULE_RESERVED, RUST_KEYWORDS, rust_ident
from steelix.rustast import nodes as rs
from steelix.rustast.printer import string_literal

if TYPE_CHECKING:
    from steelix.lower.stmt import StmtLowering

# Names a class module must not take: they would hide what generated code
# refers to through `use super::*`.
_MOD_RESERVED = frozenset({"rt", "pyo3", "std", "core", "alloc", "body"}) | RUST_KEYWORDS
# Names of the items in a class module besides the methods, and locals of
# the class body fn.
_CLASS_ITEM_RESERVED = frozenset({"body", NAMESPACE, "__class__"})


def mod_name(class_name: str, taken: set[str]) -> str:
    """The Rust module name of a class: its name in snake case
    (`TextCalendar` -> `text_calendar`), made unique among `taken`."""
    name = re.sub(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", "_", class_name).lower()
    if not name.isidentifier() or not name.isascii():
        name = "class"
    while name in taken or name in _MOD_RESERVED:
        name += "_"
    taken.add(name)
    return name


def lower_class_def(lowering: StmtLowering, stmt: ast.ClassDef) -> list[rs.Stmt]:
    """A class statement: its module, then `rt::class!` creating the class,
    decorators applied, and the class bound to its name."""
    ctx = lowering.ctx
    module = ctx.module
    scope = module.scopes.class_of(stmt)
    outer = ctx.class_body
    taken = outer.mod_names if outer is not None else module.mod_names
    name = mod_name(stmt.name, taken)
    mod = lower_class_mod(module, scope, name)
    if outer is not None:
        outer.items.append(mod)
    else:
        module.class_mods.append(mod)

    exprs = lowering.exprs
    out: list[rs.Stmt] = []
    decorators: list[tuple[int, Value]] = []
    for decorator in stmt.decorator_list:
        exprs.line = decorator.lineno
        with exprs.fresh() as prelude:
            value = exprs.hoist(exprs.value(decorator), "decorator")
        out += lowering.flush_items()
        out += lowering.mark(decorator.lineno, prelude) + prelude
        decorators.append((decorator.lineno, value))

    exprs.line = stmt.lineno
    with exprs.fresh() as prelude:
        items = [b.value if isinstance(b, ast.Starred) else b for b in stmt.bases]
        items += [k.value for k in stmt.keywords]
        values = exprs.operands(items)
        parts: list[str | rs.Expr] = [f"{string_literal(stmt.name)}("]
        for i, (base, value) in enumerate(zip(stmt.bases, values, strict=False)):
            if i:
                parts.append(", ")
            if isinstance(base, ast.Starred):
                parts += ["*", value.bound()]
            else:
                parts.append(value.arg())
        for i, (keyword, value) in enumerate(
            zip(stmt.keywords, values[len(stmt.bases) :], strict=True)
        ):
            if i or stmt.bases:
                parts.append(", ")
            if keyword.arg is None:
                parts += ["**", value.bound()]
            else:
                parts += [f"{keyword_arg(keyword.arg)} = ", value.arg()]
        parts.append(")")
        args: list[rs.Expr] = [
            rs.Path(ctx.context()),
            rs.Tokens(merge_text(parts)),
            rs.Path(f"{name}::body"),
            rs.Tokens((f"line = {first_line(stmt)}",)),
        ]
        args += [rs.Tokens((option,)) for option in _class_options(module, scope)]
        created: rs.Expr = rs.Try(rs.MacroCall("rt::class", tuple(args)))
    out += lowering.flush_items()
    # Creating the class can always raise (the metaclass, the body).
    out += lowering.at_line(stmt.lineno) + prelude
    if decorators:
        temp = ctx.temp(mod_name(stmt.name, set()))
        out.append(rs.Let(temp, init=created))
        for line, decorator in reversed(decorators[1:]):
            applied = exprs.call_value(decorator, [Value(rs.Path(temp), Kind.OWNED)])
            out += lowering.at_line(line)
            out.append(rs.Let(temp, init=applied.expr))
        # The outermost one is applied where the class is stored, like a
        # decorated function: `m.set_global("Point", decorator.call1((point,))?)?`.
        line, decorator = decorators[0]
        out += lowering.at_line(line)
        created = exprs.call_value(decorator, [Value(rs.Path(temp), Kind.OWNED)]).expr
    with exprs.fresh() as prelude:
        store = lowering.store_name(stmt, binding_name(stmt), Value(created, Kind.OWNED))
    assert not prelude  # it would run before the decorator
    return out + [store]


def merge_text(parts: list[str | rs.Expr]) -> tuple[str | rs.Expr, ...]:
    merged: list[str | rs.Expr] = []
    for part in parts:
        if isinstance(part, str) and merged and isinstance(merged[-1], str):
            merged[-1] += part
        else:
            merged.append(part)
    return tuple(merged)


def _class_options(module: ModuleContext, scope: ClassScope) -> list[str]:
    node = scope.node
    options = []
    if scope.qualname != scope.name:
        options.append(f"qualname = {string_literal(scope.qualname)}")
    attributes = module.static_attributes.get((scope.qualname, first_line(node)), ())
    if attributes:
        options.append(f"attributes = [{', '.join(string_literal(a) for a in attributes)}]")
    if not module.future_annotations:
        entries = [
            f"({string_literal(s.target.id)}, {string_literal(ast.unparse(s.annotation))})"
            for s in node.body
            if isinstance(s, ast.AnnAssign) and isinstance(s.target, ast.Name)
        ]
        if entries:
            options.append(f"annotations = [{', '.join(entries)}]")
    return options


def lower_class_mod(module: ModuleContext, scope: ClassScope, name: str) -> rs.Mod:
    """The Rust module of a class: its body fn, methods and nested classes."""
    node = scope.node
    plan = module.plan
    reserved = set(module.fn_names.mapping.values()) | FUNCTION_RESERVED | MODULE_RESERVED
    reserved |= _CLASS_ITEM_RESERVED
    method_names: dict[ast.AST, str] = {}
    for method in scope.methods:
        if isinstance(method.node, ast.FunctionDef):
            rust = rust_ident(method.node.name)
            while rust in reserved:
                rust += "_"
            reserved.add(rust)
            method_names[method.node] = rust
    body_info = ClassBody(scope, name, method_names)
    flow = analyze_class_flow(node, plan.python)
    ctx = Context(module, flow, None, class_body=body_info, item_names=set(reserved))

    from steelix.lower.stmt import StmtLowering

    line = first_line(node)
    lowering = StmtLowering(ctx, line=line)
    body: list[rs.Stmt] = []
    if any("__class__" in method.frees for method in scope.methods):
        cell = rs.MethodCall(rs.Path(NAMESPACE), "class_cell")
        body.append(rs.Let("__class__", init=cell))
    if module.future_annotations and any(isinstance(s, ast.AnnAssign) for s in node.body):
        setup = rs.MethodCall(rs.Path(NAMESPACE), "setup_annotations")
        body.append(rs.ExprStmt(rs.Try(setup)))
    doc = docstring(node, module.filename)  # type: ignore[arg-type]
    if doc is not None:
        set_doc = rs.MethodCall(
            rs.Path(NAMESPACE), "set", (rs.StrLit("__doc__"), rs.StrLit(doc, lines=True))
        )
        body += lowering.at_line(node.body[0].lineno)
        body.append(rs.ExprStmt(rs.Try(set_doc)))
    body += lowering.block(node.body).stmts
    ok = rs.Call(rs.Path("Ok"), (rs.Tuple(()),))
    block = rs.Block(tuple(body), ok if flow.end_reachable else None)
    framed = in_frame(NAMESPACE, scope.qualname, line, block)
    body_fn = rs.Fn(
        name="body",
        params=(
            rs.Param(ctx.context(), "&rt::Module<'py>"),
            rs.Param(NAMESPACE, "&rt::Namespace<'py>"),
        ),
        ret="PyResult<()>",
        body=bind_py(framed, ctx.context()),
        lifetimes=("'py",),
        doc=(f"The body of `class {scope.qualname}`, run in its namespace `{NAMESPACE}`.",),
        comments=(f"{module.filename}:{node.lineno}",),
    )
    doc_lines = tuple(doc.splitlines()) if doc else ()
    return rs.Mod(name, (body_fn, *body_info.items), doc=doc_lines)


def lower_method(ctx: Context, node: ast.FunctionDef) -> str:
    """Lower a function defined in a class body into a fn of the class's
    module and return its name."""
    body = ctx.class_body
    assert body is not None
    module = ctx.module
    rust = body.method_names[node]
    comments = module.comments.get(node, [])
    body.items.append(lower_function(module, module.scopes.of(node), rust, comments, True))
    return rust
