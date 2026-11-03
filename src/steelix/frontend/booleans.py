"""Function locals that can be a Rust `bool` (design §5).

A local qualifies when every value it is ever assigned is a Python `bool`
by the language's own rules, whatever the operands' types are, and the
lowering already computes it as a Rust `bool`:

- `True` and `False`
- `not x`
- `a is b`, `a is not b`, `a in b`, `a not in b` (a single comparison:
  `in` converts the result of `__contains__` to `bool`)
- `isinstance(x, t)` and `bool(x)`, when the name can only be the builtin
- another such local
- `and`, `or` and conditional expressions whose operands all qualify

Comparisons such as `a < b` or `a == b` do not: rich comparisons may return
any object (numpy arrays, a custom `__eq__`), and the local stores that
object.

The local must also be an ordinary Rust local: assigned only by plain
`x = value` statements (no `for`, `with`, `except`, `import`, `del`,
augmented or tuple assignment, `:=`), never read while it may be
unassigned (that would need `Option`, design §5), not captured by a nested
function, and not the target of a `try` whose body is that single
assignment (its `match` yields an object).
"""

from __future__ import annotations

import ast

from steelix.frontend.bindings import LocalsInfo, ModuleBindings
from steelix.frontend.regions import expression_body
from steelix.frontend.resolve import Lookup, resolve_call
from steelix.frontend.scope import FunctionScope, scope_body, walk_scope, walrus_targets

_BOOL_COMPARISONS = (ast.Is, ast.IsNot, ast.In, ast.NotIn)


def bool_locals(
    fn: ast.FunctionDef | ast.Lambda | ast.GeneratorExp,
    scope: FunctionScope,
    info: LocalsInfo,
    bindings: ModuleBindings,
) -> frozenset[str]:
    """The locals of `fn` that are a Rust `bool` (see the module doc)."""
    if not isinstance(fn, ast.FunctionDef):
        return frozenset()
    candidates = set(scope.locals) - set(scope.params) - scope.cells - info.optional
    candidates -= set(walrus_targets(fn))
    assigned: dict[str, list[ast.expr]] = {}
    plain_targets: set[int] = set()
    for node in _walk(fn):
        match node:
            case ast.Assign([ast.Name(name) as target], value):
                assigned.setdefault(name, []).append(value)
                plain_targets.add(id(target))
            case ast.Try(body=[ast.Assign([ast.Name(name)])]) if expression_body(node) is not None:
                candidates.discard(name)
            case ast.FunctionDef(name) | ast.ClassDef(name):
                candidates.discard(name)
            case ast.Import(names) | ast.ImportFrom(names=names):
                candidates -= {(a.asname or a.name).partition(".")[0] for a in names}
            case ast.ExceptHandler(name=str(name)):
                candidates.discard(name)
            case ast.MatchAs(name=str(name)) | ast.MatchStar(name=str(name)):
                candidates.discard(name)
            case ast.MatchMapping(rest=str(name)):
                candidates.discard(name)
    for node in _walk(fn):
        if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
            if id(node) not in plain_targets:
                candidates.discard(node.id)
    candidates &= set(assigned)
    # Assignments from other candidates: drop locals until nothing changes.
    changed = True
    while changed:
        changed = False
        for name in sorted(candidates):
            if not all(_is_bool(v, candidates, scope, bindings) for v in assigned[name]):
                candidates.discard(name)
                changed = True
    return frozenset(candidates)


def _walk(fn: ast.FunctionDef) -> list[ast.AST]:
    return [node for part in scope_body(fn) for node in walk_scope(part)]


def _is_bool(
    expr: ast.expr, names: set[str], scope: FunctionScope, bindings: ModuleBindings
) -> bool:
    match expr:
        case ast.Constant(value):
            return isinstance(value, bool)
        case ast.UnaryOp(ast.Not()):
            return True
        case ast.Compare(_, [op], [_]):
            return isinstance(op, _BOOL_COMPARISONS)
        case ast.Name(name):
            return name in names
        case ast.BoolOp(_, values):
            return all(_is_bool(v, names, scope, bindings) for v in values)
        case ast.IfExp(_, body, orelse):
            return _is_bool(body, names, scope, bindings) and _is_bool(
                orelse, names, scope, bindings
            )
        case ast.Call(ast.Name(name), args, []) if not any(
            isinstance(a, ast.Starred) for a in args
        ):
            arity = {"isinstance": 2, "bool": 1}.get(name)
            return (
                arity == len(args)
                and scope.is_global(name)
                and resolve_call(bindings, name, expr) is Lookup.BUILTIN
            )
    return False
