"""A draft `#[pyclass]` for one class of the project, for a human to finish
(design §15 "类的改写", step 2).

The draft is never compiled or wired into the build, and the transpiler
never upgrades classes by itself (design §10 "不选的方案"). It lists the
instance attributes as fields typed `Py<PyAny>`, a `#[pymethods]` stub per
method with its Python signature and the generated fn it replaces, and,
first of all, the facts found by the attribute index (`attrs.py`) that block
or complicate the upgrade.
"""

from __future__ import annotations

import ast

from steelix.attrs import READ, Access, AttributeIndex, ClassInfo, _plural, outside_users
from steelix.emit.crate import rust_layout
from steelix.lower.classes import mod_name
from steelix.lower.context import NAMESPACE
from steelix.lower.names import FUNCTION_RESERVED, MODULE_RESERVED, Renamer, rust_ident
from steelix.rustast.fmt import RustfmtError, rustfmt

_CLASS_ITEM_RESERVED = frozenset({"body", NAMESPACE, "__class__"})
# The result types PyO3 requires of some special methods.
_RESULTS = {
    "__init__": "Self",
    "__hash__": "isize",
    "__len__": "usize",
    "__bool__": "bool",
    "__contains__": "bool",
    "__setattr__": "()",
    "__delattr__": "()",
    "__setitem__": "()",
    "__delitem__": "()",
    "__set__": "()",
    "__delete__": "()",
}


def draft(index: AttributeIndex, cls: ClassInfo, command: str) -> str:
    """The draft Rust source for `cls`. `command` is the steelix command
    line that wrote it, quoted in the header."""
    generated = _generated_names(index, cls)
    lines = [
        f"// DRAFT of a `#[pyclass]` for class `{cls.qualname}` ({cls.file}:{cls.line}),",
        f"// written by `{command}`.",
        "//",
        "// This is advice for a human, not generated code: it is not part of the build,",
        "// every body is `todo!()` and nothing checks it against the program. Design §15",
        "// (类的改写, step 2) describes how to finish it, design §10 why steelix does",
        "// not do it.",
        "//",
    ]
    if generated is not None:
        file, path, _ = generated
        lines.append(f"// The class is generated as `pub mod {path}` in src/{file}; the struct and")
        lines.append("// its `#[pymethods]` replace that mod, and in `exec_module` the")
        setter = f'm.set_global("{cls.name}", py.get_type::<{cls.name}>())'
        lines.append(f"// `rt::class!(..)` becomes `{setter}`.")
    else:
        lines.append("// The class is not generated as a Rust mod (it is defined in a function and")
        lines.append("// stays Python).")
    lines.append("//")
    facts = blocking_facts(index, cls)
    lines.append("// What blocks or complicates the upgrade:")
    if facts:
        for fact in facts:
            lines += _wrap(f"- {fact}", "// ", "//   ")
    else:
        lines.append("// - nothing that the analysis checks (see `steelix attrs` for its limits)")
    lines.append("//")
    lines += _wrap(
        "The analysis only sees the project's modules: code outside the project, tests "
        "and `eval` may also read, write or subclass the class.",
        "// ",
        "// ",
    )
    lines.append("")
    lines.append("use pyo3::prelude::*;")
    lines.append("use pyo3::types::{PyDict, PyTuple, PyType};")
    lines.append("")
    lines += _struct(cls)
    lines.append("")
    lines += _methods(cls, generated)
    source = "\n".join(lines) + "\n"
    try:
        return rustfmt(source)
    except RustfmtError:
        return source


def blocking_facts(index: AttributeIndex, cls: ClassInfo) -> list[str]:
    """What makes the upgrade of `cls` to a `#[pyclass]` hard, as sentences."""
    facts: list[str] = []
    if cls.location == "function":
        facts.append("defined in a function: a `#[pyclass]` is a single type, not one per call.")
    elif cls.location == "conditional":
        facts.append(
            "defined conditionally (in a module level `if`, `try`, ..): a `#[pyclass]` "
            "always exists."
        )
    elif cls.location == "nested":
        facts.append(
            f"nested class: `__qualname__` must stay `{cls.qualname}` (`#[pyclass(name = ..)]` "
            "only sets the name), and the outer class must expose it as an attribute."
        )
    for source, base in zip(cls.base_sources, cls.bases, strict=True):
        if source == "object":
            continue
        if base is not None:
            facts.append(
                f"base `{source}` is a project class ({base.file}:{base.line}): it must become "
                f"a `#[pyclass(subclass)]` first, and this one `#[pyclass(extends = "
                f"{base.name})]`."
            )
        else:
            facts.append(
                f"base `{source}` is not a project class: a `#[pyclass]` can only extend "
                "native types, not classes defined in Python (§10)."
            )
    for keyword in cls.keywords:
        if keyword.startswith("metaclass="):
            facts.append(f"metaclass (`{keyword}`): `#[pyclass]` has no metaclass.")
        else:
            facts.append(f"class keyword `{keyword}` (passed to `__init_subclass__`).")
    for decorator in cls.decorators:
        if cls.dataclass is not None and ast.unparse(cls.dataclass) == decorator:
            facts.append(
                f"`@{decorator}`: the generated `__init__`, `__repr__`, `__eq__` (and "
                "`__hash__`, ordering, `frozen`) have to be written by hand, stubs below."
            )
        else:
            facts.append(
                f"class decorator `@{decorator}`: it receives the type, check what it does."
            )
    subclasses = cls.all_subclasses()
    if subclasses:
        listed = ", ".join(f"{c.full_name} ({c.file}:{c.line})" for c in subclasses)
        facts.append(
            f"subclassed in the project by {listed}: needs `#[pyclass(subclass)]`; the "
            "subclasses' methods read and write the fields too."
        )
    for line, text in cls.body_logic:
        facts.append(f"class body logic at line {line}: `{text}`.")
    if cls.hooks:
        facts.append(
            f"defines {', '.join(f'`{h}`' for h in cls.hooks)}: attribute access is dynamic."
        )
    if cls.slots is not None:
        facts.append(
            f"`__slots__` ({', '.join(cls.slots) or 'empty'}): the fields replace them; "
            "check whether anything relies on the slot descriptors."
        )
    dynamic = [a for a in cls.dynamic if a.kind not in ("__dict__", "vars")]
    namespace = [a for a in cls.dynamic if a.kind in ("__dict__", "vars")]
    if dynamic:
        facts.append(
            "dynamic attribute access on its instances: "
            + ", ".join(f"`{a.kind}` in {a.function} at {a.location}" for a in dynamic)
            + "."
        )
    if namespace:
        facts.append(
            "uses the instance `__dict__`: "
            + ", ".join(f"`{_dynamic_text(a)}` in {a.function} at {a.location}" for a in namespace)
            + "; a `#[pyclass]` has no `__dict__` unless `#[pyclass(dict)]`."
        )
    if index.dynamic:
        functions = sorted({f"{a.module}:{a.function}" for a in index.dynamic})
        shown = ", ".join(functions[:5]) + (", .." if len(functions) > 5 else "")
        facts.append(
            f"{_plural(len(index.dynamic), 'dynamic access', 'dynamic accesses')} in the "
            f"project with a receiver of unknown class may concern it ({shown})."
        )
    for member in cls.class_members():
        writes = [a for a in member.accesses if a.effect != READ]
        if writes:
            where = ", ".join(f"in {a.function} at {a.location}" for a in writes)
            facts.append(
                f"class attribute `{member.name}` is reassigned ({where}): it can not be a "
                "`#[classattr]`, keep it in the type's `__dict__`."
            )
    shadowed = [m.name for m in cls.instance_attributes() if m.class_line is not None]
    if shadowed and cls.dataclass is None:
        facts.append(
            "instance attributes with a class level default: "
            + ", ".join(f"`{n}`" for n in shadowed)
            + " (reads before the first assignment see the class attribute)."
        )
    external = [m.name for m in cls.instance_attributes() if not m.static]
    if external:
        facts.append(
            "instance attributes assigned outside the class's methods: "
            + ", ".join(f"`{n}`" for n in external)
            + "."
        )
    functions, modules = outside_users(cls)
    if functions:
        facts.append(
            f"its attributes are used outside its methods by "
            f"{_plural(len(functions), 'function')} in {_plural(len(modules), 'module')} "
            f"({', '.join(sorted(modules))}): all of them have to be rewritten with it."
        )
    attributes = {m.name for m in cls.instance_attributes()}
    unknown = [a for a in index.unknown if a.attribute in attributes]
    if unknown:
        by_name: dict[str, list[Access]] = {}
        for access in unknown:
            assert access.attribute is not None
            by_name.setdefault(access.attribute, []).append(access)
        parts = [
            f"`{name}` {len(found)}x in {', '.join(sorted({a.module for a in found}))}"
            for name, found in sorted(by_name.items())
        ]
        facts.append(
            "names of its attributes are also used through receivers of unknown class: "
            + "; ".join(parts)
            + f". See `steelix attrs --class {cls.full_name}`."
        )
    methods = [a for a in index.unknown_methods if a.attribute in cls.methods]
    methods = [a for a in methods if not (a.attribute or "").startswith("__")]
    if methods:
        names = sorted({a.attribute or "" for a in methods})
        facts.append(
            "names of its methods are called through receivers of unknown class: "
            + ", ".join(f"`{n}`" for n in names)
            + " (inlining them in the callers changes what a subclass override does)."
        )
    return facts


def _dynamic_text(access: Access) -> str:
    if access.kind == "__dict__":
        return f"{access.receiver}.__dict__"
    return f"{access.kind}({access.receiver})"


def _wrap(text: str, first: str, rest: str, width: int = 96) -> list[str]:
    words = text.split(" ")
    lines: list[str] = []
    current = first
    for word in words:
        if len(current) + len(word) > width and current.strip() not in ("//", "// -"):
            lines.append(current.rstrip())
            current = rest
        current += word + " "
    lines.append(current.rstrip())
    return lines


def _generated_names(index: AttributeIndex, cls: ClassInfo) -> tuple[str, str, dict] | None:
    """Where the transpiler puts the class: the file under `src/`, the
    module path of its mod (relative to that file), and the Rust fn name
    of each method node. Mirrors `lower/classes.py`, ignoring the rare
    renames caused by functions kept as Python."""
    if cls.location == "function":
        return None
    project = index.project
    layout = rust_layout(project)
    module = layout[cls.file]
    tree = project.parsed[cls.file].tree
    own = module.path[-1].removeprefix("r#")
    taken: set[str] = {*(c.removeprefix("r#") for c in module.children), own}
    module_classes = [c for c in index.classes if c.file == cls.file]
    names: dict[int, str] = {}
    by_taken: dict[int, set[str]] = {}
    for other in module_classes:
        if other.location == "function":
            continue
        outer = next((o for o in module_classes if other in o.nested.values()), None)
        scope = by_taken.setdefault(id(outer), set()) if outer is not None else taken
        names[id(other)] = mod_name(other.name, scope)
    path = [names[id(cls)]]
    current = cls
    while True:
        outer = next((o for o in module_classes if current in o.nested.values()), None)
        if outer is None:
            break
        path.insert(0, names[id(outer)])
        current = outer
    defs = [s.name for s in tree.body if isinstance(s, ast.FunctionDef)]
    reserved = set(Renamer(defs, MODULE_RESERVED).mapping.values())
    reserved |= FUNCTION_RESERVED | MODULE_RESERVED | _CLASS_ITEM_RESERVED
    fns: dict[int, str] = {}
    for stmt in cls.node.body:
        if isinstance(stmt, ast.FunctionDef):
            rust = rust_ident(stmt.name)
            while rust in reserved:
                rust += "_"
            reserved.add(rust)
            fns[id(stmt)] = rust
    return module.file, "::".join(path), fns


def _struct(cls: ClassInfo) -> list[str]:
    options = [f'module = "{cls.module}"', f'name = "{cls.name}"']
    if cls.all_subclasses():
        options.append("subclass")
    for base in cls.bases:
        if base is not None:
            options.append(f"extends = {base.name}")
    lines = [f"#[pyclass({', '.join(options)})]", f"pub struct {cls.name} {{"]
    members = sorted(cls.instance_attributes(), key=lambda m: _field_order(cls, m))
    if not members:
        lines.append("    // No instance attributes found.")
    for member in members:
        writers = _functions(a for a in member.accesses if a.effect != READ)
        readers = _functions(a for a in member.accesses if a.effect == READ)
        doc = f"`self.{member.name}`"
        if writers:
            doc += f", assigned in {', '.join(writers)}"
        if readers:
            doc += f", read in {', '.join(readers)}"
        lines += _wrap(doc + ".", "    /// ", "    /// ")
        lines.append(
            "    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it)."
        )
        lines.append(f"    {_field_name(member.name)}: Py<PyAny>,")
    lines.append("}")
    return lines


def _field_order(cls: ClassInfo, member) -> tuple:
    """Dataclass fields in order, then the order of the first assignment in
    the class's own methods (usually `__init__`)."""
    if member.name in cls.fields:
        return (0, cls.fields.index(member.name), "")
    lines = [
        a.line
        for a in member.accesses
        if a.effect != READ and a.file == cls.file and cls.own_function(a.function)
    ]
    return (1, min(lines, default=1 << 30), member.name)


def _functions(accesses) -> list[str]:
    names: list[str] = []
    for access in accesses:
        if access.function not in names:
            names.append(access.function)
    if len(names) > 6:
        return [*names[:6], f"{len(names) - 6} more"]
    return names


def _field_name(name: str) -> str:
    ident = rust_ident(name)
    return ident if ident != "slf" else "self_"


def _methods(cls: ClassInfo, generated: tuple[str, str, dict] | None) -> list[str]:
    lines = ["#[pymethods]", f"impl {cls.name} {{"]
    for member in cls.class_members():
        if member.class_line is None or member.name in cls.methods:
            continue
        if member.name == "__slots__" or member.name in cls.nested:
            continue
        reassigned = any(a.effect != READ for a in member.accesses)
        value = _class_value(cls, member.name)
        lines.append(f"    /// `{member.name} = {value}` (line {member.class_line}).")
        if reassigned:
            lines.append("    // Reassigned at run time: keep it in the type's `__dict__` instead.")
        lines.append("    #[classattr]")
        lines.append(f"    fn {_field_name(member.name)}(py: Python<'_>) -> Py<PyAny> {{")
        lines.append("        todo!()")
        lines.append("    }")
        lines.append("")
    if cls.dataclass is not None and "__init__" not in cls.methods:
        lines += _dataclass_init(cls)
    for stmt in cls.node.body:
        if isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef):
            lines += _method(cls, stmt, generated)
            lines.append("")
    if cls.dataclass is not None:
        lines += _dataclass_methods(cls)
    lines += [
        "    // pickle and `copy`: a `#[pyclass]` has no `__dict__` and does not pickle by",
        "    // default. Add `__reduce__` returning the type and the arguments that rebuild",
        f"    // the instance (the type must stay reachable as `{cls.module}.{cls.qualname}`).",
        "    // fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {",
        "    //     todo!()",
        "    // }",
        "}",
    ]
    return lines


def _class_value(cls: ClassInfo, name: str) -> str:
    for stmt in cls.node.body:
        if isinstance(stmt, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id == name for t in stmt.targets
        ):
            text = ast.unparse(stmt.value)
        elif (
            isinstance(stmt, ast.AnnAssign)
            and isinstance(stmt.target, ast.Name)
            and stmt.target.id == name
            and stmt.value is not None
        ):
            text = ast.unparse(stmt.value)
        else:
            continue
        return text if len(text) <= 50 else text[:47] + "..."
    return ".."


def _method(
    cls: ClassInfo,
    fn: ast.FunctionDef | ast.AsyncFunctionDef,
    generated: tuple[str, str, dict] | None,
) -> list[str]:
    decorators = [ast.unparse(d) for d in fn.decorator_list]
    signature = _python_signature(fn)
    prefix = "async def" if isinstance(fn, ast.AsyncFunctionDef) else "def"
    decorated = "".join(f"@{d} " for d in decorators)
    doc = f"`{decorated}{prefix} {fn.name}({signature})` (line {fn.lineno})"
    if generated is not None and id(fn) in generated[2]:
        doc += f", generated as `{generated[1]}::{generated[2][id(fn)]}` in src/{generated[0]}"
    elif isinstance(fn, ast.AsyncFunctionDef):
        doc += ", stays Python until coroutines are lowered (§8)"
    lines = _wrap(doc + ".", "    /// ", "    /// ")
    attributes: list[str] = []
    receiver = "&self"
    rust_name = rust_ident(fn.name)
    static = "staticmethod" in decorators
    classmethod = "classmethod" in decorators or fn.name in (
        "__init_subclass__",
        "__class_getitem__",
    )
    for decorator in decorators:
        if decorator in ("staticmethod", "classmethod"):
            continue
        if decorator == "property":
            attributes.append("#[getter]")
        elif decorator.endswith(".setter"):
            attributes.append("#[setter]")
            rust_name = f"set_{fn.name}"
        elif decorator.endswith(".deleter"):
            lines.append("    // TODO: a property deleter has no `#[pymethods]` equivalent.")
        else:
            lines.append(f"    // TODO: decorator `@{decorator}` has no `#[pymethods]` equivalent.")
    params = _params(fn)
    if fn.name == "__init__":
        attributes.append("#[new]")
        lines.append(
            "    // `#[new]` is `__new__`: it builds the struct from the arguments instead of"
        )
        lines.append("    // assigning attributes of an existing instance.")
        lines += _callers(cls)
        receiver = ""
        rust_name = "new"
        params = params[1:]
    elif static:
        attributes.append("#[staticmethod]")
        receiver = ""
    elif classmethod or fn.name == "__new__":
        attributes.append("#[classmethod]")
        receiver = "cls: &Bound<'py, PyType>"
        params = params[1:]
    else:
        params = params[1:]
        end = fn.end_lineno or fn.lineno
        writes = any(
            a.via == "self"
            and a.effect != READ
            and a.file == cls.file
            and fn.lineno <= a.line <= end
            for m in cls.members.values()
            for a in m.accesses
        )
        if writes:
            receiver = "&mut self"
    if rust_name.startswith("r#"):
        attributes.append(f'#[pyo3(name = "{fn.name}")]')
    pyo3_signature = _pyo3_signature(params)
    if pyo3_signature is not None:
        attributes.append(f"#[pyo3(signature = ({pyo3_signature}))]")
    for name, _, default in params:
        if default is not None and default != "None":
            lines.append(f"    // TODO: `{name}` defaults to `{default}` in Python.")
    result = f"PyResult<{_RESULTS.get(fn.name, 'Py<PyAny>')}>"
    if any(d.endswith(".setter") for d in decorators):
        result = "PyResult<()>"
        if receiver == "&self":
            receiver = "&mut self"
    if fn.name == "__new__":
        lines.append("    // TODO: `__new__` and `__init__` together become one `#[new]`.")
    lines += [f"    {a}" for a in attributes]
    args = [receiver] if receiver else []
    args.append("py: Python<'py>")
    for name, kind, default in params:
        args.append(f"{_field_name(name)}: {_rust_type(kind, default)}")
    lines.append(f"    fn {rust_name}<'py>({', '.join(args)}) -> {result} {{")
    lines.append("        todo!()")
    lines.append("    }")
    return lines


def _callers(cls: ClassInfo) -> list[str]:
    if not cls.constructed:
        return []
    listed = ", ".join(f"{a.function} ({a.location})" for a in cls.constructed)
    return _wrap(f"Called in {listed}.", "    // ", "    // ")


def _python_signature(fn: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
    return ast.unparse(fn.args)


def _params(fn: ast.FunctionDef | ast.AsyncFunctionDef) -> list[tuple[str, str, str | None]]:
    """(name, kind, default source) of each parameter; kinds are `posonly`,
    `arg`, `varargs`, `kwonly`, `varkw`."""
    args = fn.args
    positional = [*args.posonlyargs, *args.args]
    defaults: list[ast.expr | None] = [None] * (len(positional) - len(args.defaults))
    defaults += args.defaults
    params: list[tuple[str, str, str | None]] = []
    for i, (arg, default) in enumerate(zip(positional, defaults, strict=True)):
        kind = "posonly" if i < len(args.posonlyargs) else "arg"
        params.append((arg.arg, kind, None if default is None else ast.unparse(default)))
    if args.vararg is not None:
        params.append((args.vararg.arg, "varargs", None))
    for arg, default in zip(args.kwonlyargs, args.kw_defaults, strict=True):
        params.append((arg.arg, "kwonly", None if default is None else ast.unparse(default)))
    if args.kwarg is not None:
        params.append((args.kwarg.arg, "varkw", None))
    return params


def _pyo3_signature(params: list[tuple[str, str, str | None]]) -> str | None:
    if all(kind == "arg" and default is None for _, kind, default in params):
        return None
    parts: list[str] = []
    star = False
    for i, (name, kind, default) in enumerate(params):
        if kind == "varargs":
            parts.append(f"*{name}")
            star = True
        elif kind == "varkw":
            parts.append(f"**{name}")
        else:
            if kind == "kwonly" and not star:
                parts.append("*")
                star = True
            parts.append(name if default is None else f"{name} = None")
        following = params[i + 1][1] if i + 1 < len(params) else None
        if kind == "posonly" and following != "posonly":
            parts.append("/")
    return ", ".join(parts)


def _rust_type(kind: str, default: str | None) -> str:
    if kind == "varargs":
        return "&Bound<'py, PyTuple>"
    if kind == "varkw":
        return "Option<&Bound<'py, PyDict>>"
    if default is not None:
        return "Option<&Bound<'py, PyAny>>"
    return "&Bound<'py, PyAny>"


def _dataclass_init(cls: ClassInfo) -> list[str]:
    params = []
    for name in cls.fields:
        default = None
        for stmt in cls.node.body:
            if (
                isinstance(stmt, ast.AnnAssign)
                and isinstance(stmt.target, ast.Name)
                and stmt.target.id == name
                and stmt.value is not None
            ):
                default = ast.unparse(stmt.value)
        params.append((name, "arg", default))
    lines = ["    /// The `__init__` that `@dataclass` generates, from the fields in order."]
    for name, _, default in params:
        if default is not None and default != "None":
            lines.append(f"    // TODO: `{name}` defaults to `{default}` in Python.")
    lines += _callers(cls)
    lines.append("    #[new]")
    signature = _pyo3_signature(params)
    if signature is not None:
        lines.append(f"    #[pyo3(signature = ({signature}))]")
    args = ["py: Python<'py>"] + [
        f"{_field_name(name)}: {_rust_type(kind, default)}" for name, kind, default in params
    ]
    lines.append(f"    fn new<'py>({', '.join(args)}) -> PyResult<Self> {{")
    lines.append("        todo!()")
    lines.append("    }")
    lines.append("")
    return lines


def _dataclass_methods(cls: ClassInfo) -> list[str]:
    decorator = cls.dataclass
    options: dict[str, bool] = {}
    if isinstance(decorator, ast.Call):
        for keyword in decorator.keywords:
            if keyword.arg and isinstance(keyword.value, ast.Constant):
                options[keyword.arg] = bool(keyword.value.value)
    fields = ", ".join(f"{name}=.." for name in cls.fields)
    lines: list[str] = []
    if options.get("repr", True) and "__repr__" not in cls.methods:
        lines += [
            f"    /// `@dataclass` repr: `{cls.qualname}({fields})`, each field with `repr()`.",
            "    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {",
            "        todo!()",
            "    }",
            "",
        ]
    if options.get("eq", True) and "__eq__" not in cls.methods:
        lines += [
            "    /// `@dataclass` eq: compares the fields as a tuple, `NotImplemented` for",
            "    /// other types. Without `frozen=True` (or `unsafe_hash`) instances are",
            "    /// unhashable (`__hash__ = None`).",
            "    fn __eq__<'py>(",
            "        &self,",
            "        py: Python<'py>,",
            "        other: &Bound<'py, PyAny>,",
            "    ) -> PyResult<Py<PyAny>> {",
            "        todo!()",
            "    }",
            "",
        ]
    if options.get("order"):
        lines.append(
            "    // TODO: `order=True`: `__lt__`, `__le__`, `__gt__`, `__ge__` on the field tuple."
        )
        lines.append("")
    if options.get("frozen"):
        lines.append(
            "    // TODO: `frozen=True`: use `#[pyclass(frozen)]` and a `__hash__` of the fields."
        )
        lines.append("")
    return lines
