"""Find the modules of a project: the entry script (or module) and the
project's own modules it imports (design §7).

The project directory is the entry script's directory, which is
`sys.path[0]` for `python app.py` (the current directory for `python -m
pkg`). A module belongs to the project if an import found statically
resolves to a source file under it, the way CPython's path finder would
find it there: `import a.b`, `from a import b` (`b` may be a submodule),
relative imports, and `importlib.import_module("a.b")` / `__import__("a")`
with a literal name. Imports inside functions and `try` count too: the
modules are only executed when they are actually imported at run time.
More modules can be included by name or path (for imports with computed
names), and modules can be excluded.

Everything else (the standard library, third-party packages, namespace
packages, project modules that are left out) imports from its Python
source at run time, as before.
"""

from __future__ import annotations

import _imp
import ast
import importlib.machinery
import os
from collections.abc import Iterable, Iterator
from dataclasses import dataclass, field
from pathlib import Path

from steelix.frontend.parse import ParsedModule, parse_source, read_source


@dataclass(frozen=True)
class SourceModule:
    """A Python module of the project."""

    name: str
    """The full module name, `pkg.sub`."""
    relpath: str
    """The source file relative to the project directory, with `/`:
    `pkg/sub.py`, `pkg/__init__.py` for a package."""
    package: bool

    @property
    def package_name(self) -> str:
        """The package relative imports start from (`__package__`)."""
        return self.name if self.package else self.name.rpartition(".")[0]


@dataclass
class Project:
    root: Path
    """The project directory, `sys.path[0]`, with symlinks resolved."""
    main: SourceModule
    """The module that runs as `__main__`. For a script its name is the file
    name without `.py`, which is what `import` would call it."""
    script: Path | None
    """The entry script as it was given, or `None` for `-m module`."""
    modules: dict[str, SourceModule]
    """The modules that `import` finds as transpiled code, by name, sorted.
    The entry script is among them only if something imports it by name."""
    parsed: dict[str, ParsedModule] = field(default_factory=dict)
    """The parsed source of the main module and of each module, by `relpath`."""
    notes: list[str] = field(default_factory=list)
    """What was left out and why, for the report."""
    left_out: set[str] = field(default_factory=set)
    """Project modules found by the analysis that import from their Python
    source at run time (excluded, namespace packages, not compiling): their
    file, or their directory for a package, relative to the project."""

    @property
    def run_module(self) -> bool:
        """Whether the program runs like `python -m main` rather than as a script."""
        return self.script is None

    def sources(self) -> list[SourceModule]:
        """The main module and the modules, each source file once."""
        seen = {self.main.relpath: self.main}
        for module in self.modules.values():
            seen.setdefault(module.relpath, module)
        return list(seen.values())

    def path(self, module: SourceModule) -> Path:
        return self.root / module.relpath


class ProjectError(Exception):
    """A request that can not be satisfied, such as an unknown module to include."""


def find_project(
    script: Path | None = None,
    *,
    module: str | None = None,
    root: Path | None = None,
    include: Iterable[str] = (),
    exclude: Iterable[str] = (),
    run_main: bool = True,
) -> Project:
    """The project of the entry `script`, or of `python -m module` run in
    `root` (default: the current directory). Without `run_main` (extension
    modules), a package given as `module` is the main module itself rather
    than its `__main__`, and all of the package's modules are included."""
    excluded = list(exclude)
    if script is not None:
        directory = Path(os.path.dirname(os.path.realpath(script)))
        # A symlinked script runs from the directory of its target.
        main = SourceModule(script.stem, os.path.basename(os.path.realpath(script)), False)
        finder = _Finder(directory, excluded)
    else:
        assert module is not None
        directory = Path(os.path.realpath(root or Path.cwd()))
        finder = _Finder(directory, excluded)
        found = finder.locate(module)
        if found is None:
            raise ProjectError(f"no module named {module!r} in {directory}")
        if found.package and run_main:
            main_name = f"{module}.__main__"
            found = finder.locate(main_name)
            if found is None:
                raise ProjectError(f"{module!r} is a package and has no __main__.py")
        main = found
        if finder.is_excluded(main.name):
            raise ProjectError(f"can not exclude the main module {main.name!r}")
    project = Project(directory, main, script, {})
    finder.project = project

    main_parsed = _parse(project, main, required=True)
    assert main_parsed is not None
    if script is not None:
        main_parsed.path = script  # `sys.argv[0]` keeps the path as given
    project.parsed[main.relpath] = main_parsed
    queue: list[tuple[SourceModule, ParsedModule]] = [(main, main_parsed)]
    if project.run_module:
        finder.add_with_parents(main.name, queue)
        if not run_main and main.package:
            # A library package: all of its modules can be imported.
            include = [str(project.path(main).parent), *include]
    for item in include:
        for name in finder.included(item):
            finder.add_with_parents(name, queue)
    while queue:
        source, parsed = queue.pop()
        for name in imported_names(parsed.tree, source):
            finder.add_with_parents(name, queue)
    project.modules = dict(sorted(project.modules.items()))
    return project


def _parse(project: Project, module: SourceModule, *, required: bool) -> ParsedModule | None:
    """The parsed source of a module. A module with a syntax error is left
    out (unless `required`): importing it fails at run time like it does
    with CPython."""
    path = project.path(module)
    try:
        parsed = parse_source(read_source(path), module.name, module.relpath)
    except (SyntaxError, UnicodeDecodeError, ValueError) as err:
        if required:
            raise
        project.notes.append(f"{module.relpath}: not transpiled, it does not compile: {err}")
        project.left_out.add(module.relpath)
        return None
    parsed.path = path
    return parsed


class _Finder:
    """Locates modules under the project directory like CPython's path
    finder would, and collects them into the project."""

    def __init__(self, root: Path, exclude: list[str]) -> None:
        self.root = root
        self.exclude = exclude
        self.project: Project
        self.failed: set[str] = set()
        self.noted: set[str] = set()

    def is_excluded(self, name: str) -> bool:
        return any(name == e or name.startswith(e + ".") for e in self.exclude)

    def locate(self, name: str) -> SourceModule | None:
        """The project module `name`, if the path finder would find it in the
        project directory (and builtin and frozen modules do not take
        precedence)."""
        parts = name.split(".")
        if not all(parts) or _imp.is_builtin(parts[0]) or _imp.is_frozen(parts[0]):
            return None
        directory = self.root
        for i, part in enumerate(parts):
            last = i == len(parts) - 1
            if any((directory / part / f"__init__{s}").is_file() for s in _EXTENSIONS):
                return None
            init = directory / part / "__init__.py"
            if init.is_file():
                if last:
                    return SourceModule(name, self.relpath(init), True)
                directory = directory / part
                continue
            if any((directory / f"{part}{s}").is_file() for s in _EXTENSIONS):
                return None
            if last and (directory / f"{part}.py").is_file():
                return SourceModule(name, self.relpath(directory / f"{part}.py"), False)
            if (directory / part).is_dir():
                self.project.left_out.add(self.relpath(directory / part))
                self.note(
                    ".".join(parts[: i + 1]),
                    f"{self.relpath(directory / part)}/: namespace package, not transpiled",
                )
            return None
        return None

    def relpath(self, path: Path) -> str:
        return path.relative_to(self.root).as_posix()

    def note(self, key: str, message: str) -> None:
        if key not in self.noted:
            self.noted.add(key)
            self.project.notes.append(message)

    def add_with_parents(self, name: str, queue: list[tuple[SourceModule, ParsedModule]]) -> None:
        """Adds the project module `name` and its parent packages, if they
        are project modules, and queues them for scanning."""
        parts = name.split(".")
        for i in range(1, len(parts) + 1):
            prefix = ".".join(parts[:i])
            if prefix in self.project.modules:
                continue
            if prefix in self.failed:
                return
            found = self.locate(prefix)
            if found is None:
                return
            if self.is_excluded(prefix):
                self.note(prefix, f"{found.relpath}: excluded, imports from its Python source")
                excluded = found.relpath.removesuffix("/__init__.py") if found.package else None
                self.project.left_out.add(excluded or found.relpath)
                return
            parsed = self.project.parsed.get(found.relpath)
            if parsed is None:
                parsed = _parse(self.project, found, required=False)
            if parsed is None:
                self.failed.add(prefix)
                return
            self.project.parsed[found.relpath] = parsed
            self.project.modules[prefix] = found
            queue.append((found, parsed))

    def included(self, item: str) -> Iterator[str]:
        """The module names an `--include` argument stands for: a module
        name, or a `.py` file or a directory (all modules in it)."""
        path = Path(item)
        if item.endswith(".py") or os.sep in item or path.is_dir():
            if not path.exists():
                raise ProjectError(f"--include {item}: no such file or directory")
            files = sorted(path.rglob("*.py")) if path.is_dir() else [path]
            for file in files:
                name = self.module_name(Path(os.path.realpath(file)))
                if name is None:
                    if not path.is_dir():
                        raise ProjectError(
                            f"--include {item}: not an importable module of {self.root}"
                        )
                    continue
                yield name
        else:
            if self.locate(item) is None:
                raise ProjectError(f"--include {item}: no such module in {self.root}")
            yield item

    def module_name(self, file: Path) -> str | None:
        """The name `file` is imported as from the project directory, if any:
        every directory on the way must be a package."""
        try:
            relative = file.relative_to(self.root)
        except ValueError:
            return None
        parts = list(relative.parts)
        if "__pycache__" in parts or any(p.startswith(".") for p in parts):
            return None
        directory = self.root
        for part in parts[:-1]:
            directory = directory / part
            if not (directory / "__init__.py").is_file():
                return None
        stem = parts[-1].removesuffix(".py")
        names = parts[:-1] if stem == "__init__" else [*parts[:-1], stem]
        if not names:
            return None
        name = ".".join(names)
        found = self.locate(name)
        return name if found is not None and found.relpath == relative.as_posix() else None


_EXTENSIONS = tuple(importlib.machinery.EXTENSION_SUFFIXES)


def imported_names(tree: ast.Module, module: SourceModule) -> Iterator[str]:
    """The absolute module names `tree` may import, found statically: the
    packages and modules of `import` statements, the possible submodules of
    `from` imports, and literal names given to `importlib.import_module` or
    `__import__`. Names that are not modules are filtered out later."""
    # A top-level module (the script) has no package: its relative imports fail.
    package = module.package_name
    for node in ast.walk(tree):
        match node:
            case ast.Import(names):
                for alias in names:
                    yield alias.name
            case ast.ImportFrom(name, names, level):
                base = resolve_relative(name, level, package) if level else name
                if base is None:
                    continue
                yield base
                for alias in names:
                    if alias.name != "*":
                        yield f"{base}.{alias.name}"
            case ast.Call(func, [ast.Constant(str() as name), *rest]):
                if _is_import_module(func):
                    if name.startswith("."):
                        match rest:
                            case [ast.Constant(str() as anchor), *_]:
                                dots = len(name) - len(name.lstrip("."))
                                resolved = resolve_relative(name[dots:] or None, dots, anchor)
                                if resolved is not None:
                                    yield resolved
                    else:
                        yield name
                elif isinstance(func, ast.Name) and func.id == "__import__":
                    yield name


def _is_import_module(func: ast.expr) -> bool:
    return (
        isinstance(func, ast.Attribute)
        and func.attr == "import_module"
        or (isinstance(func, ast.Name) and func.id == "import_module")
    )


def resolve_relative(name: str | None, level: int, package: str | None) -> str | None:
    """The absolute name of `from {'.' * level}{name} import ..` in `package`,
    or `None` where the import fails (no package, beyond the top level)."""
    if not package:
        return None
    bits = package.rsplit(".", level - 1)
    if len(bits) < level:
        return None
    base = bits[0]
    return f"{base}.{name}" if name else base


def cross_module_patches(project: Project) -> dict[str, set[str] | None]:
    """For each project module, the functions other modules may replace, or
    `None` if they may bind any of its globals (design §7).

    A direct Rust call of a function would not see the replacement, so
    these functions are called through globals. Like the analysis of a
    module patching itself, this is conservative and only knows the usual
    spellings: assigning or deleting an attribute of a module object
    (`helpers.f = ..`, `setattr(helpers, "f", ..)`, `del helpers.f`),
    `vars(helpers)` and `helpers.__dict__`, a call with a module object and
    the name of one of its functions (`mock.patch.object(helpers, "f")`),
    and a string naming a function (`mock.patch("pkg.helpers.f")`). The
    module objects are names bound by importing a project module, and
    attributes of them that are project modules.
    """
    names = {name: name for name in project.modules}
    names["__main__"] = project.main.name
    defs = {
        name: _def_names(project.parsed[source.relpath].tree)
        for name, source in [*project.modules.items(), (project.main.name, project.main)]
    }
    patches: dict[str, set[str] | None] = {}

    def mark(module: str, attr: str | None) -> None:
        if attr is None:
            patches[module] = None
        elif attr in defs[module]:
            found = patches.setdefault(module, set())
            if found is not None:
                found.add(attr)

    for source in project.sources():
        tree = project.parsed[source.relpath].tree
        aliases = _module_aliases(tree, source, names)

        for node in ast.walk(tree):
            match node:
                case ast.Attribute(obj, attr, ast.Store() | ast.Del()):
                    if (module := _module_object(obj, aliases, names)) is not None:
                        mark(module, attr)
                case ast.Attribute(obj, "__dict__"):
                    if (module := _module_object(obj, aliases, names)) is not None:
                        mark(module, None)
                case ast.Call(ast.Name("vars"), [obj]):
                    if (module := _module_object(obj, aliases, names)) is not None:
                        mark(module, None)
                case ast.Call(ast.Name("setattr" | "delattr"), [obj, key, *_]):
                    if (module := _module_object(obj, aliases, names)) is not None:
                        match key:
                            case ast.Constant(str() as attr):
                                mark(module, attr)
                            case _:
                                mark(module, None)
                case ast.Call(func, [obj, ast.Constant(str() as attr), *_]):
                    if not (isinstance(func, ast.Name) and func.id in {"getattr", "hasattr"}):
                        if (module := _module_object(obj, aliases, names)) is not None:
                            mark(module, attr)
                case ast.Constant(str() as text) if "." in text:
                    module, _, attr = text.rpartition(".")
                    if module in names:
                        mark(names[module], attr)
    return patches


def _module_object(expr: ast.expr, aliases: dict[str, str], names: dict[str, str]) -> str | None:
    """The project module `expr` evaluates to, if it is a usual spelling of one."""
    match expr:
        case ast.Name(name):
            return aliases.get(name)
        case ast.Attribute(value, attr):
            base = _module_object(value, aliases, names)
            return names.get(f"{base}.{attr}") if base else None
        case ast.Call(func, [ast.Constant(str() as name), *_]) if _is_import_module(func):
            return names.get(name)
        case ast.Subscript(ast.Attribute(_, "modules"), ast.Constant(str() as name)):
            return names.get(name)
    return None


def _def_names(tree: ast.Module) -> set[str]:
    return {
        stmt.name for stmt in tree.body if isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef)
    }


def _module_aliases(
    tree: ast.Module, source: SourceModule, names: dict[str, str]
) -> dict[str, str]:
    """The names (in any scope) bound to project module objects by imports,
    or by assigning another such name."""
    aliases: dict[str, str] = {}
    for node in ast.walk(tree):
        match node:
            case ast.Import(imported):
                for alias in imported:
                    if alias.asname is not None:
                        if alias.name in names:
                            aliases[alias.asname] = names[alias.name]
                    else:
                        top = alias.name.split(".")[0]
                        if top in names:
                            aliases[top] = names[top]
            case ast.ImportFrom(module, imported, level):
                base = resolve_relative(module, level, source.package_name) if level else module
                for alias in imported:
                    full = f"{base}.{alias.name}"
                    if base is not None and full in names:
                        aliases[alias.asname or alias.name] = names[full]
    while True:
        found = {
            target.id: aliases[value.id]
            for node in ast.walk(tree)
            if isinstance(node, ast.Assign) and isinstance(value := node.value, ast.Name)
            if value.id in aliases
            for target in node.targets
            if isinstance(target, ast.Name) and target.id not in aliases
        }
        if not found:
            return aliases
        aliases |= found
