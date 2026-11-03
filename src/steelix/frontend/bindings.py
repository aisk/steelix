"""Binding analyses that `symtable` does not provide.

* Module level: how many times each global name is bound, and whether the
  module does anything (`import *`, `globals()`, `setattr` on its own module
  object, `exec`) that makes the set of globals unknowable. This decides
  which calls can be direct Rust calls (design §7).
* Function level: definite-assignment analysis deciding how each local variable
  is declared in Rust (design §5).
"""

from __future__ import annotations

import ast
from collections import Counter
from collections.abc import Collection
from dataclasses import dataclass, field

from steelix.frontend.desugar import binding_name
from steelix.frontend.regions import (
    Region,
    expression_body,
    region_node_ids,
    regions_in,
    try_regions,
    with_regions,
)
from steelix.frontend.scope import (
    MODULE_SCOPE,
    FunctionNode,
    FunctionScope,
    ModuleScopes,
    comprehension_variables,
    scope_body,
    walk_scope,
)

# Names the import system and module creation put into every module's globals.
MODULE_DUNDERS = frozenset(
    {
        "__name__",
        "__doc__",
        "__file__",
        "__spec__",
        "__loader__",
        "__package__",
        "__builtins__",
        "__path__",
        "__cached__",
        "__dict__",
    }
)

# Builtins that expose the calling scope's namespace: `globals` always,
# `vars` and `locals` when called without arguments.
_NAMESPACE_BUILTINS = frozenset({"globals", "vars", "locals"})


# --- module level ----------------------------------------------------------------


@dataclass
class ModuleBindings:
    counts: Counter[str]
    """Number of binding sites of each module global."""
    defs: dict[str, tuple[ast.FunctionDef, int]]
    """Top-level `def` statements by name, with their index in the module body."""
    star_import: bool
    namespace_access: bool
    """The module calls `globals()` / `vars()` / `locals()`."""
    writes_defaults: bool = False
    """The module may assign `__defaults__` / `__kwdefaults__` of a function."""
    patches_itself: bool = False
    """The module may bind its globals through its own module object
    (`setattr(sys.modules[__name__], ..)`) or with `exec` / `eval`."""
    patched: frozenset[str] = frozenset()
    """Functions that other modules of the project may replace
    (`helpers.f = ..`, `mock.patch("helpers.f")`): calls of them go through
    globals."""

    @property
    def dynamic(self) -> bool:
        """Globals can be bound by means invisible to the analysis."""
        return self.star_import or self.namespace_access or self.patches_itself

    def is_module_bound(self, name: str) -> bool:
        return name in self.counts or name in MODULE_DUNDERS

    def direct_function(self, name: str) -> ast.FunctionDef | None:
        """The `def` that is the only binding of `name`, if calls may bypass globals.

        A decorated name is bound to whatever the decorator returns.
        """
        if self.dynamic or self.counts[name] != 1 or name not in self.defs:
            return None
        if name in self.patched:
            return None
        fn = self.defs[name][0]
        return None if fn.decorator_list else fn

    def def_index(self, name: str) -> int:
        return self.defs[name][1]


def analyze_module_bindings(
    module: ast.Module,
    python: Collection[ast.stmt] = (),
    *,
    own_names: Collection[str] = ("__main__",),
    patched: Collection[str] | None = (),
) -> ModuleBindings:
    """Bindings of the module globals. A `def` among the statements `python`
    that are kept as Python source is not a target of direct calls: it has
    no Rust function, so calls go through globals.

    `own_names` are the names the module runs under (`__main__` for the
    script, its import name), which `sys.modules[..]` may name it by.
    `patched` are the functions that other modules of the project may
    replace, `None` if they may bind any global of the module."""
    counts: Counter[str] = Counter()
    defs: dict[str, tuple[ast.FunctionDef, int]] = {}
    star_import = False

    for index, stmt in enumerate(module.body):
        if isinstance(stmt, ast.FunctionDef) and stmt not in python:
            defs[stmt.name] = (stmt, index)

    def visit_module_level(node: ast.AST) -> None:
        nonlocal star_import
        for name in _bound_names(node):
            counts[name] += 1
        if isinstance(node, ast.ImportFrom) and any(a.name == "*" for a in node.names):
            star_import = True
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
            # The name itself was counted above; the body is another scope.
            declared_global: set[str] = set()
            for child in _scope_children(node):
                visit_nested_scope(child, declared_global)
            return
        if isinstance(node, ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp):
            # `:=` in a comprehension binds in the enclosing scope (PEP 572).
            for name in _comprehension_walrus_targets(node):
                counts[name] += 1
        if isinstance(
            node, ast.Lambda | ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp
        ):
            visit_nested_scope(node, declared_global=set())
            return
        for child in ast.iter_child_nodes(node):
            visit_module_level(child)

    def visit_nested_scope(node: ast.AST, declared_global: set[str]) -> None:
        # Inside functions only names declared `global` bind module globals.
        if isinstance(node, ast.Global):
            declared_global.update(node.names)
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda | ast.ClassDef):
            inner: set[str] = set()
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                for name in _bound_names(node):
                    if name in declared_global:
                        counts[name] += 1
            for child in _scope_children(node):
                visit_nested_scope(child, inner)
            return
        for name in _bound_names(node):
            if name in declared_global:
                counts[name] += 1
        for child in ast.iter_child_nodes(node):
            visit_nested_scope(child, declared_global)

    for stmt in module.body:
        visit_module_level(stmt)

    namespace_access = _uses_namespace_builtins(module, set())
    writes_defaults = _writes_defaults(module)
    def_names = {s.name for s in module.body if _is_def(s)}
    patches_itself = patched is None or _patches_itself(module, def_names, own_names)

    return ModuleBindings(
        counts,
        defs,
        star_import,
        namespace_access,
        writes_defaults,
        patches_itself,
        frozenset(patched or ()),
    )


def _is_def(stmt: ast.stmt) -> bool:
    return isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef)


def _patches_itself(module: ast.Module, def_names: set[str], own_names: Collection[str]) -> bool:
    """Whether code anywhere in the module (native or kept as Python) may
    bind its globals behind the analysis' back. Conservative, and only for
    the usual spellings (design §12 lists what is not detected):

    * `setattr` / `delattr`, an attribute assignment or deletion, `.__dict__`
      or `vars(..)` on the module object: `sys.modules[__name__]`,
      `sys.modules["__main__"]` (or `.get(..)`, and the module's import name
      instead of `__main__`), an imported `__main__` (or the module imported
      by its own name), or a name assigned one of those
    * `setattr` / `delattr` with a string literal naming a module level `def`
    * `exec` / `eval` without an explicit namespace (`globals()` is caught as
      namespace access)
    """
    refs: set[str] = set()
    for node in ast.walk(module):
        if isinstance(node, ast.Import):
            for alias in node.names:
                # `import pkg.mod` binds `pkg`, `import pkg.mod as me` the module.
                if alias.name in own_names and (alias.asname or "." not in alias.name):
                    refs.add(alias.asname or alias.name)
    # Names assigned the module object, also through other such names.
    while True:
        found = {
            target.id
            for node in ast.walk(module)
            if isinstance(node, ast.Assign | ast.NamedExpr)
            and _is_module_object(node.value, refs, own_names)
            for target in (node.targets if isinstance(node, ast.Assign) else [node.target])
            if isinstance(target, ast.Name)
        }
        if found <= refs:
            break
        refs |= found

    for node in ast.walk(module):
        match node:
            case ast.Call(ast.Name("setattr" | "delattr"), [obj, *rest]):
                if _is_module_object(obj, refs, own_names):
                    return True
                match rest:
                    case [ast.Constant(str() as name), *_] if name in def_names:
                        return True
            case ast.Call(ast.Name("exec" | "eval"), args, keywords):
                if len(args) < 2 and not any(k.arg == "globals" for k in keywords):
                    return True
            case ast.Call(ast.Name("vars"), [obj]) if _is_module_object(obj, refs, own_names):
                return True
            case ast.Attribute(obj, _, ast.Store() | ast.Del()) if _is_module_object(
                obj, refs, own_names
            ):
                return True
            case ast.Attribute(obj, "__dict__") if _is_module_object(obj, refs, own_names):
                return True
    return False


def _is_module_object(expr: ast.expr, refs: set[str], own_names: Collection[str]) -> bool:
    """Whether `expr` is a usual spelling of the module's own module object."""
    match expr:
        case ast.Name(name):
            return name in refs
        case ast.Subscript(modules, key) if _is_sys_modules(modules):
            return _is_own_name(key, own_names)
        case ast.Call(ast.Attribute(modules, "get"), [key, *_]) if _is_sys_modules(modules):
            return _is_own_name(key, own_names)
    return False


def _is_sys_modules(expr: ast.expr) -> bool:
    """`sys.modules`, also when imported under another name or with `from`."""
    return (
        isinstance(expr, ast.Attribute)
        and expr.attr == "modules"
        or (isinstance(expr, ast.Name) and expr.id == "modules")
    )


def _is_own_name(expr: ast.expr, own_names: Collection[str]) -> bool:
    return (
        isinstance(expr, ast.Name)
        and expr.id == "__name__"
        or (isinstance(expr, ast.Constant) and expr.value in own_names)
    )


def _comprehension_walrus_targets(node: ast.AST) -> list[str]:
    """Names bound by `:=` in a comprehension, nested comprehensions included."""
    names = []
    for child in ast.iter_child_nodes(node):
        if isinstance(child, ast.Lambda | ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
            continue
        if isinstance(child, ast.NamedExpr) and isinstance(child.target, ast.Name):
            names.append(child.target.id)
        names += _comprehension_walrus_targets(child)
    return names


_DEFAULTS_ATTRS = frozenset({"__defaults__", "__kwdefaults__"})


def _writes_defaults(module: ast.Module) -> bool:
    """Whether the module may change the defaults of a function object: it
    assigns or deletes `__defaults__` / `__kwdefaults__`, or spells either
    name as a string (`setattr(f, "__defaults__", ..)`)."""
    for node in ast.walk(module):
        match node:
            case ast.Attribute(_, attr, ast.Store() | ast.Del()) if attr in _DEFAULTS_ATTRS:
                return True
            case ast.Constant(str() as value) if value in _DEFAULTS_ATTRS:
                return True
    return False


def is_immutable_constant(expr: ast.expr) -> bool:
    """A literal whose value can not change: `None`, a bool, a number, a
    string or bytes."""
    return isinstance(expr, ast.Constant) and expr.value is not Ellipsis


def _uses_namespace_builtins(node: ast.AST, local_names: set[str]) -> bool:
    """Whether `globals`, or `vars` / `locals` without arguments, is used
    where it can be the builtin. `vars(obj)` reads another namespace."""
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda):
        args = node.args
        inner = {a.arg for a in args.posonlyargs + args.args + args.kwonlyargs}
        inner.update(a.arg for a in (args.vararg, args.kwarg) if a is not None)
        body = node.body if isinstance(node.body, list) else [node.body]
        for stmt in body:
            for child in ast.walk(stmt):
                if isinstance(child, ast.Name) and isinstance(child.ctx, ast.Store):
                    inner.add(child.id)
        # Decorators, defaults and annotations are evaluated in the outer scope.
        outer = [n for n in ast.iter_child_nodes(node) if n not in body and n is not args]
        outer += args.defaults + [d for d in args.kw_defaults if d is not None]
        return any(_uses_namespace_builtins(n, local_names) for n in outer) or any(
            _uses_namespace_builtins(stmt, inner) for stmt in body
        )
    if isinstance(node, ast.ClassDef):
        # In a class body `locals()` and `vars()` are the class namespace.
        outer = [*node.decorator_list, *node.bases, *node.keywords]
        inner = local_names | {"locals", "vars"}
        return any(_uses_namespace_builtins(n, local_names) for n in outer) or any(
            _uses_namespace_builtins(stmt, inner) for stmt in node.body
        )
    if (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Name)
        and node.func.id in ("vars", "locals")
        and (node.args or node.keywords)
    ):
        # `vars(obj)`: only the arguments can use the namespace.
        children: list[ast.AST] = [*node.args, *node.keywords]
        return any(_uses_namespace_builtins(c, local_names) for c in children)
    if (
        isinstance(node, ast.Name)
        and isinstance(node.ctx, ast.Load)
        and node.id in _NAMESPACE_BUILTINS
        and node.id not in local_names
    ):
        return True
    return any(_uses_namespace_builtins(c, local_names) for c in ast.iter_child_nodes(node))


def _scope_children(node: ast.AST) -> list[ast.AST]:
    """Children of a scope-introducing node that are evaluated inside the new scope."""
    match node:
        case ast.FunctionDef() | ast.AsyncFunctionDef():
            return list(node.body)
        case ast.ClassDef():
            return list(node.body)
        case ast.Lambda():
            return [node.body]
    return list(ast.iter_child_nodes(node))


def _bound_names(node: ast.AST) -> list[str]:
    """Names bound (or unbound) directly by `node`, not by its children. The
    one definition of a binding that the other helpers build on."""
    match node:
        case ast.Name(id, ast.Store() | ast.Del()):
            return [id]
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
            return [binding_name(node)]
        case ast.Import(names) | ast.ImportFrom(names=names):
            return [_import_name(alias) for alias in names if alias.name != "*"]
        case ast.ExceptHandler(name=str(name)):
            return [name]
        case ast.MatchAs(name=str(name)) | ast.MatchStar(name=str(name)):
            return [name]
        case ast.MatchMapping(rest=str(name)):
            return [name]
        case ast.TypeAlias(ast.Name(id)):
            return [id]
    return []


# --- function level ----------------------------------------------------------------


@dataclass
class LocalsInfo:
    """How the locals of one function are declared in Rust.

    Module level code is analysed the same way with an empty scope, which only
    computes reachability.
    """

    optional: set[str] = field(default_factory=set)
    """Locals read (or deleted) where they might be unassigned: `Option<Bound>`."""
    mutable: set[str] = field(default_factory=set)
    """Locals (and parameters) assigned while possibly already assigned."""
    declared_at: dict[str, ast.stmt | ast.NamedExpr] = field(default_factory=dict)
    """Locals declared with `let` at their first assignment (or import, or
    `:=` that always runs) instead of at the top."""
    loop_scoped: dict[str, ast.For] = field(default_factory=dict)
    """Loop variables only bound by one `for` and only read inside its body, which
    are declared with `let` at the start of each iteration."""
    assigned_params: set[str] = field(default_factory=set)
    """Parameters that are reassigned, so they are shadowed by an owned `let mut`."""
    declared_at_top: list[str] = field(default_factory=list)
    """Non-parameter locals declared at the top of the function, in source order."""
    order: list[str] = field(default_factory=list)
    """All non-parameter locals, in order of first occurrence."""
    assigned_deletes: set[ast.Name] = field(default_factory=set)
    """`del x` targets where `x` is definitely assigned."""
    frozen: set[str] = field(default_factory=set)
    """Cells that are never rebound once a nested function captured them: the
    nested functions receive their value instead of the cell (design §6)."""
    unreachable: set[ast.stmt] = field(default_factory=set)
    end_reachable: bool = True
    handler_scoped: set[str] = field(default_factory=set)
    """`except .. as name` names used only in handlers that bind them: each
    handler declares its own Rust variable, which goes away at its end like
    the implicit `del name`."""
    scoped: dict[str, Region] = field(default_factory=dict)
    """Locals used only inside one closure region (`frontend.regions`),
    declared inside its closure."""
    region_top: dict[Region, list[str]] = field(default_factory=dict)
    """Region scoped locals declared at the start of the region's closure."""
    with_params: set[str] = field(default_factory=set)
    """`with .. as name` names bound by the parameter of the `with` closure."""
    infallible_tries: set[ast.Try] = field(default_factory=set)
    """`try` statements whose body can not raise (`try: x = 1`): their
    handlers never run, and their statements are unreachable."""


@dataclass(frozen=True)
class _Flow:
    definite: frozenset[str]
    """Locals assigned on every path to this point."""
    maybe: frozenset[str]
    """Locals assigned on some path to this point."""


# `None` stands for "unreachable", the identity of `_merge`.
State = _Flow | None


def _merge(*states: State) -> State:
    """The state where control flow from all `states` joins."""
    reachable = [s for s in states if s is not None]
    if not reachable:
        return None
    definite = reachable[0].definite
    maybe = reachable[0].maybe
    for state in reachable[1:]:
        definite &= state.definite
        maybe |= state.maybe
    return _Flow(definite, maybe)


@dataclass
class _Loop:
    breaks: list[State] = field(default_factory=list)
    continues: list[State] = field(default_factory=list)


class _DefiniteAssignment:
    def __init__(
        self,
        scope: FunctionScope,
        scopes: ModuleScopes | None = None,
        opaque: Collection[ast.stmt] = (),
    ) -> None:
        self.scope = scope
        self.scopes = scopes
        self.opaque = opaque
        """Statements kept as Python source, which bind no locals here."""
        self.info = LocalsInfo()
        self.loops: list[_Loop] = []
        self.recording = True
        """Off while re-analysing code only for the state after it."""
        self.region_entry: dict[Region, _Flow] = {}
        self.candidates: dict[Region, set[str]] = {}
        """Locals whose every occurrence is in the region (innermost)."""
        self.tries: dict[ast.Try, bool] = {}
        """Whether the body of each `try` may raise, on some path analysed."""

    def run(self, body: list[ast.stmt]) -> LocalsInfo:
        params = frozenset(self.scope.params)
        for stmt in body:
            for node in walk_scope(stmt):
                if isinstance(node, ast.Delete):
                    # A deleted local can be unassigned again afterwards.
                    for target in node.targets:
                        self.info.optional.update(self._local_names(target))
        self.info.handler_scoped = self._handler_scoped(body)
        self._region_candidates(body)
        end = self.block(body, _Flow(params, params))
        self.info.end_reachable = end is not None
        self._infallible_tries()
        self._loop_scoped(body)
        self._regions(body)
        self._declarations(body)
        params = set(self.scope.params)
        self.info.order += sorted(self.scope.locals - params - set(self.info.order))
        self._frozen(body)
        return self.info

    def block(self, stmts: list[ast.stmt], state: State) -> State:
        for stmt in stmts:
            if state is None:
                if self.recording:
                    self.info.unreachable.add(stmt)
                continue
            state = self.stmt(stmt, state)
        return state

    def stmt(self, stmt: ast.stmt, state: _Flow) -> State:
        if stmt in self.opaque:
            return state
        match stmt:
            case ast.Expr(value):
                state = self.expr(value, state)
            case ast.Assign(targets, value):
                state = self.expr(value, state)
                for target in targets:
                    state = self.store(target, state)
            case ast.AugAssign(target, _, value):
                if isinstance(target, ast.Name):
                    self.read_name(target.id, state)
                else:
                    state = self.expr(target, state)
                state = self.expr(value, state)
                return self.store(target, state)
            case ast.Delete(targets):
                for target in targets:
                    state = self.delete(target, state)
            case ast.Return(value):
                if value is not None:
                    self.expr(value, state)
                return None
            case ast.If(test, body, orelse):
                state = self.expr(test, state)
                return _merge(self.block(body, state), self.block(orelse, state))
            case ast.While(test, body, orelse):
                return self.loop(stmt, state, test=test)
            case ast.For(target, iter_, body, orelse):
                state = self.expr(iter_, state)
                return self.loop(stmt, state, target=target)
            case ast.Break():
                self.loops[-1].breaks.append(state)
                return None
            case ast.Continue():
                self.loops[-1].continues.append(state)
                return None
            case ast.Import(names) | ast.ImportFrom(names=names):
                for alias in names:
                    name = alias.asname or alias.name.split(".")[0]
                    state = self.assign_name(name, state)
            case ast.Assert(test, msg):
                state = self.expr(test, state)
                if msg is not None:
                    # Only evaluated when the test fails, and then it raises.
                    self.expr(msg, state)
            case ast.FunctionDef(name, args, decorator_list=decorators):
                for expr in [*decorators, *args.defaults, *args.kw_defaults]:
                    if expr is not None:
                        state = self.expr(expr, state)
                return self.assign_name(binding_name(stmt), state)
            case ast.ClassDef(bases=bases, keywords=keywords, decorator_list=decorators):
                for expr in [*decorators, *bases, *(k.value for k in keywords)]:
                    state = self.expr(expr, state)
                return self.assign_name(binding_name(stmt), state)
            case ast.AnnAssign(target, _, value):
                # In a class body: the annotation is evaluated lazily.
                if value is not None:
                    state = self.expr(value, state)
                    return self.store(target, state)
            case ast.Pass() | ast.Global() | ast.Nonlocal():
                pass
            case ast.Raise(exc, cause):
                for expr in (exc, cause):
                    if expr is not None:
                        state = self.expr(expr, state)
                return None
            case ast.Try():
                return self.try_(stmt, state)
            case ast.With():
                return self.with_(stmt, state)
            case _:
                raise AssertionError(f"unexpected statement {stmt!r}")
        return state

    def try_(self, stmt: ast.Try, entry: _Flow) -> State:
        regions = try_regions(stmt)
        jumps = self._jump_marks()
        for region in regions:
            if region.part == "handled":
                entry = self.enter(region, entry)
        body_entry = entry
        for region in regions:
            if region.part == "body":
                body_entry = self.enter(region, entry)
        end_body = self.block(stmt.body, body_entry)
        # An exception can come from anywhere in the body, but a body that
        # evaluates one expression assigns nothing before it succeeds.
        if expression_body(stmt) is None:
            raised = _after_exception(body_entry, stmt.body)
        else:
            raised = body_entry
        ends: list[State] = []
        raises = not self._cannot_raise(stmt.body, body_entry)
        if self.recording:
            self.tries[stmt] = self.tries.get(stmt, False) or raises
        for handler in stmt.handlers if raises else ():
            state = raised
            if handler.type is not None:
                state = self.expr(handler.type, state)
            if handler.name is not None:
                state = self.assign_name(handler.name, state)
            end = self.block(handler.body, state)
            if end is not None and handler.name is not None:
                # The implicit `del name` at the end of the handler.
                end = _Flow(end.definite - {handler.name}, end.maybe - {handler.name})
            ends.append(end)
        end_else = self.block(stmt.orelse, end_body) if stmt.orelse else end_body
        normal = _merge(end_else, *ends)
        if not stmt.finalbody:
            return normal
        # `finally` also runs after exceptions and jumps out of any part.
        parts: list[ast.AST] = [*stmt.body, *stmt.handlers, *stmt.orelse]
        self.block(stmt.finalbody, _merge(normal, _after_exception(entry, parts)))
        self._through_finally(jumps, stmt.finalbody)
        if normal is None:
            return None
        return self.silently(lambda: self.block(stmt.finalbody, normal))

    def _cannot_raise(self, body: list[ast.stmt], state: _Flow) -> bool:
        """Whether running `body` in `state` can not raise: `pass`, or one
        statement storing a constant, an assigned local or a tuple of those
        in a local (`x = 1`, `y = a, b`), or returning or evaluating it."""
        if all(isinstance(stmt, ast.Pass) for stmt in body):
            return True
        if len(body) != 1:
            return False
        match body[0]:
            case ast.Expr(value) | ast.Return(ast.expr() as value):
                return self._cannot_fail(value, state)
            case ast.Assign([ast.Name(name)], value):
                return self._plain_local(name) and self._cannot_fail(value, state)
        return False

    def _cannot_fail(self, expr: ast.expr, state: _Flow) -> bool:
        match expr:
            case ast.Constant():
                return True
            case ast.Name(name, ast.Load()):
                return self._plain_local(name) and name in state.definite
            case ast.Tuple(elts, ast.Load()):
                return all(self._cannot_fail(elt, state) for elt in elts)
        return False

    def _plain_local(self, name: str) -> bool:
        """A local that is a plain Rust variable, not a cell."""
        return self.scope.is_local(name) and name not in self.scope.cells

    def _infallible_tries(self) -> None:
        """Record the `try` statements whose body never raised on any path
        analysed, and make their handlers unreachable."""
        for stmt, raises in self.tries.items():
            if raises:
                continue
            self.info.infallible_tries.add(stmt)
            for handler in stmt.handlers:
                self.info.unreachable.update(handler.body)

    def with_(self, stmt: ast.With, state: _Flow) -> State:
        entry = state
        for i, item in enumerate(stmt.items):
            state = self.expr(item.context_expr, state)
            state = self.enter(Region(stmt, i), state)
            if i == 0:
                entry = state
            if item.optional_vars is not None:
                state = self.store(item.optional_vars, state)
        end = self.block(stmt.body, state)
        # `__exit__` may suppress an exception from anywhere in the block.
        parts: list[ast.AST] = [*with_regions(stmt)[0].nodes()]
        return _merge(end, _after_exception(entry, parts))

    def enter(self, region: Region, state: _Flow) -> _Flow:
        """The state at the start of a closure region. Locals that live only
        in the region start out unassigned on every run of the closure."""
        if self.recording:
            previous = self.region_entry.get(region)
            self.region_entry[region] = state if previous is None else _merge(previous, state)  # type: ignore[assignment]
        inner = self.candidates.get(region, set())
        return _Flow(state.definite - inner, state.maybe - inner)

    def silently(self, analyse):  # type: ignore[no-untyped-def]
        saved = self.recording
        self.recording = False
        try:
            return analyse()
        finally:
            self.recording = saved

    def _jump_marks(self) -> tuple[int, int] | None:
        if not self.loops:
            return None
        loop = self.loops[-1]
        return len(loop.breaks), len(loop.continues)

    def _through_finally(self, marks: tuple[int, int] | None, finalbody: list[ast.stmt]) -> None:
        """`break` and `continue` in a `try` run its `finally` block first,
        which may assign or delete locals (approximated conservatively)."""
        if marks is None:
            return
        loop = self.loops[-1]
        assigned = _assigned_names(finalbody)
        deleted = _deleted_names(finalbody)

        def through(state: State) -> State:
            if state is None:
                return None
            return _Flow(state.definite - deleted, state.maybe | assigned)

        loop.breaks[marks[0] :] = [through(s) for s in loop.breaks[marks[0] :]]
        loop.continues[marks[1] :] = [through(s) for s in loop.continues[marks[1] :]]

    def loop(
        self,
        stmt: ast.While | ast.For,
        entry: _Flow,
        test: ast.expr | None = None,
        target: ast.expr | None = None,
    ) -> State:
        """`while` and `for`: iterate to a fixed point at the loop head."""
        head = entry
        while True:
            # The test runs before every iteration and before the loop ends.
            tested = head if test is None else self.expr(test, head)
            self.loops.append(_Loop())
            start: State = tested if target is None else self.store(target, tested)
            end = self.block(stmt.body, start)
            loop = self.loops.pop()
            new_head = _merge(entry, end, *loop.continues)
            assert new_head is not None
            if new_head == head:
                break
            head = new_head
        # The loop ends normally when the iterator is exhausted or the test is
        # false; `while True` only ends through `break` (or `return`).
        exhausted: State = None if test is not None and is_constant_true(test) else tested
        return _merge(self.block(stmt.orelse, exhausted), *loop.breaks)

    def store(self, target: ast.expr, state: _Flow) -> _Flow:
        match target:
            case ast.Name(id):
                return self.assign_name(id, state)
            case ast.Tuple(elts) | ast.List(elts):
                for elt in elts:
                    state = self.store(elt, state)
            case ast.Starred(value):
                return self.store(value, state)
            case ast.Attribute(value):
                state = self.expr(value, state)
            case ast.Subscript(value, index):
                state = self.expr(value, state)
                state = self.expr(index, state)
        return state

    def assign_name(self, name: str, state: _Flow) -> _Flow:
        if not self.scope.is_local(name):
            return state
        if name in state.maybe and self.recording:
            self.info.mutable.add(name)
            if name in self.scope.params:
                self.info.assigned_params.add(name)
        return _Flow(state.definite | {name}, state.maybe | {name})

    def delete(self, target: ast.expr, state: _Flow) -> _Flow:
        match target:
            case ast.Name(id) if self.scope.is_local(id):
                if id in state.definite:
                    self.info.assigned_deletes.add(target)
                return _Flow(state.definite - {id}, state.maybe - {id})
            case ast.Tuple(elts) | ast.List(elts):
                for elt in elts:
                    state = self.delete(elt, state)
            case ast.Attribute(value):
                state = self.expr(value, state)
            case ast.Subscript(value, index):
                state = self.expr(value, state)
                state = self.expr(index, state)
        return state

    def expr(self, expr: ast.expr, state: _Flow, hidden: frozenset[str] = frozenset()) -> _Flow:
        """Evaluate `expr`: its reads, and the assignments of `:=` in it, in
        evaluation order. Parts that may not run (the right side of `and`,
        a branch of a conditional expression, the body of a comprehension)
        may or may not assign. `hidden` are the variables of the
        comprehensions around `expr`, which are not locals of this scope."""
        match expr:
            case ast.Name(name, ast.Load()):
                if name not in hidden:
                    self.read_name(name, state)
                return state
            case ast.NamedExpr(ast.Name(name), value):
                state = self.expr(value, state, hidden)
                return self.assign_name(name, state)
            case ast.BoolOp(_, [first, *rest]):
                state = self.expr(first, state, hidden)
                ends = [state]
                for value in rest:
                    state = self.expr(value, state, hidden)
                    ends.append(state)
                return _merge(*ends)  # type: ignore[return-value]
            case ast.IfExp(test, body, orelse):
                state = self.expr(test, state, hidden)
                then = self.expr(body, state, hidden)
                return _merge(then, self.expr(orelse, state, hidden))  # type: ignore[return-value]
            case ast.Compare(left, _, [first, *rest]):
                state = self.expr(first, self.expr(left, state, hidden), hidden)
                ends = [state]
                for comparator in rest:
                    state = self.expr(comparator, state, hidden)
                    ends.append(state)
                return _merge(*ends)  # type: ignore[return-value]
            case ast.Lambda(args):
                for default in [*args.defaults, *args.kw_defaults]:
                    if default is not None:
                        state = self.expr(default, state, hidden)
                return state
            case ast.GeneratorExp(generators=generators):
                # The rest runs later, in the generator's own scope.
                return self.expr(generators[0].iter, state, hidden)
            case ast.ListComp() | ast.SetComp() | ast.DictComp():
                entry = self.expr(expr.generators[0].iter, state, hidden)
                inner = hidden | comprehension_variables(expr)
                head = entry
                while True:
                    end = self._comprehension_body(expr, head, inner)
                    new_head = _merge(entry, end)
                    assert new_head is not None
                    if new_head == head:
                        return head
                    head = new_head
            case ast.Dict(keys, values):
                for key, value in zip(keys, values, strict=True):
                    if key is not None:
                        state = self.expr(key, state, hidden)
                    state = self.expr(value, state, hidden)
                return state
        for child in ast.iter_child_nodes(expr):
            if isinstance(child, ast.expr):
                state = self.expr(child, state, hidden)
            elif isinstance(child, ast.keyword):
                state = self.expr(child.value, state, hidden)
        return state

    def _comprehension_body(
        self, node: ast.ListComp | ast.SetComp | ast.DictComp, state: _Flow, hidden: frozenset[str]
    ) -> _Flow:
        """One iteration of a comprehension's loops, as far as its value."""
        for index, generator in enumerate(node.generators):
            if index:
                state = self.expr(generator.iter, state, hidden)
            state = self.expr(generator.target, state, hidden)
            for test in generator.ifs:
                state = self.expr(test, state, hidden)
        if isinstance(node, ast.DictComp):
            # The key is evaluated first (since 3.8).
            return self.expr(node.value, self.expr(node.key, state, hidden), hidden)
        return self.expr(node.elt, state, hidden)

    def read_name(self, name: str, state: _Flow) -> None:
        if self.scope.is_local(name) and name not in state.definite and self.recording:
            self.info.optional.add(name)

    def _nested_frees(self, node: FunctionNode) -> tuple[str, ...]:
        if self.scopes is None:
            return ()
        return self.scopes.of(node).frees

    def _occurrences(self, stmt: ast.stmt) -> list[str]:
        """Names of this scope in `stmt`, in source order. A nested function
        counts as an occurrence of the variables it captures."""
        found = [
            (node.lineno, node.col_offset, name)
            for node in walk_scope(stmt)
            for name in _names_bound_or_used(node, self._nested_frees)
        ]
        found.sort(key=lambda item: item[:2])  # stable: a def's name before its frees
        return [name for _, _, name in found]

    def _binding_counts(self, body: list[ast.stmt]) -> Counter[str]:
        return Counter(
            name for stmt in body for node in walk_scope(stmt) for name in _bound_names(node)
        )

    def _local_names(self, target: ast.expr) -> list[str]:
        return [
            node.id
            for node in ast.walk(target)
            if isinstance(node, ast.Name)
            and not isinstance(node.ctx, ast.Load)
            and self.scope.is_local(node.id)
        ]

    def _loop_scoped(self, body: list[ast.stmt]) -> None:
        """Find `for` targets that can be declared inside the loop body."""
        bindings = self._binding_counts(body)
        loads: dict[str, list[ast.Name]] = {}
        loops: list[ast.For] = []
        for stmt in body:
            for node in walk_scope(stmt):
                match node:
                    case ast.Name(name, ast.Load()):
                        loads.setdefault(name, []).append(node)
                    case ast.For() if node not in self.info.unreachable:
                        loops.append(node)
        for loop in loops:
            names = _target_names(loop.target)
            if names is None or len(set(names)) != len(names):
                continue
            inside = {id(n) for stmt in loop.body for n in walk_scope(stmt)}
            if all(
                self.scope.is_local(name)
                and name not in self.scope.cells
                and name not in self.scope.params
                and name not in self.info.optional
                and bindings[name] == 1
                and all(id(load) in inside for load in loads.get(name, []))
                for name in names
            ):
                for name in names:
                    self.info.loop_scoped[name] = loop

    def _declarations(self, body: list[ast.stmt]) -> None:
        """Pick locals whose first occurrence is a top-level assignment."""
        info = self.info
        seen: set[str] = set(self.scope.params) | set(info.loop_scoped)
        elsewhere = set(info.scoped) | info.handler_scoped | info.with_params
        for stmt in body:
            first = [
                name
                for name in dict.fromkeys(self._occurrences(stmt))
                if name not in seen and self.scope.is_local(name)
            ]
            seen.update(first)
            info.order += first
            first = [name for name in first if name not in elsewhere]
            for name, node in self._declaring(stmt, first).items():
                if node is None:
                    info.declared_at_top.append(name)
                else:
                    info.declared_at[name] = node
        # Locals that live in a closure region are declared there, the same way.
        by_region: dict[Region, set[str]] = {}
        for name, region in info.scoped.items():
            if name not in info.with_params:
                by_region.setdefault(region, set()).add(name)
        for region, names in by_region.items():
            top = info.region_top.setdefault(region, [])
            stmts = _region_statements(region)
            for name in names:
                if not any(name in self._occurrences(stmt) for stmt in stmts):
                    top.append(name)
            seen = set(top)
            for stmt in stmts:
                first = [
                    name
                    for name in dict.fromkeys(self._occurrences(stmt))
                    if name in names and name not in seen
                ]
                seen.update(first)
                for name, node in self._declaring(stmt, first).items():
                    if node is None:
                        top.append(name)
                    else:
                        info.declared_at[name] = node
            top.sort(key=info.order.index)

    def _handler_scoped(self, body: list[ast.stmt]) -> set[str]:
        """`except .. as name` names whose every occurrence is inside a
        handler binding that name, where nothing else binds it."""
        handlers: dict[str, list[ast.ExceptHandler]] = {}
        for stmt in body:
            for node in walk_scope(stmt):
                if isinstance(node, ast.ExceptHandler) and node.name is not None:
                    handlers.setdefault(node.name, []).append(node)
        scoped = set()
        for name, bound in handlers.items():
            if not self.scope.is_local(name) or name in self.scope.cells:
                continue
            if name in self.scope.params:
                continue
            inside: set[int] = set()
            nested = False
            for handler in bound:
                for stmt in handler.body:
                    for node in walk_scope(stmt):
                        inside.add(id(node))
                        if isinstance(node, ast.ExceptHandler) and node.name == name:
                            nested = True
            if nested:
                continue
            ok = True
            for stmt in body:
                for node in walk_scope(stmt):
                    if node in bound:
                        continue
                    if name in _names_bound_or_used(node, self._nested_frees) and (
                        id(node) not in inside or name in _bound_names(node)
                    ):
                        ok = False
            if ok:
                scoped.add(name)
        return scoped

    def _region_candidates(self, body: list[ast.stmt]) -> None:
        """The locals each closure region could declare itself: all their
        occurrences are inside it (the innermost such region)."""
        regions = list(regions_in(body))
        if not regions:
            return
        ids = {region: region_node_ids(region) for region in regions}
        where: dict[str, list[int]] = {}
        for stmt in body:
            for node in walk_scope(stmt):
                for name in _names_bound_or_used(node, self._nested_frees):
                    where.setdefault(name, []).append(id(node))
        excluded = set(self.scope.params) | self.scope.cells | self.info.handler_scoped
        for name, nodes in where.items():
            if not self.scope.is_local(name) or name in excluded:
                continue
            containing = [r for r in regions if all(n in ids[r] for n in nodes)]
            if containing:
                innermost = min(containing, key=lambda r: len(ids[r]))
                self.candidates.setdefault(innermost, set()).add(name)

    def _regions(self, body: list[ast.stmt]) -> None:
        """Decide where locals assigned in closure regions are declared. A
        closure can not initialize a variable of the enclosing fn, so a local
        it assigns that is not assigned before it runs (and does not live in
        the region alone) has to be an `Option`."""
        info = self.info
        for region, names in self.candidates.items():
            for name in names:
                if name not in info.optional and name not in info.loop_scoped:
                    info.scoped[name] = region
        self._with_params(body)
        for region, entry in self.region_entry.items():
            inner = region_node_ids(region)
            for name in _assigned_names(region.nodes(), skip=info.unreachable):
                if not self.scope.is_local(name) or name in self.scope.cells:
                    continue
                if name in info.handler_scoped or name in info.loop_scoped:
                    continue
                if name in info.with_params:
                    continue
                home = info.scoped.get(name)
                if home is not None and _within(home, region, inner):
                    continue  # declared in this region or one inside it
                if name not in entry.definite:
                    info.optional.add(name)

    def _with_params(self, body: list[ast.stmt]) -> None:
        """`with .. as name` names that the closure of each `with` binding
        them can take as its parameter: nothing else binds them, they are
        only used inside those `with` statements, and no `with` binding the
        name is inside another one."""
        info = self.info
        targets: dict[str, list[Region]] = {}
        for region in self.region_entry:
            if isinstance(region.node, ast.With):
                target = region.node.items[region.part].optional_vars  # type: ignore[index]
                if isinstance(target, ast.Name):
                    targets.setdefault(target.id, []).append(region)
        if not targets:
            return
        counts = self._binding_counts(body)
        where: dict[str, list[int]] = {}
        for stmt in body:
            for node in walk_scope(stmt):
                for name in _names_bound_or_used(node, self._nested_frees):
                    where.setdefault(name, []).append(id(node))
        excluded = set(self.scope.params) | self.scope.cells | info.handler_scoped
        for name, regions in targets.items():
            if not self.scope.is_local(name) or name in excluded:
                continue
            if name in info.optional or name in info.loop_scoped or counts[name] != len(regions):
                continue
            ids = [region_node_ids(region) for region in regions]
            inside = set().union(*ids)
            if not all(node in inside for node in where.get(name, [])):
                continue
            if any(id(r.node) in other for r in regions for other in ids):
                continue  # a `with` binding it inside another
            info.with_params.add(name)

    def _declaring(
        self, stmt: ast.stmt, first: list[str]
    ) -> dict[str, ast.stmt | ast.NamedExpr | None]:
        """Where each of the locals `first` seen in the top-level statement
        `stmt` is declared with `let`: at `stmt`, at a `:=` in it, or at the
        top (`None`)."""
        if (
            isinstance(stmt, ast.Try)
            and stmt in self.info.infallible_tries
            and not stmt.finalbody
            and len(stmt.body) == 1
        ):
            # Its body runs in place, as if the `try` was not there.
            inner = self._declaring(stmt.body[0], first)
            return {name: inner.get(name) for name in first}
        declared = self._declarable(stmt, first)
        walrus: dict[str, ast.NamedExpr] = {}
        if stmt not in self.info.unreachable:
            for node in _unconditional_walrus(_header_expr(stmt)):
                name = node.target.id  # type: ignore[union-attr]
                if (
                    name in first
                    and name not in declared
                    and name not in walrus
                    and name not in self.info.optional
                    and name not in self.scope.cells
                ):
                    walrus[name] = node
        return {name: stmt if name in declared else walrus.get(name) for name in first}

    def _try_assigned(self, stmt: ast.Try) -> str | None:
        """`x` for a `try` that assigns it a value on every way out, which
        is then `let x = match .. { Ok(value) => value, .. }`: the body is
        `x = expr`, and each handler ends assigning `x` (only there) or
        leaves (`raise`, `return`, `break`, `continue`)."""
        if stmt.finalbody or stmt.orelse or expression_body(stmt) is None:
            return None
        match stmt.body[0]:
            case ast.Assign([ast.Name(name)]) if self._plain_local(name):
                pass
            case _:
                return None
        for handler in stmt.handlers:
            body = [s for s in handler.body if s not in self.info.unreachable]
            if not body or handler.name == name:
                return None
            match body[-1]:
                case ast.Assign([ast.Name(target)]) if target == name:
                    rest = [*body[:-1], body[-1].value]
                case ast.Raise() | ast.Return() | ast.Break() | ast.Continue():
                    rest = body
                case _:
                    return None
            if name in _assigned_names(rest) or any(
                isinstance(node, ast.Name) and node.id == name
                for part in rest
                for node in walk_scope(part)
            ):
                return None
        return name

    def _declarable(self, stmt: ast.stmt, first: list[str]) -> set[str]:
        """Names a reachable top-level assignment can declare with `let`."""
        if stmt in self.info.unreachable:
            return set()
        if isinstance(stmt, ast.FunctionDef):
            # Not if the function, its decorators or defaults refer to the name.
            read = {n.id for n in ast.walk(stmt) if isinstance(n, ast.Name)}
            read.update(self._nested_frees(stmt))
            name = stmt.name
            if name in first and name not in read and name not in self.info.optional:
                return {name}
            return set()
        if isinstance(stmt, ast.Try):
            name = self._try_assigned(stmt)
            if name is not None and name in first and name not in self.info.optional:
                return {name}
            return set()
        if isinstance(stmt, ast.Import | ast.ImportFrom):
            bound = [_import_name(alias) for alias in stmt.names]
            return {
                name
                for name in bound
                if name in first and bound.count(name) == 1 and name not in self.info.optional
            }
        if not isinstance(stmt, ast.Assign):
            return set()
        if len(stmt.targets) == 1 and isinstance(stmt.targets[0], ast.Tuple | ast.List):
            # A tuple target is declared by a single `let` pattern: all or nothing.
            names = _flat_target_names(stmt.targets[0], starred=True)
            if (
                names is None
                or len(set(names)) != len(names)
                or any(name not in first or name in self.info.optional for name in names)
            ):
                return set()
            return set(names)
        # An `Option` local (one that is deleted later) can be declared here too,
        # as long as the value does not read it first.
        read = set()
        for node in walk_scope(stmt.value):
            if isinstance(node, ast.Name):
                read.add(node.id)
            elif isinstance(node, FunctionNode):
                read.update(self._nested_frees(node))
        return {
            t.id
            for t in stmt.targets
            if isinstance(t, ast.Name) and t.id in first and t.id not in read
        }

    def _frozen(self, body: list[ast.stmt]) -> None:
        """Cells whose value is fixed by the time any nested function captures
        them: parameters that are never rebound, and locals bound exactly once
        by a top-level statement that comes before every capture."""
        scope = self.scope
        info = self.info
        rebound_inside = {
            name
            for inner in scope.descendants()
            for name in inner.nonlocals
            if inner.owner(name) is scope
        }
        bindings = self._binding_counts(body)
        for name in scope.cells:
            if name in rebound_inside or name in info.optional:
                continue
            if name in scope.params:
                once = bindings[name] == 0
            else:
                once = bindings[name] == 1 and name in info.declared_at
            if once and name not in info.mutable:
                info.frozen.add(name)


def _after_exception(state: _Flow, nodes: list[ast.AST]) -> _Flow:
    """The state where an exception raised anywhere in `nodes` (starting
    in `state`) is caught: anything they assign may or may not be."""
    return _Flow(state.definite - _deleted_names(nodes), state.maybe | _assigned_names(nodes))


def _assigned_names(nodes: list[ast.AST], skip: Collection[ast.AST] = ()) -> set[str]:
    """Names bound (or unbound) by `nodes`, not counting nested functions or
    the nodes in `skip`."""
    return {
        name for part in nodes for node in walk_scope(part, skip) for name in _bound_names(node)
    }


def _deleted_names(nodes: list[ast.AST]) -> set[str]:
    """Names `nodes` may unbind: `del name`, and the names of handlers."""
    names = set()
    for part in nodes:
        for node in walk_scope(part):
            match node:
                case ast.Name(id, ast.Del()):
                    names.add(id)
                case ast.ExceptHandler(name=str(name)):
                    names.add(name)
    return names


def _names_bound_or_used(node: ast.AST, frees) -> list[str]:  # type: ignore[no-untyped-def]
    """The names `node` itself reads, binds or captures (not its children):
    `frees(node)` are the variables a nested function captures."""
    match node:
        case ast.Name(id):
            return [id]
        case ast.FunctionDef() | ast.Lambda() | ast.GeneratorExp():
            return [*_bound_names(node), *frees(node)]
    return _bound_names(node)


def _header_expr(stmt: ast.stmt) -> ast.expr | None:
    """The expression a statement evaluates first, whose prelude runs right
    before the statement (not in a block or loop of its own)."""
    match stmt:
        case ast.Expr(value) | ast.Assign(value=value):
            return value
        case ast.If(test):
            return test
        case ast.For(iter=iter_):
            return iter_
    return None


def _unconditional_walrus(expr: ast.expr | None) -> list[ast.NamedExpr]:
    """The `:=` in `expr` that run whenever `expr` is evaluated, in order,
    leaving out those in parts with statements of their own: the right
    side of `and`/`or`, the branches of a conditional expression, the later
    comparisons of a chain, comprehensions, lambdas and displays with `*`."""
    match expr:
        case None | ast.Lambda() | ast.GeneratorExp():
            return []
        case ast.ListComp() | ast.SetComp() | ast.DictComp():
            return []
        case ast.NamedExpr(ast.Name(), value):
            return [*_unconditional_walrus(value), expr]
        case ast.BoolOp(_, [first, *_]):
            return _unconditional_walrus(first)
        case ast.IfExp(test):
            return _unconditional_walrus(test)
        case ast.Compare(left, _, [first, *_]):
            return _unconditional_walrus(left) + _unconditional_walrus(first)
        case ast.List(elts) | ast.Tuple(elts) | ast.Set(elts) if any(
            isinstance(elt, ast.Starred) for elt in elts
        ):
            return []
        case ast.Dict(keys) if None in keys:
            return []
    found = []
    for child in ast.iter_child_nodes(expr):
        if isinstance(child, ast.expr):
            found += _unconditional_walrus(child)
        elif isinstance(child, ast.keyword):
            found += _unconditional_walrus(child.value)
    return found


def _within(inner: Region, outer: Region, outer_ids: set[int]) -> bool:
    """Whether the region `inner` is `outer` or nested in it."""
    if inner.node is outer.node:
        if isinstance(inner.part, int) and isinstance(outer.part, int):
            return inner.part >= outer.part
        return inner.part == outer.part or outer.part == "handled"
    return id(inner.node) in outer_ids


def _region_statements(region: Region) -> list[ast.stmt]:
    """The top-level statements of a region's closure body."""
    match region.node, region.part:
        case ast.Try(), "body":
            return list(region.node.body)
        case ast.With(items=items, body=body), int(index) if index == len(items) - 1:
            return list(body)
    return []


def _import_name(alias: ast.alias) -> str:
    return alias.asname or alias.name.split(".")[0]


def _target_names(target: ast.expr) -> list[str] | None:
    """The names of a target made of names and (nested) tuples, else `None`."""
    match target:
        case ast.Name(id):
            return [id]
        case ast.Tuple(elts) | ast.List(elts):
            names = []
            for elt in elts:
                inner = _target_names(elt)
                if inner is None:
                    return None
                names += inner
            return names
    return None


def _flat_target_names(target: ast.expr, starred: bool = False) -> list[str] | None:
    """`x` or `a, b` (`a, *b` with `starred`): the names, else `None`."""
    match target:
        case ast.Name(id):
            return [id]
        case ast.Tuple(elts) | ast.List(elts):
            names = []
            for elt in elts:
                match elt:
                    case ast.Name(id):
                        names.append(id)
                    case ast.Starred(ast.Name(id)) if starred:
                        names.append(id)
                    case _:
                        return None
            return names
    return None


def is_constant_true(expr: ast.expr) -> bool:
    return isinstance(expr, ast.Constant) and bool(expr.value)


def analyze_locals(scope: FunctionScope, scopes: ModuleScopes | None = None) -> LocalsInfo:
    fn = scope.node
    assert fn is not None
    body = scope_body(fn)
    if isinstance(fn, ast.Lambda):
        # The body of a lambda is an expression returned right away.
        body = [ast.copy_location(ast.Return(fn.body), fn.body)]
    return _DefiniteAssignment(scope, scopes).run(body)  # type: ignore[arg-type]


def analyze_module_flow(module: ast.Module, python: Collection[ast.stmt] = ()) -> LocalsInfo:
    """Reachability of module level statements (there are no locals).
    Statements kept as Python source are opaque: control continues after them."""
    return _DefiniteAssignment(MODULE_SCOPE, opaque=python).run(module.body)


def analyze_class_flow(node: ast.ClassDef, python: Collection[ast.stmt] = ()) -> LocalsInfo:
    """Reachability of the statements of a class body, whose names all live
    in the class namespace, like module level code."""
    return _DefiniteAssignment(MODULE_SCOPE, opaque=python).run(node.body)
