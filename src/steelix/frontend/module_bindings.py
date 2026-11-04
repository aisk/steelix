"""Module level binding analysis that `symtable` does not provide: how many
times each global name is bound, and whether the module does anything
(`import *`, `globals()`, `setattr` on its own module object, `exec`) that
makes the set of globals unknowable. This decides which calls can be
direct Rust calls (design §7).
"""

from __future__ import annotations

import ast
from collections import Counter
from collections.abc import Collection
from dataclasses import dataclass

from steelix.frontend.desugar import binding_name

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
        for name in bound_names(node):
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
                for name in bound_names(node):
                    if name in declared_global:
                        counts[name] += 1
            for child in _scope_children(node):
                visit_nested_scope(child, inner)
            return
        for name in bound_names(node):
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


def bound_names(node: ast.AST) -> list[str]:
    """Names bound (or unbound) directly by `node`, not by its children. The
    one definition of a binding that the other helpers build on."""
    match node:
        case ast.Name(id, ast.Store() | ast.Del()):
            return [id]
        case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
            return [binding_name(node)]
        case ast.Import(names) | ast.ImportFrom(names=names):
            return [import_name(alias) for alias in names if alias.name != "*"]
        case ast.ExceptHandler(name=str(name)):
            return [name]
        case ast.MatchAs(name=str(name)) | ast.MatchStar(name=str(name)):
            return [name]
        case ast.MatchMapping(rest=str(name)):
            return [name]
        case ast.TypeAlias(ast.Name(id)):
            return [id]
    return []


def import_name(alias: ast.alias) -> str:
    return alias.asname or alias.name.split(".")[0]
