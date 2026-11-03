"""Lowering state shared by the expression and statement lowerings."""

from __future__ import annotations

import ast
from dataclasses import dataclass, field

from steelix.diagnostics import SourceLocation, UnsupportedError
from steelix.frontend.bindings import LocalsInfo, ModuleBindings
from steelix.frontend.callgraph import CallGraph
from steelix.frontend.fallback import FallbackPlan
from steelix.frontend.scope import ClassScope, FunctionNode, FunctionScope, ModuleScopes
from steelix.lower.names import CONTEXT, FUNCTION_RESERVED, RUST_KEYWORDS, Renamer
from steelix.rustast import nodes as rs


@dataclass
class ModuleContext:
    filename: str
    bindings: ModuleBindings
    callgraph: CallGraph
    fn_names: Renamer
    """Rust names of the module level functions."""
    scopes: ModuleScopes
    comments: dict[ast.stmt, list[str]] = field(default_factory=dict)
    """Python comments to emit before each statement."""
    future_annotations: bool = False
    """`from __future__ import annotations`: annotations are strings."""
    plan: FallbackPlan = field(default_factory=FallbackPlan)
    """The statements kept as Python source (design §13)."""
    source_lines: list[str] = field(default_factory=list)
    """The lines of the Python source, for the statements kept as Python."""
    annotation_index: dict[ast.AnnAssign, int] = field(default_factory=dict)
    """The index of each module level annotation in the module's
    `__annotate__` (`name: annotation` assignments in module scope)."""
    leading_comments: dict[ast.stmt, list[str]] = field(default_factory=dict)
    """The comments before each statement kept as Python (those inside it
    stay in its source)."""
    frozen: dict[FunctionScope, set[str]] = field(default_factory=dict)
    """Cells of each function that nested functions receive by value."""
    names: dict[FunctionScope, Renamer] = field(default_factory=dict)
    """Rust names of each lowered function's parameters and locals."""
    uses: set[str] = field(default_factory=set)
    """`use` paths the generated module needs besides the fixed ones."""
    class_mods: list[rs.Mod] = field(default_factory=list)
    """The Rust modules of the classes defined in module code."""
    mod_names: set[str] = field(default_factory=set)
    """Names of the Rust modules in `class_mods`."""
    static_attributes: dict[tuple[str, int], tuple[str, ...]] = field(default_factory=dict)
    """`__static_attributes__` of each class by qualified name and first line."""
    varnames: dict[tuple[str, int], tuple[str, ...]] = field(default_factory=dict)
    """`co_varnames` of each function by qualified name and first line."""
    traceback_lines: bool = False
    """Emit `frame.line(n)` before fallible statements so tracebacks show
    the line of the statement; otherwise native frames report their `def`
    line (design §9)."""

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return UnsupportedError(feature, SourceLocation.of(self.filename, node))


@dataclass
class FunctionInfo:
    """What lowering needs to know about the function being lowered."""

    node: FunctionNode
    scope: FunctionScope
    locals: LocalsInfo
    names: Renamer
    """Rust names of parameters, captured variables and locals."""
    by_value: set[str] = field(default_factory=set)
    """Captured variables received by value rather than as `rt::Cell`."""
    walrus_targets: frozenset[str] = frozenset()
    """Locals that `:=` assigns somewhere in the function: reading one is
    not pure, an assignment expression later in the same expression may
    change it."""
    bool_locals: frozenset[str] = frozenset()
    """Locals that are a Rust `bool` (`frontend.booleans`, design §5)."""
    kind: str = "function"
    """`function`, `coroutine` or `generator` (`scope.function_kind`)."""

    @property
    def is_async(self) -> bool:
        """Whether the Rust fn is an `async fn`: a coroutine or a generator."""
        return self.kind != "function"

    def local_type(self, name: str) -> str:
        """The Rust type of an ordinary (not `Option`) local."""
        return "bool" if name in self.bool_locals else "Bound<'py, PyAny>"

    def is_cell(self, name: str) -> bool:
        """Whether `name` is a `rt::Cell` here: an owned cell that nested
        functions may rebind, or a captured one."""
        if self.scope.is_free(name):
            return name not in self.by_value
        return name in self.scope.cells and name not in self.locals.frozen

    @property
    def varargs(self) -> str | None:
        if isinstance(self.node, ast.GeneratorExp):
            return None
        args = self.node.args
        return args.vararg.arg if args.vararg is not None else None

    @property
    def varkw(self) -> str | None:
        if isinstance(self.node, ast.GeneratorExp):
            return None
        args = self.node.args
        return args.kwarg.arg if args.kwarg is not None else None


@dataclass
class ClassBody:
    """What lowering a class body needs to know: its names live in the
    class namespace `ns`, and its methods are items of the class's Rust
    module."""

    scope: ClassScope
    mod: str
    """The name of the class's Rust module."""
    method_names: dict[ast.AST, str]
    """The Rust names of the functions defined in the body."""
    items: list[rs.Item] = field(default_factory=list)
    """Methods and modules of nested classes, in source order."""
    mod_names: set[str] = field(default_factory=set)


NAMESPACE = "ns"
"""The `&rt::Namespace<'py>` parameter of a class body."""


@dataclass
class Context:
    """Lowering state for one Rust function body: a `def` or `exec_module`."""

    module: ModuleContext
    flow: LocalsInfo
    """Reachability (and, in functions, the declarations of locals)."""
    function: FunctionInfo | None = None
    """`None` while lowering module level code."""
    context_name: str = CONTEXT
    """The `&rt::Module<'py>` parameter, `m` unless a Python local is `m`."""
    statement_index: int | None = None
    """Index of the current top-level statement, for module level code."""
    items: list[rs.Fn] = field(default_factory=list)
    """Nested functions waiting to be declared before the current statement."""
    item_names: set[str] = field(default_factory=set)
    class_body: ClassBody | None = None
    """Set while lowering the body of a class."""
    _temps: set[str] = field(default_factory=set)
    pinned: set[str] = field(default_factory=set)
    """Temporaries holding objects that something else keeps alive anyway
    (a module, a builtin): `lower.temps` leaves them alone."""
    _labels: set[str] = field(default_factory=set)

    @property
    def at_module_level(self) -> bool:
        return self.function is None and self.class_body is None

    def namespace(self) -> str:
        """What code kept as Python runs in: the class namespace or the module."""
        return NAMESPACE if self.class_body is not None else self.context()

    def lifetime(self) -> str:
        """The lifetime of `Bound` values in closure signatures."""
        return "'_" if self.at_module_level else "'py"

    def py(self) -> str:
        """The `Python<'py>` token, bound by `bind_py` where the body uses it."""
        return "py"

    def context(self) -> str:
        """The name of the `&rt::Module<'py>` parameter every generated fn has."""
        return self.context_name

    def temp(self, base: str) -> str:
        """A fresh Rust name for a temporary that can not clash with locals,
        functions or names the generated code relies on."""
        taken = set(self.module.fn_names.mapping.values()) | FUNCTION_RESERVED | self.item_names
        taken.add(self.context_name)
        if self.function is not None:
            taken.update(self.function.names.mapping.values())
        if not base.isidentifier() or not base.isascii():
            base = "value"
        if not base.strip("_0123456789"):
            base = "underscore"  # not `__`, which clippy finds undescriptive
        if base in RUST_KEYWORDS or base in ("_", "self", "Self", "crate", "super"):
            base += "_"
        name = base
        n = 1
        while name in taken or name in self._temps:
            n += 1
            name = f"{base}{n}"
        self._temps.add(name)
        return name

    def reserve(self, name: str) -> None:
        """Keep temporaries from taking `name`, a variable of the generated
        code such as a comprehension's."""
        self._temps.add(name)

    def release(self, name: str) -> None:
        """Let temporaries take `name` again, once it is out of scope."""
        self._temps.discard(name)
        self.pinned.discard(name)

    def taken_temps(self) -> set[str]:
        return self._temps

    def binding(self, base: str) -> str:
        """A name for a short-lived binding of the generated code, such as
        the `err` of `Err(err) => ..`, that may not hide locals or
        temporaries. Unlike `temp`, the name is not used up: every `match`
        arm can bind its own `err`."""
        taken = set(self.module.fn_names.mapping.values()) | FUNCTION_RESERVED | self._temps
        taken |= self.item_names
        taken.add(self.context_name)
        if self.function is not None:
            taken.update(self.function.names.mapping.values())
        name = base
        n = 1
        while name in taken:
            n += 1
            name = f"{base}{n}"
        return name

    def item_name(self, base: str, allowed: str | None = None) -> str:
        """A fresh name for a nested `fn` item, which must not be shadowed by
        locals or temporaries of the enclosing function (`allowed` is a local
        declared by the very statement that defines the item)."""
        taken = set(self.module.fn_names.mapping.values()) | FUNCTION_RESERVED | self._temps
        taken.add(self.context_name)
        taken |= self.item_names
        if self.function is not None:
            taken.update(v for v in self.function.names.mapping.values() if v != allowed)
        name = base
        n = 1
        while name in taken:
            n += 1
            numbered = base in ("lambda", "genexpr")
            name = f"{base}{n}" if numbered else name + "_"
        self.item_names.add(name)
        return name

    def label(self, base: str) -> str:
        """A fresh loop or block label such as `'for_else`."""
        name = base
        n = 1
        while name in self._labels:
            n += 1
            name = f"{base}{n}"
        self._labels.add(name)
        return "'" + name

    def unsupported(self, feature: str, node: ast.AST) -> UnsupportedError:
        return self.module.unsupported(feature, node)
