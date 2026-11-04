"""A static index of who reads and writes the attributes of the project's
classes (design §15, "被多个模块共享的类").

Before a class is rewritten as a `#[pyclass]` (the second step of §15),
every function that reads or writes its instances has to be known. This
module lists, for each class of the project, each attribute and the
functions that read it, write it, assign it with an augmented assignment or
delete it, with file and line.

An access is attributed to a class only when the receiver's class is known
conservatively:

- `self` in a method (the first parameter of a function in the class body,
  as for `__static_attributes__`) and `cls` in a classmethod, also inside
  nested functions and lambdas that do not rebind them;
- a parameter annotated with the class (`p: Point`, `p: "Point"`,
  `p: Point | None`, `Optional[Point]`);
- a local whose every binding is `x = Point(..)` or `x: Point = ..`;
- the class object itself (`Point.x`, `type(self).x`, `self.__class__.x`).

A name only resolves to a class when the module binds it once (by the
`class` statement or a `from .. import`), the module's globals can not be
bound in ways the analysis does not see, and no other module assigns it.
The inferred receivers (annotations, constructed locals) are marked, since
an annotation is not enforced. Accesses through a receiver of unknown class
whose attribute name is an attribute of some project class are listed
separately, so that a reviewer only has to check those. Dynamic access
(`getattr`/`setattr`/`hasattr`/`delattr` with a computed name, `vars(x)`,
`x.__dict__`, `object.__setattr__`) is reported per class when the receiver
is known and per function otherwise.
"""

from __future__ import annotations

import ast
from collections.abc import Iterator
from dataclasses import dataclass, field

from steelix.frontend.classinfo import compiler_info
from steelix.frontend.fallback import first_line
from steelix.frontend.module_bindings import ModuleBindings, analyze_module_bindings
from steelix.project import Project, _module_aliases, _module_object, resolve_relative

READ = "read"
WRITE = "write"
AUGMENTED = "augmented"
DELETE = "del"
_DYNAMIC_KINDS = {"getattr": READ, "hasattr": READ, "setattr": WRITE, "delattr": DELETE}
_HOOKS = ("__getattr__", "__getattribute__", "__setattr__", "__delattr__")
_IMPLICIT_CLASSMETHODS = frozenset({"__init_subclass__", "__class_getitem__", "__new__"})


@dataclass(frozen=True)
class Receiver:
    """What an expression is known to evaluate to: an instance of `cls`, or
    `cls` itself (`is_class`)."""

    cls: ClassInfo
    is_class: bool
    via: str
    """`self`, `cls`, `class` (the class object by name), `type`
    (`type(self)`), `constructed` or `annotation`."""
    evidence: str
    """Why, for inferred receivers: `ctx = Context(..)`, `p: Point`."""


@dataclass(frozen=True)
class Access:
    """One read, write, augmented assignment or deletion of an attribute,
    or a dynamic access (`attribute` is `None` when the name is computed)."""

    attribute: str | None
    kind: str
    """`read`, `write`, `augmented`, `del`, or for calls the function name
    (`getattr`, `setattr`, `hasattr`, `delattr`, `vars`, `__dict__`,
    `object.__setattr__`, ..)."""
    module: str
    file: str
    line: int
    function: str
    """The qualified name of the function the access is in, `<module>` for
    module level code, the class's qualified name for a class body."""
    receiver: str
    """The source of the receiver expression."""
    via: str = "unknown"
    evidence: str = ""

    @property
    def location(self) -> str:
        return f"{self.file}:{self.line}"

    @property
    def inferred(self) -> bool:
        return self.via in ("constructed", "annotation")

    @property
    def effect(self) -> str:
        """`read`, `write`, `augmented` or `del`, also for `getattr` and co."""
        return _DYNAMIC_KINDS.get(self.kind, self.kind)

    def to_json(self) -> dict:
        data = {
            "attribute": self.attribute,
            "kind": self.kind,
            "module": self.module,
            "file": self.file,
            "line": self.line,
            "function": self.function,
            "receiver": self.receiver,
            "via": self.via,
        }
        if self.evidence:
            data["evidence"] = self.evidence
        return data


@dataclass
class Member:
    """An attribute of a class with its accesses."""

    name: str
    instance: bool
    """An instance attribute (assigned on instances), otherwise a class
    attribute read or written through the class or its instances."""
    static: bool = False
    """In `__static_attributes__` (or a dataclass field, or `__slots__`)."""
    class_line: int | None = None
    """The line of the class body statement that binds it, if any."""
    accesses: list[Access] = field(default_factory=list)


@dataclass
class ClassInfo:
    module: str
    qualname: str
    file: str
    node: ast.ClassDef
    location: str
    """`top level`, `conditional` (in a module level `if`, `try`, ..),
    `nested` (in a class) or `function` (in a function)."""
    static_attributes: tuple[str, ...] = ()
    """Runtime names (private names mangled)."""
    fields: tuple[str, ...] = ()
    """Annotated names of the class body, the fields of a dataclass."""
    slots: tuple[str, ...] | None = None
    methods: dict[str, list[ast.FunctionDef | ast.AsyncFunctionDef]] = field(default_factory=dict)
    class_attributes: dict[str, int] = field(default_factory=dict)
    """Names the class body binds other than methods, with their line."""
    body_logic: list[tuple[int, str]] = field(default_factory=list)
    """Class body statements other than defs, docstrings, constant
    assignments and annotations."""
    nested: dict[str, ClassInfo] = field(default_factory=dict)
    bases: list[ClassInfo | None] = field(default_factory=list)
    """The project class of each base, `None` for other bases."""
    subclasses: list[ClassInfo] = field(default_factory=list)
    members: dict[str, Member] = field(default_factory=dict)
    dynamic: list[Access] = field(default_factory=list)
    """Dynamic accesses on receivers of this class."""
    method_uses: dict[str, list[Access]] = field(default_factory=dict)
    """Reads of the class's methods through known receivers (calls, mostly)."""
    constructed: list[Access] = field(default_factory=list)
    """Calls of the class (by a name known to be the class, or `cls(..)`)."""

    @property
    def name(self) -> str:
        return self.node.name

    @property
    def full_name(self) -> str:
        return f"{self.module}.{self.qualname}"

    @property
    def line(self) -> int:
        return self.node.lineno

    @property
    def base_sources(self) -> list[str]:
        return [ast.unparse(b) for b in self.node.bases]

    @property
    def keywords(self) -> list[str]:
        return [
            f"{k.arg}={ast.unparse(k.value)}" if k.arg else f"**{ast.unparse(k.value)}"
            for k in self.node.keywords
        ]

    @property
    def decorators(self) -> list[str]:
        return [ast.unparse(d) for d in self.node.decorator_list]

    @property
    def dataclass(self) -> ast.expr | None:
        """The `@dataclass` decorator, if any."""
        for decorator in self.node.decorator_list:
            target = decorator.func if isinstance(decorator, ast.Call) else decorator
            if ast.unparse(target) in ("dataclass", "dataclasses.dataclass"):
                return decorator
        return None

    @property
    def hooks(self) -> list[str]:
        """The attribute access hooks the class defines."""
        return [name for name in _HOOKS if name in self.methods]

    def instance_attributes(self) -> list[Member]:
        return [m for m in self.members.values() if m.instance]

    def class_members(self) -> list[Member]:
        return [m for m in self.members.values() if not m.instance]

    def mro(self) -> Iterator[ClassInfo]:
        """The class and its project bases, depth first (an approximation of
        the MRO that is good enough for finding where a name is defined)."""
        seen: set[int] = set()
        pending = [self]
        while pending:
            cls = pending.pop(0)
            if id(cls) in seen:
                continue
            seen.add(id(cls))
            yield cls
            pending[0:0] = [b for b in cls.bases if b is not None]

    def all_subclasses(self) -> list[ClassInfo]:
        found: list[ClassInfo] = []
        pending = list(self.subclasses)
        while pending:
            cls = pending.pop(0)
            if cls not in found:
                found.append(cls)
                pending += cls.subclasses
        return found

    def own_function(self, function: str) -> bool:
        """Whether `function` is one of the class's methods (or a function
        nested in one, or the class body)."""
        return function == self.qualname or function.startswith(self.qualname + ".")


@dataclass
class AttributeIndex:
    project: Project
    classes: list[ClassInfo]
    unknown: list[Access]
    """Accesses through receivers of unknown class whose attribute name is an
    instance attribute of a project class."""
    unknown_methods: list[Access]
    """Reads through receivers of unknown class of names that are methods of
    project classes."""
    dynamic: list[Access]
    """Dynamic accesses whose receiver's class is unknown."""

    def find(self, name: str) -> list[ClassInfo]:
        """The classes `name` designates: `Point`, `shapes.Point`,
        `pkg.shapes.Point`, `Outer.Inner` (a dotted suffix of the full name)."""
        return [c for c in self.classes if c.full_name == name or c.full_name.endswith("." + name)]

    def candidates(self, attribute: str) -> list[ClassInfo]:
        """The classes that have an instance attribute named `attribute`."""
        return [c for c in self.classes if attribute in c.members and c.members[attribute].instance]

    def method_candidates(self, name: str) -> list[ClassInfo]:
        """The classes that define a method named `name`."""
        return [c for c in self.classes if name in c.methods]


def build_index(project: Project) -> AttributeIndex:
    """The attribute index of all modules of `project`."""
    return _Indexer(project).run()


# --- analysis ----------------------------------------------------------------------


@dataclass
class _Module:
    name: str
    file: str
    package: str
    tree: ast.Module
    bindings: ModuleBindings
    classes: dict[str, ClassInfo] = field(default_factory=dict)
    """Classes bound at module level by their `class` statement."""
    imports: dict[str, tuple[str, str]] = field(default_factory=dict)
    """`from base import name as alias`: alias -> (base, name)."""
    module_names: set[str] = field(default_factory=set)
    """Names bound to module objects by `import` statements."""
    patched: set[str] = field(default_factory=set)
    """Module globals other modules may assign (`mod.Name = ..`)."""


@dataclass
class _Raw:
    attribute: str | None
    kind: str
    receiver: Receiver | None
    access: Access


@dataclass
class _Scope:
    label: str
    env: dict[str, Receiver]
    local: set[str]
    """Names local to the function (or class body); for module level code
    it is empty and names are globals."""
    is_function: bool
    mangle: str | None
    """The class name private names are mangled with."""
    hidden: set[str] = field(default_factory=set)
    """Locals of the enclosing functions, which hide globals of the same name."""
    outer: _Scope | None = None
    """For a class body, the scope the class statement is in."""
    class_node: ast.ClassDef | None = None


class _Indexer:
    def __init__(self, project: Project) -> None:
        self.project = project
        self.modules: dict[str, _Module] = {}
        self.classes: list[ClassInfo] = []
        self.raw: list[_Raw] = []

    def run(self) -> AttributeIndex:
        sources = self.project.sources()
        names = {name: name for name in self.project.modules}
        names["__main__"] = self.project.main.name
        for source in sources:
            parsed = self.project.parsed[source.relpath]
            module = _Module(
                source.name,
                source.relpath,
                source.package_name,
                parsed.tree,
                analyze_module_bindings(parsed.tree),
            )
            self.modules[source.name] = module
            info = compiler_info(parsed.source, parsed.filename)
            self._collect_classes(module, info.static_attributes)
            self._collect_imports(module)
        for source in sources:
            tree = self.project.parsed[source.relpath].tree
            aliases = _module_aliases(tree, source, names)
            for node in ast.walk(tree):
                match node:
                    case ast.Attribute(obj, attr, ast.Store() | ast.Del()):
                        target = _module_object(obj, aliases, names)
                        if target in self.modules:
                            self.modules[target].patched.add(attr)
                    case ast.Call(ast.Name("setattr" | "delattr"), [obj, key, *_]):
                        target = _module_object(obj, aliases, names)
                        if target in self.modules:
                            if isinstance(key, ast.Constant) and isinstance(key.value, str):
                                self.modules[target].patched.add(key.value)
                            else:
                                self.modules[target].bindings.patches_itself = True
        for cls in self.classes:
            module = self.modules[cls.module]
            cls.bases = [self._resolve_base(module, cls, base) for base in cls.node.bases]
            for base in cls.bases:
                if base is not None and cls not in base.subclasses:
                    base.subclasses.append(cls)
        for module in self.modules.values():
            _Walker(self, module).run()
        return self._classify()

    # --- classes and imports

    def _collect_classes(self, module: _Module, static: dict[tuple[str, int], tuple]) -> None:
        def visit(nodes: list[ast.stmt], prefix: str, where: str, outer: ClassInfo | None) -> None:
            for node in nodes:
                if isinstance(node, ast.ClassDef):
                    qualname = prefix + node.name
                    cls = ClassInfo(module.name, qualname, module.file, node, where)
                    names = static.get((qualname, first_line(node)), ())
                    cls.static_attributes = tuple(_mangle(n, node.name) for n in names)
                    _describe_body(cls)
                    self.classes.append(cls)
                    if outer is not None:
                        outer.nested[node.name] = cls
                    elif where in ("top level", "conditional"):
                        module.classes[node.name] = cls
                    visit(node.body, qualname + ".", "nested", cls)
                elif isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
                    inner = f"{prefix}{node.name}.<locals>."
                    visit(node.body, inner, "function", None)
                else:
                    children = [c for c in ast.iter_child_nodes(node) if isinstance(c, ast.stmt)]
                    for handler in getattr(node, "handlers", []):
                        children += handler.body
                    for case in getattr(node, "cases", []):
                        children += case.body
                    if where == "top level":
                        visit(children, prefix, "conditional", outer)
                    else:
                        visit(children, prefix, where, outer)

        visit(module.tree.body, "", "top level", None)

    def _collect_imports(self, module: _Module) -> None:
        for node in _module_level(module.tree.body):
            if isinstance(node, ast.ImportFrom):
                base = (
                    resolve_relative(node.module, node.level, module.package)
                    if node.level
                    else node.module
                )
                if base is None:
                    continue
                for alias in node.names:
                    if alias.name != "*":
                        module.imports[alias.asname or alias.name] = (base, alias.name)
            elif isinstance(node, ast.Import):
                for alias in node.names:
                    module.module_names.add(alias.asname or alias.name.split(".")[0])

    def global_class(self, module: _Module, name: str, depth: int = 0) -> ClassInfo | None:
        """The project class the module global `name` is bound to, if that
        is certain."""
        if depth > 10 or module.bindings.dynamic or module.bindings.counts[name] != 1:
            return None
        if name in module.patched:
            return None
        if name in module.classes:
            return module.classes[name]
        if name in module.imports:
            base, original = module.imports[name]
            if base in self.modules and f"{base}.{original}" not in self.modules:
                return self.global_class(self.modules[base], original, depth + 1)
        return None

    def global_module(self, module: _Module, name: str) -> str | None:
        """The module the module global `name` is bound to by an import:
        a project module's name, or `?` for other modules."""
        if module.bindings.counts[name] != 1:
            return None
        if name in module.module_names:
            return name if name in self.modules else "?"
        if name in module.imports:
            base, original = module.imports[name]
            full = f"{base}.{original}"
            if full in self.modules:
                return full
        return None

    def _resolve_base(self, module: _Module, cls: ClassInfo, base: ast.expr) -> ClassInfo | None:
        if cls.location in ("top level", "conditional", "nested"):
            return self.class_expr(module, base)
        return None

    def class_expr(self, module: _Module, expr: ast.expr) -> ClassInfo | None:
        """The project class a module level expression names: `Point`,
        `shapes.Point`, `Outer.Inner`."""
        match expr:
            case ast.Name(name):
                return self.global_class(module, name)
            case ast.Attribute(value, attr):
                outer = self.class_expr(module, value)
                if outer is not None:
                    return outer.nested.get(attr)
                target = self.module_expr(module, value)
                if target is not None:
                    return self.global_class(self.modules[target], attr)
        return None

    def module_expr(self, module: _Module, expr: ast.expr) -> str | None:
        match expr:
            case ast.Name(name):
                found = self.global_module(module, name)
                return found if found in self.modules else None
            case ast.Attribute(value, attr):
                base = self.module_expr(module, value)
                if base is not None and f"{base}.{attr}" in self.modules:
                    return f"{base}.{attr}"
        return None

    # --- members

    def _classify(self) -> AttributeIndex:
        # Instance attributes: assigned on instances anywhere the receiver
        # is known, or declared (`__static_attributes__`, fields, slots).
        for cls in self.classes:
            for name in (*cls.static_attributes, *cls.fields, *(cls.slots or ())):
                cls.members.setdefault(name, Member(name, True, static=True))
        for raw in self.raw:
            receiver = raw.receiver
            if receiver is None or receiver.is_class or raw.attribute is None:
                continue
            if raw.access.effect in (WRITE, AUGMENTED, DELETE):
                owner = self._instance_owner(receiver.cls, raw.attribute) or receiver.cls
                owner.members.setdefault(raw.attribute, Member(raw.attribute, True))
        for cls in self.classes:
            for name, line in cls.class_attributes.items():
                member = cls.members.setdefault(name, Member(name, False))
                member.class_line = line

        unknown: list[Access] = []
        unknown_methods: list[Access] = []
        dynamic: list[Access] = []
        instance_names = {m.name for c in self.classes for m in c.members.values() if m.instance}
        method_names = {name for c in self.classes for name in c.methods}
        for raw in self.raw:
            receiver = raw.receiver
            if raw.attribute is None:
                if receiver is None:
                    dynamic.append(raw.access)
                else:
                    receiver.cls.dynamic.append(raw.access)
                continue
            if receiver is None:
                if raw.attribute in instance_names:
                    unknown.append(raw.access)
                elif (
                    raw.attribute in method_names
                    and raw.access.effect == READ
                    and not (raw.attribute.startswith("__") and raw.attribute.endswith("__"))
                ):
                    unknown_methods.append(raw.access)
                continue
            member = self._member(receiver, raw.attribute, raw.access.effect)
            if member is not None:
                member.accesses.append(raw.access)
            elif (owner := self._method_owner(receiver.cls, raw.attribute)) is not None:
                owner.method_uses.setdefault(raw.attribute, []).append(raw.access)
        for cls in self.classes:
            for member in cls.members.values():
                # The class's own module first.
                member.accesses.sort(key=lambda a, c=cls: (a.file != c.file, *_order(a)))
            cls.members = dict(sorted(cls.members.items()))
            cls.dynamic.sort(key=_order)
            cls.constructed.sort(key=lambda a, c=cls: (a.file != c.file, *_order(a)))
            for accesses in cls.method_uses.values():
                accesses.sort(key=lambda a, c=cls: (a.file != c.file, *_order(a)))
            cls.method_uses = dict(sorted(cls.method_uses.items()))
        unknown.sort(key=_order)
        unknown_methods.sort(key=_order)
        dynamic.sort(key=_order)
        return AttributeIndex(self.project, self.classes, unknown, unknown_methods, dynamic)

    def _method_owner(self, cls: ClassInfo, name: str) -> ClassInfo | None:
        for c in cls.mro():
            if name in c.methods:
                return c
        return None

    def _instance_owner(self, cls: ClassInfo, name: str) -> ClassInfo | None:
        for c in cls.mro():
            if name in c.static_attributes or name in c.fields or name in (c.slots or ()):
                return c
        return None

    def _member(self, receiver: Receiver, name: str, effect: str) -> Member | None:
        """The member an access belongs to, `None` for methods read through
        an instance or the class."""
        for cls in receiver.cls.mro():
            member = cls.members.get(name)
            if not receiver.is_class and member is not None and member.instance:
                return member
            if name in cls.methods and effect == READ:
                return None
            if member is not None and not member.instance:
                if receiver.is_class or effect == READ:
                    return member
                # Assigning through an instance creates an instance
                # attribute, recorded on the class itself.
                return receiver.cls.members[name]
            if name in cls.methods:
                if receiver.is_class:
                    # A method replaced through the class.
                    return cls.members.setdefault(name, Member(name, False))
                return receiver.cls.members[name]
        if receiver.is_class:
            return receiver.cls.members.setdefault(name, Member(name, False))
        return receiver.cls.members.setdefault(name, Member(name, True))


def _is_super(expr: ast.expr) -> bool:
    """`super()` or `super(C, self)`: the receiver is a proxy for a base."""
    return (
        isinstance(expr, ast.Call) and isinstance(expr.func, ast.Name) and expr.func.id == "super"
    )


def _order(access: Access) -> tuple:
    return (access.file, access.line, access.function, access.attribute or "")


def _mangle(name: str, class_name: str) -> str:
    """The runtime name of `name` used in class `class_name` (`_Py_Mangle`)."""
    if not name.startswith("__") or name.endswith("__") or "." in name:
        return name
    stripped = class_name.lstrip("_")
    return f"_{stripped}{name}" if stripped else name


def _module_level(nodes: list[ast.stmt]) -> Iterator[ast.stmt]:
    """The statements that run at module level, also inside `if`, `try`, ..."""
    for node in nodes:
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
            continue
        yield node
        children = [c for c in ast.iter_child_nodes(node) if isinstance(c, ast.stmt)]
        for handler in getattr(node, "handlers", []):
            children += handler.body
        for case in getattr(node, "cases", []):
            children += case.body
        yield from _module_level(children)


def _is_constant(expr: ast.expr | None) -> bool:
    match expr:
        case ast.Constant():
            return True
        case ast.Tuple(elts) | ast.List(elts) | ast.Set(elts):
            return all(_is_constant(e) for e in elts)
        case ast.Dict(keys, values):
            return all(k is not None and _is_constant(k) for k in keys) and all(
                _is_constant(v) for v in values
            )
        case ast.UnaryOp(ast.USub() | ast.UAdd() | ast.Not() | ast.Invert(), operand):
            return _is_constant(operand)
    return False


def _describe_body(cls: ClassInfo) -> None:
    """Fill in what the class body defines."""
    for index, stmt in enumerate(cls.node.body):
        match stmt:
            case ast.FunctionDef() | ast.AsyncFunctionDef():
                cls.methods.setdefault(stmt.name, []).append(stmt)
                continue
            case ast.Expr(ast.Constant(str())) if index == 0:
                continue
            case ast.Pass():
                continue
            case ast.ClassDef():
                cls.class_attributes[stmt.name] = stmt.lineno
                cls.body_logic.append((stmt.lineno, f"nested class {stmt.name}"))
                continue
            case ast.AnnAssign(ast.Name(name), annotation, value):
                if not _is_class_var(annotation):
                    cls.fields = (*cls.fields, _mangle(name, cls.name))
                if value is not None:
                    cls.class_attributes[_mangle(name, cls.name)] = stmt.lineno
                if value is None or _is_constant(value) or _is_field_call(value):
                    continue
            case ast.Assign(targets, value) if all(isinstance(t, ast.Name) for t in targets):
                for target in targets:
                    assert isinstance(target, ast.Name)
                    cls.class_attributes[_mangle(target.id, cls.name)] = stmt.lineno
                    if target.id == "__slots__":
                        cls.slots = _slot_names(value, cls.name)
                if _is_constant(value):
                    continue
        for node in ast.walk(stmt):
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store | ast.Del):
                cls.class_attributes.setdefault(_mangle(node.id, cls.name), stmt.lineno)
        text = ast.unparse(stmt).splitlines()[0]
        if len(text) > 60:
            text = text[:57] + "..."
        cls.body_logic.append((stmt.lineno, text))


def _is_class_var(annotation: ast.expr) -> bool:
    target = annotation.value if isinstance(annotation, ast.Subscript) else annotation
    text = ast.unparse(target)
    return text.rsplit(".", 1)[-1] in ("ClassVar", "InitVar")


def _is_field_call(value: ast.expr) -> bool:
    return isinstance(value, ast.Call) and ast.unparse(value.func) in (
        "field",
        "dataclasses.field",
    )


def _slot_names(value: ast.expr, class_name: str) -> tuple[str, ...]:
    if isinstance(value, ast.Constant) and isinstance(value.value, str):
        return (_mangle(value.value, class_name),)
    if isinstance(value, ast.Tuple | ast.List | ast.Set):
        return tuple(
            _mangle(e.value, class_name)
            for e in value.elts
            if isinstance(e, ast.Constant) and isinstance(e.value, str)
        )
    return ()


# --- walking functions -------------------------------------------------------------


_FunctionNode = ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda
_Comprehension = ast.ListComp | ast.SetComp | ast.DictComp | ast.GeneratorExp


def _bindings(fn: _FunctionNode | ast.ClassDef) -> list[tuple[str, ast.AST]]:
    """Every site that binds a name in the scope of `fn` (parameters and
    the statement or expression that binds), not looking into nested
    scopes. Comprehension variables count as bindings of the enclosing
    scope, which is conservative."""
    found: list[tuple[str, ast.AST]] = []
    if not isinstance(fn, ast.ClassDef):
        args = fn.args
        for arg in [*args.posonlyargs, *args.args, *args.kwonlyargs]:
            found.append((arg.arg, arg))
        for arg in (args.vararg, args.kwarg):
            if arg is not None:
                found.append((arg.arg, arg))
    declared: set[str] = set()

    def visit(node: ast.AST, parent: ast.AST | None) -> None:
        match node:
            case ast.Global(names) | ast.Nonlocal(names):
                declared.update(names)
            case ast.Name(name, ast.Store() | ast.Del()):
                found.append((name, parent or node))
            case ast.FunctionDef() | ast.AsyncFunctionDef() | ast.ClassDef():
                found.append((node.name, node))
                for child in [*node.decorator_list, *getattr(node, "bases", [])]:
                    visit(child, node)
                if not isinstance(node, ast.ClassDef):
                    for default in [*node.args.defaults, *node.args.kw_defaults]:
                        if default is not None:
                            visit(default, node)
                return
            case ast.Lambda():
                for default in [*node.args.defaults, *node.args.kw_defaults]:
                    if default is not None:
                        visit(default, node)
                return
            case ast.Import(names) | ast.ImportFrom(names=names):
                for alias in names:
                    found.append(((alias.asname or alias.name).split(".")[0], node))
            case ast.ExceptHandler(name=str(name)):
                found.append((name, node))
            case ast.MatchAs(name=str(name)) | ast.MatchStar(name=str(name)):
                found.append((name, node))
            case ast.MatchMapping(rest=str(name)):
                found.append((name, node))
        for child in ast.iter_child_nodes(node):
            visit(child, node if isinstance(node, ast.stmt | ast.NamedExpr) else parent)

    body = fn.body if isinstance(fn.body, list) else [fn.body]
    for stmt in body:
        visit(stmt, None)
    return [(n, site) for n, site in found if n not in declared]


def _declared(fn: _FunctionNode | ast.ClassDef, kind: type) -> set[str]:
    """Names declared `global` or `nonlocal` (`kind`) directly in `fn`."""
    names: set[str] = set()
    body = fn.body if isinstance(fn.body, list) else [fn.body]
    pending: list[ast.AST] = list(body)
    while pending:
        node = pending.pop()
        if isinstance(node, kind):
            names.update(node.names)  # type: ignore[attr-defined]
        if isinstance(node, _FunctionNode | ast.ClassDef):
            continue
        pending += ast.iter_child_nodes(node)
    return names


def _rebound_by_nested(fn: _FunctionNode | ast.ClassDef) -> set[str]:
    """Names that functions nested in `fn` declare `nonlocal`."""
    names: set[str] = set()
    body = fn.body if isinstance(fn.body, list) else [fn.body]
    for stmt in body:
        for node in ast.walk(stmt):
            if isinstance(node, ast.Nonlocal):
                names.update(node.names)
    return names


class _Walker:
    def __init__(self, indexer: _Indexer, module: _Module) -> None:
        self.indexer = indexer
        self.module = module

    def run(self) -> None:
        scope = _Scope("<module>", {}, set(), False, None)
        for stmt in self.module.tree.body:
            self.visit(stmt, scope)

    # --- scopes

    def visit_function(
        self,
        fn: _FunctionNode,
        outer: _Scope,
        label: str,
        method_of: ClassInfo | None,
        mangle: str | None,
    ) -> None:
        bindings = _bindings(fn)
        local = {name for name, _ in bindings}
        sites: dict[str, list[ast.AST]] = {}
        for name, site in bindings:
            sites.setdefault(name, []).append(site)
        unreliable = _rebound_by_nested(fn) | _declared(fn, ast.Global)
        env = {}
        hidden: set[str] = set()
        if outer.is_function:
            env = {k: v for k, v in outer.env.items() if k not in local}
            hidden = outer.hidden | outer.local
        for name in unreliable:
            env.pop(name, None)
        args = fn.args
        positional = [*args.posonlyargs, *args.args]
        params = {a.arg: a for a in [*positional, *args.kwonlyargs]}
        first = positional[0].arg if positional else None
        for name, found in sites.items():
            if name in unreliable:
                continue
            receiver = None
            if len(found) == 1 and name in params:
                if method_of is not None and name == first and not isinstance(fn, ast.Lambda):
                    receiver = self._first_parameter(fn, method_of, name)
                annotation = params[name].annotation
                if receiver is None and annotation is not None:
                    cls = self._annotation_class(annotation)
                    if cls is not None:
                        receiver = Receiver(
                            cls, False, "annotation", f"{name}: {ast.unparse(annotation)}"
                        )
            elif name not in params:
                receiver = self._local_class(name, found)
            if receiver is not None:
                env[name] = receiver
        scope = _Scope(label, env, local, True, mangle, hidden - local)
        body = fn.body if isinstance(fn.body, list) else [fn.body]
        for stmt in body:
            self.visit(stmt, scope)

    def _first_parameter(
        self, fn: ast.FunctionDef | ast.AsyncFunctionDef, cls: ClassInfo, name: str
    ) -> Receiver | None:
        decorators = {ast.unparse(d) for d in fn.decorator_list}
        if "staticmethod" in decorators:
            return None
        if "classmethod" in decorators or fn.name in _IMPLICIT_CLASSMETHODS:
            return Receiver(cls, True, "cls", "")
        return Receiver(cls, False, "self", "")

    def _annotation_class(self, annotation: ast.expr) -> ClassInfo | None:
        match annotation:
            case ast.Constant(str() as text):
                try:
                    parsed = ast.parse(text, mode="eval").body
                except SyntaxError:
                    return None
                return self._annotation_class(parsed)
            case ast.BinOp(left, ast.BitOr(), right):
                if isinstance(right, ast.Constant) and right.value is None:
                    return self._annotation_class(left)
                if isinstance(left, ast.Constant) and left.value is None:
                    return self._annotation_class(right)
                return None
            case ast.Subscript(func, inner) if ast.unparse(func) in (
                "Optional",
                "typing.Optional",
            ):
                return self._annotation_class(inner)
        return self.indexer.class_expr(self.module, annotation)

    def _local_class(self, name: str, sites: list[ast.AST]) -> Receiver | None:
        """The class of a local all of whose bindings construct it or are
        annotated with it."""
        result: Receiver | None = None
        for site in sites:
            receiver = None
            match site:
                case ast.Assign([ast.Name(target)], ast.Call(func)) if target == name:
                    cls = self.indexer.class_expr(self.module, func)
                    if cls is not None:
                        receiver = Receiver(
                            cls, False, "constructed", f"{name} = {ast.unparse(func)}(..)"
                        )
                case ast.AnnAssign(ast.Name(target), annotation) if target == name:
                    cls = self._annotation_class(annotation)
                    if cls is not None:
                        receiver = Receiver(
                            cls, False, "annotation", f"{name}: {ast.unparse(annotation)}"
                        )
            if receiver is None or (result is not None and result.cls is not receiver.cls):
                return None
            result = result or receiver
        return result

    # --- nodes

    def visit(self, node: ast.AST, scope: _Scope) -> None:
        match node:
            case ast.FunctionDef() | ast.AsyncFunctionDef():
                self.visit_header(node, scope)
                cls = self._class_of_body(node, scope)
                prefix = "" if scope.label == "<module>" else scope.label + "."
                if scope.is_function:
                    prefix = scope.label + ".<locals>."
                outer = self._outer_of(scope)
                self.visit_function(node, outer, prefix + node.name, cls, scope.mangle)
                return
            case ast.Lambda():
                for default in [*node.args.defaults, *node.args.kw_defaults]:
                    if default is not None:
                        self.visit(default, scope)
                self.visit_function(node, self._outer_of(scope), scope.label, None, scope.mangle)
                return
            case ast.ClassDef():
                self.visit_header(node, scope)
                if scope.is_function:
                    label = f"{scope.label}.<locals>.{node.name}"
                elif scope.label == "<module>":
                    label = node.name
                else:
                    label = f"{scope.label}.{node.name}"
                local = {name for name, _ in _bindings(node)}
                env = {k: v for k, v in scope.env.items() if k not in local}
                hidden = (scope.hidden | scope.local) if scope.is_function else set()
                body_scope = _Scope(label, env, local, False, node.name, hidden, scope, node)
                for stmt in node.body:
                    self.visit(stmt, body_scope)
                return
            case ast.Assign(targets, value):
                # The value is evaluated first.
                self.visit(value, scope)
                for target in targets:
                    self.visit(target, scope)
                return
            case ast.AnnAssign(target, annotation, value):
                if value is not None:
                    self.visit(value, scope)
                self.visit(target, scope)
                self.visit(annotation, scope)
                return
            case ast.AugAssign(ast.Attribute() as target, _, value):
                self.visit(value, scope)
                self.visit(target.value, scope)
                self.record(target, AUGMENTED, scope)
                return
            case ast.Attribute(value, attr, ctx):
                self.visit(value, scope)
                if attr == "__dict__":
                    self.record_dynamic("__dict__", value, None, node, scope)
                    return
                kind = {ast.Load: READ, ast.Store: WRITE, ast.Del: DELETE}[type(ctx)]
                self.record(node, kind, scope)
                return
            case ast.Call():
                self.visit_call(node, scope)
                for child in ast.iter_child_nodes(node):
                    self.visit(child, scope)
                return
        for child in ast.iter_child_nodes(node):
            self.visit(child, scope)

    def visit_header(self, node: ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef, scope):
        for decorator in node.decorator_list:
            self.visit(decorator, scope)
        if isinstance(node, ast.ClassDef):
            for child in [*node.bases, *node.keywords]:
                self.visit(child, scope)
        else:
            for default in [*node.args.defaults, *node.args.kw_defaults]:
                if default is not None:
                    self.visit(default, scope)

    def _class_of_body(self, node: ast.AST, scope: _Scope) -> ClassInfo | None:
        if scope.class_node is None:
            return None
        for cls in self.indexer.classes:
            if cls.node is scope.class_node:
                return cls
        return None

    def _outer_of(self, scope: _Scope) -> _Scope:
        """The scope whose names functions defined in `scope` see: class
        bodies are skipped."""
        while scope.outer is not None:
            scope = scope.outer
        return scope

    def visit_call(self, node: ast.Call, scope: _Scope) -> None:
        func = node.func
        created = self.resolve(func, scope)
        if created is not None and created.is_class:
            access = self._access(None, "call", func, node, scope, created)
            created.cls.constructed.append(access)
        name = None
        if isinstance(func, ast.Name) and self.builtin(func.id, scope):
            name = func.id
        elif isinstance(func, ast.Attribute) and ast.unparse(func) in (
            "object.__setattr__",
            "object.__getattribute__",
            "object.__delattr__",
        ):
            name = ast.unparse(func)
        if name is None or not node.args:
            return
        target = node.args[0]
        if name == "vars" and len(node.args) == 1:
            self.record_dynamic("vars", target, None, node, scope)
        elif name in _DYNAMIC_KINDS or name.startswith("object."):
            if len(node.args) < 2:
                return
            key = node.args[1]
            literal = key.value if isinstance(key, ast.Constant) else None
            kind = name
            if name.startswith("object."):
                kind = {"__setattr__": "setattr", "__delattr__": "delattr"}.get(
                    name.rpartition(".")[2], "getattr"
                )
            if isinstance(literal, str):
                self._add(literal, kind, target, node, scope)
            else:
                self.record_dynamic(name, target, None, node, scope)

    def resolve(self, expr: ast.expr, scope: _Scope) -> Receiver | None:
        match expr:
            case ast.Name(name):
                if name in scope.env:
                    return scope.env[name]
                if name in scope.local or name in scope.hidden:
                    return None
                cls = self.indexer.global_class(self.module, name)
                if cls is not None:
                    return Receiver(cls, True, "class", "")
                return None
            case ast.Call(ast.Name("type"), [inner]) if self.builtin("type", scope):
                found = self.resolve(inner, scope)
                if found is not None and not found.is_class:
                    return Receiver(found.cls, True, "type", found.evidence)
                return None
            case ast.Attribute(inner, "__class__"):
                found = self.resolve(inner, scope)
                if found is not None and not found.is_class:
                    return Receiver(found.cls, True, "type", found.evidence)
                return None
            case ast.Attribute():
                if any(
                    isinstance(n, ast.Name) and (n.id in scope.local or n.id in scope.hidden)
                    for n in ast.walk(expr)
                ):
                    return None
                cls = self.indexer.class_expr(self.module, expr)
                if cls is not None:
                    return Receiver(cls, True, "class", "")
        return None

    def builtin(self, name: str, scope: _Scope) -> bool:
        """Whether `name` can only be the builtin of that name."""
        if name in scope.local or name in scope.hidden:
            return False
        return self.module.bindings.counts[name] == 0 and not self.module.bindings.dynamic

    def is_module(self, expr: ast.expr, scope: _Scope) -> bool:
        """Whether `expr` is a module object (or an attribute of a module
        outside the project, which is not of interest)."""
        return self.module_of(expr, scope) is not None

    def module_of(self, expr: ast.expr, scope: _Scope) -> str | None:
        """The project module `expr` evaluates to, `?` for other modules and
        their attributes."""
        if isinstance(expr, ast.Name) and expr.id not in scope.local | scope.hidden:
            return self.indexer.global_module(self.module, expr.id)
        if isinstance(expr, ast.Attribute):
            base = self.module_of(expr.value, scope)
            if base == "?":
                return base
            if base is not None and f"{base}.{expr.attr}" in self.indexer.modules:
                return f"{base}.{expr.attr}"
        return None

    def record(self, node: ast.Attribute, kind: str, scope: _Scope) -> None:
        if node.attr == "__class__":
            return
        attr = _mangle(node.attr, scope.mangle) if scope.mangle else node.attr
        self._add(attr, kind, node.value, node, scope)

    def _add(self, attr: str, kind: str, receiver: ast.expr, node: ast.AST, scope) -> None:
        if self.is_module(receiver, scope) or _is_super(receiver):
            return
        found = self.resolve(receiver, scope)
        self.indexer.raw.append(
            _Raw(attr, kind, found, self._access(attr, kind, receiver, node, scope, found))
        )

    def record_dynamic(self, kind: str, receiver: ast.expr, attr, node: ast.AST, scope) -> None:
        if self.is_module(receiver, scope):
            return
        found = self.resolve(receiver, scope)
        access = self._access(attr, kind, receiver, node, scope, found)
        self.indexer.raw.append(_Raw(None, kind, found, access))

    def _access(self, attr, kind, receiver, node, scope, found: Receiver | None) -> Access:
        text = ast.unparse(receiver)
        if len(text) > 40:
            text = text[:37] + "..."
        return Access(
            attr,
            kind,
            self.module.name,
            self.module.file,
            node.lineno,  # type: ignore[attr-defined]
            scope.label,
            text,
            found.via if found else "unknown",
            found.evidence if found else "",
        )


# --- reports -----------------------------------------------------------------------


def select(index: AttributeIndex, names: list[str]) -> list[ClassInfo]:
    """The classes named by `names` (see `AttributeIndex.find`); raises
    `LookupError` for a name that matches no class or several."""
    selected: list[ClassInfo] = []
    for name in names:
        found = index.find(name)
        if not found:
            raise LookupError(f"no class {name!r} in the project")
        if len(found) > 1:
            listed = ", ".join(c.full_name for c in found)
            raise LookupError(f"{name!r} is ambiguous: {listed}")
        if found[0] not in selected:
            selected.append(found[0])
    return selected


def outside_users(cls: ClassInfo) -> tuple[set[str], set[str]]:
    """The functions (`module:function`) outside the class's own methods
    that access its attributes through a known receiver, and their modules."""
    functions: set[str] = set()
    for member in cls.members.values():
        for access in member.accesses:
            if not cls.own_function(access.function) or access.module != cls.module:
                functions.add(f"{access.module}:{access.function}")
    for access in cls.dynamic:
        if not cls.own_function(access.function) or access.module != cls.module:
            functions.add(f"{access.module}:{access.function}")
    return functions, {f.partition(":")[0] for f in functions}


def _plural(count: int, word: str, plural: str | None = None) -> str:
    return f"{count} {word}" if count == 1 else f"{count} {plural or word + 's'}"


def _access_lines(accesses: list[Access], indent: str, attribute: bool = True) -> list[str]:
    rows = []
    for a in accesses:
        if attribute:
            target = f"{a.receiver}.{a.attribute}"
            if a.kind in _DYNAMIC_KINDS:
                target = f"{a.kind}({a.receiver}, {a.attribute!r})"
        elif a.kind == "__dict__":
            target = f"{a.receiver}.__dict__"
        elif a.kind == "vars":
            target = f"vars({a.receiver})"
        else:
            target = f"{a.kind}({a.receiver}, ..)"
        note = f"  [inferred: {a.evidence}]" if a.inferred else ""
        kind = a.effect if attribute else a.kind
        rows.append((kind, a.location, a.function, target + note))
    if not rows:
        return []
    widths = [max(len(r[i]) for r in rows) for i in range(3)]
    return [indent + "  ".join(r[i].ljust(widths[i]) for i in range(3)) + "  " + r[3] for r in rows]


def format_text(index: AttributeIndex, classes: list[ClassInfo] | None = None) -> str:
    """The index as text: each class with its attributes and accesses, then
    the accesses through receivers of unknown class."""
    shown = classes if classes is not None else index.classes
    lines: list[str] = []
    for cls in shown:
        lines += _class_lines(cls)
        lines.append("")
    attributes = {m.name for c in shown for m in c.instance_attributes()}
    methods = {name for c in shown for name in c.methods}
    lines.append("instance attributes of project classes used through receivers of unknown class")
    lines += _unknown_lines(
        [a for a in index.unknown if a.attribute in attributes], index.candidates
    )
    lines.append("")
    lines.append("methods of project classes used through receivers of unknown class")
    lines += _unknown_lines(
        [a for a in index.unknown_methods if a.attribute in methods], index.method_candidates
    )
    lines.append("")
    lines.append("dynamic access through receivers of unknown class")
    if not index.dynamic:
        lines.append("  none")
    lines += _access_lines(index.dynamic, "  ", attribute=False)
    return "\n".join(lines) + "\n"


def _unknown_lines(accesses: list[Access], candidates) -> list[str]:
    if not accesses:
        return ["  none"]
    lines = []
    by_name: dict[str, list[Access]] = {}
    for access in accesses:
        assert access.attribute is not None
        by_name.setdefault(access.attribute, []).append(access)
    for name, found in sorted(by_name.items()):
        owners = ", ".join(c.full_name for c in candidates(name))
        functions = {f"{a.module}:{a.function}" for a in found}
        modules = sorted({a.module for a in found})
        lines.append(
            f"  {name}  ({owners}; {_plural(len(found), 'access', 'accesses')} in "
            f"{_plural(len(functions), 'function')}: {', '.join(modules)})"
        )
        lines += _access_lines(found, "    ")
    return lines


def _class_lines(cls: ClassInfo) -> list[str]:
    lines = [f"class {cls.full_name}  ({cls.file}:{cls.line}, {cls.location})"]
    if cls.keywords or cls.base_sources not in ([], ["object"]):
        lines.append(f"  bases: {', '.join([*cls.base_sources, *cls.keywords])}")
    if cls.decorators:
        lines.append(f"  decorators: {', '.join('@' + d for d in cls.decorators)}")
    subclasses = cls.all_subclasses()
    if subclasses:
        listed = ", ".join(f"{c.full_name} ({c.file}:{c.line})" for c in subclasses)
        lines.append(f"  subclasses: {listed}")
    functions, modules = outside_users(cls)
    if functions:
        lines.append(
            f"  used outside its methods by {_plural(len(functions), 'function')} in "
            f"{_plural(len(modules), 'module')}: {', '.join(sorted(modules))}"
        )
    if cls.constructed:
        listed = ", ".join(f"{a.function} ({a.location})" for a in cls.constructed)
        lines.append(f"  constructed in: {listed}")
    if cls.hooks:
        lines.append(f"  attribute hooks: {', '.join(cls.hooks)}")
    if cls.slots is not None:
        lines.append(f"  __slots__: {', '.join(cls.slots) or '()'}")
    instance = cls.instance_attributes()
    lines.append("  instance attributes" + ("" if instance else ": none"))
    for member in instance:
        notes = []
        if not member.static:
            notes.append("not in __static_attributes__")
        if member.class_line is not None:
            notes.append(f"class default at line {member.class_line}")
        if not any(a.effect != READ for a in member.accesses) and not member.static:
            notes.append("never assigned in the project")
        lines.append(f"    {member.name}" + (f"  ({'; '.join(notes)})" if notes else ""))
        lines += _access_lines(member.accesses, "      ")
    class_members = [m for m in cls.class_members() if m.accesses]
    if class_members:
        lines.append("  class attributes")
        for member in class_members:
            where = (
                f"class body line {member.class_line}"
                if member.class_line
                else "not in the class body"
            )
            lines.append(f"    {member.name}  ({where})")
            lines += _access_lines(member.accesses, "      ")
    if cls.method_uses:
        lines.append("  methods used through instances or the class")
        for name, accesses in cls.method_uses.items():
            lines.append(f"    {name}")
            lines += _access_lines(accesses, "      ")
    if cls.dynamic:
        lines.append("  dynamic access")
        lines += _access_lines(cls.dynamic, "    ", attribute=False)
    return lines


def to_json(index: AttributeIndex, classes: list[ClassInfo] | None = None) -> dict:
    shown = classes if classes is not None else index.classes
    names = {m.name for c in shown for m in c.instance_attributes()}
    methods = {name for c in shown for name in c.methods}
    return {
        "classes": [_class_json(c) for c in shown],
        "unknown": [a.to_json() for a in index.unknown if a.attribute in names],
        "unknown_methods": [a.to_json() for a in index.unknown_methods if a.attribute in methods],
        "dynamic": [a.to_json() for a in index.dynamic],
    }


def _class_json(cls: ClassInfo) -> dict:
    functions, modules = outside_users(cls)
    return {
        "name": cls.full_name,
        "module": cls.module,
        "qualname": cls.qualname,
        "file": cls.file,
        "line": cls.line,
        "location": cls.location,
        "bases": cls.base_sources,
        "keywords": cls.keywords,
        "decorators": cls.decorators,
        "subclasses": [c.full_name for c in cls.all_subclasses()],
        "static_attributes": list(cls.static_attributes),
        "slots": None if cls.slots is None else list(cls.slots),
        "hooks": cls.hooks,
        "constructed": [a.to_json() for a in cls.constructed],
        "outside_functions": sorted(functions),
        "outside_modules": sorted(modules),
        "attributes": [
            {
                "name": m.name,
                "instance": m.instance,
                "static": m.static,
                "class_line": m.class_line,
                "accesses": [a.to_json() for a in m.accesses],
            }
            for m in cls.members.values()
        ],
        "methods": {
            name: [a.to_json() for a in accesses] for name, accesses in cls.method_uses.items()
        },
        "dynamic": [a.to_json() for a in cls.dynamic],
    }
