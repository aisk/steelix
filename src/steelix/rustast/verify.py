"""Check generated Rust for mistakes that rustc reports late or not at all.

The lowering builds Rust code piece by piece, and a few kinds of mistakes are
easy to make there and hard to notice in review:

* a path or local used outside the scope that declares it, e.g. a temporary
  declared in one block of a condition and used in the next, or a nested fn
  item declared in a loop body but used in the loop header;
* `break`/`continue` to a label that is not there, or an unlabeled one that
  leaves a labeled block (rustc E0695);
* a method call on a value of a concrete type such as `Bound<PyDict>` that
  resolves to the type's own method instead of the `PyAnyMethods` one (see
  `rustast.pyo3`). This one compiles and silently changes semantics.

The checks work on the Rust AST before printing. They know only what the
generated code spells: types come from parameter types and from the few
expressions that make typed values, anything else counts as `PyAny`.
`Tokens` (macro arguments that are not expressions) must keep every reference
in their expression parts; their text is not checked.
"""

from __future__ import annotations
import __future__

import ast
import re
from dataclasses import dataclass, field

from steelix.frontend.fallback import annotate_source
from steelix.rustast import nodes as rs
from steelix.rustast.pyo3 import must_erase

# Names in scope everywhere: the Rust prelude and crate roots.
_PRELUDE = frozenset({"Ok", "Err", "Some", "None", "Option", "Vec", "String", "Box", "std", "core"})
# What `use pyo3::prelude::*` brings in that generated code names.
_PYO3_PRELUDE = frozenset({"Python", "Bound", "Py", "PyAny", "PyResult", "PyErr", "PyModule"})

_IDENT = re.compile(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*")
_EXEC_LINE = re.compile(r"line = [1-9][0-9]*")
_FUTURE_ANNOTATIONS = rs.Tokens(("future_annotations",))
_FIRST_ANNOTATION = re.compile(r"first_annotation = (0|[1-9][0-9]*)")
_BOUND_TYPE = re.compile(r"Bound<'\w+, (Py\w+)>")

# Constructors of typed values, by path.
_TYPED_CALLS = {
    "PyList::new": "PyList",
    "PyList::empty": "PyList",
    "PyTuple::new": "PyTuple",
    "PyTuple::empty": "PyTuple",
    "PySet::new": "PySet",
    "PySet::empty": "PySet",
    "PyFrozenSet::new": "PyFrozenSet",
    "PyFrozenSet::empty": "PyFrozenSet",
    "PyDict::new": "PyDict",
    "rt::slice": "PySlice",
    "rt::slice_step": "PySlice",
    "rt::ascii": "PyString",
}
# Methods returning typed values, by method name.
_TYPED_METHODS = {
    "into_py_dict": "PyDict",
    "globals": "PyDict",
    "str": "PyString",
    "repr": "PyString",
    "get_type": "PyType",
    "try_iter": "PyIterator",
}


class VerifyError(Exception):
    def __init__(self, problems: list[str]) -> None:
        super().__init__("generated Rust failed verification:\n" + "\n".join(problems))
        self.problems = problems


@dataclass
class _Scope:
    parent: _Scope | None
    barrier: bool = False
    """A fn body: locals of enclosing scopes are not visible through it."""
    locals: dict[str, str | None] = field(default_factory=dict)
    """Locals and parameters, with their concrete type if known."""
    items: set[str] = field(default_factory=set)

    def lookup(self, name: str) -> tuple[bool, str | None]:
        scope: _Scope | None = self
        crossed = False
        while scope is not None:
            if not crossed and name in scope.locals:
                return True, scope.locals[name]
            if name in scope.items:
                return True, None
            crossed = crossed or scope.barrier
            scope = scope.parent
        return False, None


@dataclass
class _Label:
    name: str | None
    loop: bool


class _Verifier:
    def __init__(self) -> None:
        self.problems: list[str] = []
        self.fn = "<module>"
        self.labels: list[_Label] = []

    def problem(self, text: str) -> None:
        self.problems.append(f"in fn `{self.fn}`: {text}")

    # --- items ---

    def file(self, file: rs.File) -> None:
        scope = _Scope(None, items=set(_PRELUDE))
        for item in file.items:
            match item:
                case rs.Use(path):
                    if path == "pyo3::prelude::*":
                        scope.items |= _PYO3_PRELUDE | {"pyo3"}
                    elif " as " in path:
                        scope.items.add(path.rsplit(" as ", 1)[1])
                    else:
                        scope.items.add(path.rsplit("::", 1)[-1])
                    scope.items.add(path.split("::", 1)[0])
                case rs.Fn(name) | rs.ModDecl(name) | rs.Mod(name):
                    scope.items.add(name)
        self.items(file.items, scope)

    def items(self, items: list[rs.Item] | tuple[rs.Item, ...], scope: _Scope) -> None:
        for item in items:
            if isinstance(item, rs.Fn):
                self.function(item, scope)
            elif isinstance(item, rs.Mod):
                self.mod(item, scope)

    def mod(self, mod: rs.Mod, outer: _Scope) -> None:
        """`pub mod name { use super::*; .. }`: the glob import makes the
        items of the enclosing module visible."""
        scope = _Scope(outer, barrier=True)
        for item in mod.items:
            if isinstance(item, rs.Fn | rs.Mod):
                scope.items.add(item.name)
        self.items(mod.items, scope)

    def function(self, fn: rs.Fn, outer: _Scope) -> None:
        saved = self.fn, self.labels
        self.fn, self.labels = fn.name, []
        scope = _Scope(outer, barrier=True)
        for param in fn.params:
            match = _BOUND_TYPE.search(param.ty)
            ty = match.group(1) if match and match.group(1) != "PyAny" else None
            scope.locals[param.name] = ty
        self.block(fn.body, scope)
        self.fn, self.labels = saved

    # --- statements ---

    def block(self, block: rs.Block, outer: _Scope) -> None:
        scope = _Scope(outer)
        # Items are visible in the whole block, before their declaration too.
        for stmt in block.stmts:
            if isinstance(stmt, rs.ItemStmt) and isinstance(stmt.item, rs.Fn):
                scope.items.add(stmt.item.name)
        for stmt in block.stmts:
            self.stmt(stmt, scope)
        if block.tail is not None:
            self.expr(block.tail, scope)

    def stmt(self, stmt: rs.Stmt, scope: _Scope) -> None:
        match stmt:
            case rs.Let(pattern, _, _, init):
                if init is not None:
                    self.expr(init, scope)
                names = _pattern_names(pattern)
                ty = self.type_of(init, scope) if init is not None and len(names) == 1 else None
                for name in names:
                    scope.locals[name] = ty
            case rs.Assign(target, value):
                self.expr(target, scope)
                self.expr(value, scope)
            case rs.ExprStmt(expr):
                self.expr(expr, scope)
            case rs.Return(value):
                if value is not None:
                    self.expr(value, scope)
            case rs.If(cond, then, orelse, pattern):
                self.expr(cond, scope)
                if pattern is None:
                    self.block(then, scope)
                else:
                    inner = _Scope(scope)
                    for name in _pattern_names(pattern):
                        inner.locals[name] = None
                    self.block(then, inner)
                if isinstance(orelse, rs.If):
                    self.stmt(orelse, scope)
                elif orelse is not None:
                    self.block(orelse, scope)
            case rs.While(cond, body, label, pattern):
                self.expr(cond, scope)
                inner = _Scope(scope)
                for name in _pattern_names(pattern or ""):
                    inner.locals[name] = None
                self.loop(body, label, inner)
            case rs.Loop(body, label):
                self.loop(body, label, scope)
            case rs.For(pattern, iter_, body, label):
                self.expr(iter_, scope)
                inner = _Scope(scope)
                for name in _pattern_names(pattern):
                    inner.locals[name] = None
                self.loop(body, label, inner)
            case rs.LabeledBlock(label, body):
                self.labels.append(_Label(label, loop=False))
                self.block(body, scope)
                self.labels.pop()
            case rs.Break(label):
                self.jump("break", label)
            case rs.Continue(label):
                self.jump("continue", label)
            case rs.ItemStmt(rs.Fn() as fn):
                self.function(fn, scope)
            case rs.Comment():
                pass
            case _:
                self.problem(f"unknown statement {type(stmt).__name__}")

    def loop(self, body: rs.Block, label: str | None, scope: _Scope) -> None:
        self.labels.append(_Label(label, loop=True))
        self.block(body, scope)
        self.labels.pop()

    def jump(self, kind: str, label: str | None) -> None:
        if label is None:
            for target in reversed(self.labels):
                if target.loop:
                    return
                self.problem(f"unlabeled `{kind}` inside labeled block `{target.name}` (E0695)")
                return
            self.problem(f"`{kind}` outside of a loop")
            return
        for target in reversed(self.labels):
            if target.name == label:
                if kind == "continue" and not target.loop:
                    self.problem(f"`continue {label}` names a block, not a loop")
                return
        self.problem(f"`{kind} {label}` has no enclosing label `{label}`")

    # --- expressions ---

    def expr(self, expr: rs.Expr, scope: _Scope) -> None:
        match expr:
            case rs.Path(text):
                self.path(text, scope)
            case rs.IntLit() | rs.FloatLit() | rs.StrLit() | rs.ByteStrLit() | rs.BoolLit():
                pass
            case rs.RawStrLit():
                pass
            case rs.MacroCall("rt::exec_python", args):
                self.exec_python(args, scope)
            case rs.Call(rs.Path("rt::module_annotate"), args):
                self.module_annotate(args, scope)
            case rs.Call(func, args):
                self.expr(func, scope)
                self.exprs(args, scope)
            case rs.MethodCall(receiver, method, args, concrete):
                self.expr(receiver, scope)
                self.exprs(args, scope)
                ty = self.type_of(receiver, scope)
                if must_erase(ty, method) and not concrete:
                    self.problem(
                        f"`.{method}()` on a `{ty}` calls `{ty}`'s own method,"
                        " not the `PyAnyMethods` one; erase it with `.as_any()`"
                    )
            case rs.MacroCall(_, args):
                self.exprs(args, scope)
            case rs.Tokens(parts):
                self.exprs(tuple(p for p in parts if not isinstance(p, str)), scope)
            case rs.Try(inner) | rs.Ref(inner) | rs.Unary(_, inner) | rs.Await(inner):
                self.expr(inner, scope)
            case rs.Tuple(elems) | rs.Array(elems):
                self.exprs(elems, scope)
            case rs.Binary(_, lhs, rhs):
                self.expr(lhs, scope)
                self.expr(rhs, scope)
            case rs.BlockExpr(block):
                self.block(block, scope)
            case rs.IfExpr(cond, then, orelse):
                self.expr(cond, scope)
                self.block(then, scope)
                if isinstance(orelse, rs.IfExpr):
                    self.expr(orelse, scope)
                else:
                    self.block(orelse, scope)
            case rs.Closure(params, body):
                # Locals of the enclosing fn are visible, jump targets are not.
                inner = _Scope(scope)
                for param in params:
                    for name in _pattern_names(param):
                        inner.locals[name] = None
                saved = self.labels
                self.labels = []
                self.block(body, inner)
                self.labels = saved
            case rs.AsyncBlock(body):
                # Like a closure: `return` and `?` leave the block, no label
                # outside is a jump target.
                saved = self.labels
                self.labels = []
                self.block(body, scope)
                self.labels = saved
            case rs.Match(scrutinee, arms):
                self.expr(scrutinee, scope)
                for arm in arms:
                    inner = _Scope(scope)
                    for name in _pattern_names(arm.pattern):
                        inner.locals[name] = None
                    if arm.guard is not None:
                        self.expr(arm.guard, inner)
                    self.block(arm.body, inner)
            case _:
                self.problem(f"unknown expression {type(expr).__name__}")

    def exec_python(self, args: tuple[rs.Expr, ...], scope: _Scope) -> None:
        """`rt::exec_python!(m, line = N, [future_annotations | first_annotation
        = I,] r#"source"#)`: the source must start on the line after `r#"` and
        compile at line N."""
        match args:
            case (
                rs.Path() as module,
                rs.Tokens((str(line),)),
                *options,
                rs.RawStrLit(source),
            ) if _EXEC_LINE.fullmatch(line) and (
                options in ([], [_FUTURE_ANNOTATIONS])
                or (
                    len(options) == 1
                    and isinstance(options[0], rs.Tokens)
                    and _FIRST_ANNOTATION.fullmatch(str(options[0].parts[0]))
                )
            ):
                self.expr(module, scope)
            case _:
                self.problem("malformed `rt::exec_python!` call")
                return
        if not source.startswith("\n"):
            self.problem("the source of `rt::exec_python!` must start on a new line")
            return
        number = int(line.removeprefix("line = "))
        flags = __future__.annotations.compiler_flag if options == [_FUTURE_ANNOTATIONS] else 0
        padded = "\n" * (number - 2) + source
        if source[1:2].isspace():
            # An indented statement from a block, compiled in `if True:`.
            if number < 2:
                self.problem("an indented `rt::exec_python!` source must start after line 1")
                return
            padded = "\n" * (number - 2) + "if True:" + source
        try:
            compile(padded, "<exec_python>", "exec", flags=flags, dont_inherit=True)
        except SyntaxError as err:
            self.problem(f"the source of `rt::exec_python!` does not compile: {err}")

    def module_annotate(self, args: tuple[rs.Expr, ...], scope: _Scope) -> None:
        """`rt::module_annotate(m, &[(line, "name: annotation"), ..])`: the
        annotations must compile on their lines, in order."""
        match args:
            case (rs.Path() as module, rs.Ref(rs.Array(entries))) if entries:
                self.expr(module, scope)
            case _:
                self.problem("malformed `rt::module_annotate` call")
                return
        annotations: list[tuple[int, str]] = []
        for entry in entries:
            match entry:
                case rs.Tuple((rs.IntLit(line), rs.StrLit(text))) if line > 0:
                    annotations.append((line, text))
                case _:
                    self.problem("malformed `rt::module_annotate` entry")
                    return
        if annotations != sorted(annotations, key=lambda a: a[0]):
            self.problem("the annotations of `rt::module_annotate` are out of order")
        try:
            tree = ast.parse(annotate_source(annotations))
        except SyntaxError as err:
            self.problem(f"the annotations of `rt::module_annotate` do not compile: {err}")
            return
        if len(tree.body) != len(annotations) or not all(
            isinstance(s, ast.AnnAssign) and s.simple for s in tree.body
        ):
            self.problem("`rt::module_annotate` takes one `name: annotation` per entry")

    def exprs(self, exprs: tuple[rs.Expr, ...], scope: _Scope) -> None:
        for expr in exprs:
            self.expr(expr, scope)

    def path(self, text: str, scope: _Scope) -> None:
        first = text.split("::", 1)[0]
        if not _IDENT.fullmatch(first):
            self.problem(f"`{text}` is not a path")
            return
        found, _ = scope.lookup(first)
        if not found:
            self.problem(f"`{first}` is not in scope")

    def type_of(self, expr: rs.Expr, scope: _Scope) -> str | None:
        """The concrete type of a `Bound<'py, T>` expression, `None` for `PyAny`
        or anything unknown."""
        match expr:
            case rs.Path(text) if "::" not in text:
                return scope.lookup(text)[1]
            case rs.Ref(inner) | rs.Try(inner):
                return self.type_of(inner, scope)
            case rs.Call(rs.Path(path)):
                return _TYPED_CALLS.get(path)
            case rs.MethodCall(receiver, "clone"):
                return self.type_of(receiver, scope)
            case rs.MethodCall(_, method):
                return _TYPED_METHODS.get(method)
        return None


# Enum variants and paths that patterns name, which bind nothing.
_PATTERN_PATHS = frozenset(
    {"Ok", "Err", "Some", "None", "rt", "Flow", "Next", "Break", "Continue", "Return"}
)


def _pattern_names(pattern: str) -> list[str]:
    return [
        n
        for n in _IDENT.findall(pattern)
        if n not in ("mut", "ref", "_") and n not in _PATTERN_PATHS
    ]


def check(file: rs.File) -> list[str]:
    """Problems found in `file`, as readable messages."""
    verifier = _Verifier()
    verifier.file(file)
    return verifier.problems


def verify(file: rs.File) -> None:
    problems = check(file)
    if problems:
        raise VerifyError(problems)
