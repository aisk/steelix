"""Write a complete Cargo crate for a transpiled module or project.

Two output modes share the generated module files, which only expose
`exec_module(module)`:

* `bin` (default): an executable that embeds libpython and runs the module as
  `__main__`, like `python app.py`.
* `ext`: a cdylib PyO3 extension module that replaces the Python module.

A project (design §7) gets one Rust module per Python module, nested like
the packages (`pkg/__init__.py` is `src/pkg/mod.rs`, `pkg/sub.py` is
`src/pkg/sub.rs`), and a table of them in `main.rs` / `lib.rs` for the
runtime's importer.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

from steelix.frontend.parse import ParsedModule
from steelix.lower.module import LoweredModule, ModuleRole, lower_module
from steelix.lower.names import Renamer, module_ident
from steelix.project import Project, SourceModule, cross_module_patches
from steelix.rustast.fmt import config_text, rustfmt
from steelix.rustast.printer import print_file, string_literal
from steelix.rustast.verify import verify

PYO3_VERSION = "0.30"
RUNTIME_VERSION = "0.1"

# Crate-wide lint configuration. Python naming does not follow Rust's, Python
# functions may ignore their parameters, and Python code may assign values it
# never reads (including `del x`, which becomes `x = None`). The clippy lints
# are about the Python program itself, which the translation keeps as it is:
# a loop that always leaves in its first iteration, a literal close to `pi`,
# branches with the same code, a function with many parameters,
# docstrings, which are not Markdown (`>>>` lines of doctests), and
# redundant logic on bool locals (`while not done` that never sets `done`,
# `a and a`, `a and not a`).
CRATE_ALLOWS = """\
#![allow(non_snake_case, unused_variables, unused_assignments)]
#![allow(clippy::never_loop, clippy::approx_constant, clippy::if_same_then_else)]
#![allow(clippy::too_many_arguments, clippy::doc_lazy_continuation)]
#![allow(clippy::while_immutable_condition, clippy::eq_op, clippy::nonminimal_bool)]
#![allow(clippy::overly_complex_bool_expr)]"""


# Unoptimized PyO3 makes every Python operation several times slower, so the
# dev profile optimizes it and the runtime fully, and the generated code a
# little, which keeps rebuilds after an edit around a second. Packages are
# named instead of `"*"`, which would also optimize proc macros and double
# the time of a clean build.
DEV_PROFILE = """\
[profile.dev]
opt-level = 1

[profile.dev.package.pyo3]
opt-level = 3

[profile.dev.package.pyo3-ffi]
opt-level = 3

[profile.dev.package.steelix-runtime]
opt-level = 3
"""


class Mode(Enum):
    BIN = "bin"
    EXT = "ext"


@dataclass(frozen=True)
class CrateOptions:
    mode: Mode = Mode.BIN
    runtime_path: Path | None = None
    """Path dependency on `steelix-runtime`; `None` uses the published version."""
    standalone: bool = True
    """Add an empty `[workspace]` so the crate builds even inside another workspace."""
    fallback: bool = True
    """Mixed mode: keep unsupported statements as Python source (design §13).
    `False` makes them an error instead."""
    traceback_lines: bool = False
    """Emit `frame.line(n)` markers so tracebacks show statement lines
    instead of the `def` line (design §9)."""


def rust_module_name(name: str) -> str:
    """The Rust module for the Python module `name`, which as a script name
    (`my-tool.py`) need not be an identifier."""
    ident = re.sub(r"\W", "_", name)
    if not ident.isidentifier():
        ident = "m_" + ident
    # `main` and `lib` would collide with the crate root files.
    return Renamer([ident], {"main", "lib", "build"})[ident]


def render_module(
    parsed: ParsedModule,
    *,
    fallback: bool = True,
    role: ModuleRole | None = None,
    traceback_lines: bool = False,
) -> str:
    """The formatted Rust source for one Python module."""
    lowered = lower_module(parsed, fallback=fallback, role=role, traceback_lines=traceback_lines)
    return render(lowered)


def render(lowered: LoweredModule) -> str:
    """The formatted Rust source of a lowered module.

    The generated code is verified first (`rustast.verify`): a failure is a
    bug of steelix, reported instead of writing code that does not compile
    or does the wrong thing.
    """
    verify(lowered.file)
    return rustfmt(print_file(lowered.file))


def emit_crate(parsed: ParsedModule, out_dir: Path, options: CrateOptions) -> LoweredModule:
    """Write the crate; the result tells which statements stayed Python."""
    lowered = lower_module(
        parsed, fallback=options.fallback, traceback_lines=options.traceback_lines
    )
    module_rs = render(lowered)
    rust_mod = rust_module_name(parsed.name)
    src = out_dir / "src"
    src.mkdir(parents=True, exist_ok=True)
    (out_dir / "Cargo.toml").write_text(cargo_toml(parsed.name, options))
    (out_dir / "rustfmt.toml").write_text(config_text())
    (src / f"{rust_mod.removeprefix('r#')}.rs").write_text(module_rs)
    if options.mode is Mode.BIN:
        path = parsed.path or Path(parsed.filename)
        project = Path(os.path.dirname(os.path.realpath(path)))
        (out_dir / "build.rs").write_text(rustfmt(build_rs(project)))
        (src / "main.rs").write_text(rustfmt(main_rs(parsed, rust_mod)))
    else:
        (src / "lib.rs").write_text(rustfmt(lib_rs(parsed, rust_mod)))
    return lowered


# Names a module of the project must not take in the Rust module that
# declares it: the crate root's files, the names generated code uses (`use
# steelix_runtime as rt`, `pyo3::..`) and `mod.rs` of packages.
_TOP_RESERVED = frozenset({"main", "lib", "build", "steelix_runtime", "pyo3", "std", "core"})
_MOD_RESERVED = frozenset({"rt", "pyo3", "mod", "steelix_runtime"})


@dataclass(frozen=True)
class RustModule:
    """Where the generated code of a Python module goes."""

    source: SourceModule
    path: tuple[str, ...]
    """The Rust module path from the crate root: `("pkg", "sub")`."""
    children: tuple[str, ...]
    """The Rust names of a package's submodules."""

    @property
    def file(self) -> str:
        """The file under `src/`: `pkg/mod.rs` for a package, `pkg/sub.rs`."""
        parts = [p.removeprefix("r#") for p in self.path]
        if self.source.package:
            return "/".join([*parts, "mod.rs"])
        return "/".join(parts) + ".rs"

    @property
    def rust_path(self) -> str:
        return "::".join(self.path)


def rust_layout(project: Project) -> dict[str, RustModule]:
    """The Rust module of every source of the project, by `relpath`.

    Names are the Python names where they are Rust identifiers, so that
    `pkg/sub.py` is `src/pkg/sub.rs`. Other names are adapted (`my-tool` is
    `my_tool`, `type` is `r#type`, `mod` is `mod_`) and made unique among
    their siblings with a `_` suffix.
    """
    sources = project.sources()
    by_parent: dict[str, list[SourceModule]] = {}
    for source in sources:
        parent = "" if source is project.main and not project.run_module else _parent(source)
        by_parent.setdefault(parent, []).append(source)
    names: dict[str, str] = {}
    for parent, siblings in by_parent.items():
        reserved = _MOD_RESERVED | (_TOP_RESERVED if not parent else set())
        names |= _sibling_names(siblings, reserved)
    layout: dict[str, RustModule] = {}

    def path_of(source: SourceModule) -> tuple[str, ...]:
        if source is project.main and not project.run_module or "." not in source.name:
            return (names[source.relpath],)
        parent = project.modules[source.name.rpartition(".")[0]]
        return (*path_of(parent), names[source.relpath])

    for source in sources:
        children = tuple(
            names[child.relpath] for child in by_parent.get(source.name, []) if source.package
        )
        layout[source.relpath] = RustModule(source, path_of(source), children)
    return layout


def _parent(source: SourceModule) -> str:
    return source.name.rpartition(".")[0]


def _sibling_names(siblings: list[SourceModule], reserved: frozenset[str]) -> dict[str, str]:
    """Distinct Rust names for modules of one package, by `relpath`: names
    that keep their Python spelling first, renamed ones get `_` suffixes."""
    used = set(reserved)
    result: dict[str, str] = {}
    ordered = sorted(siblings, key=lambda s: s.relpath)
    wanted = {s.relpath: module_ident(s.name) for s in ordered}
    for source in ordered:
        last = source.name.rpartition(".")[2]
        if wanted[source.relpath] == last and last not in used:
            result[source.relpath] = last
            used.add(last)
    for source in ordered:
        if source.relpath in result:
            continue
        candidate = wanted[source.relpath]
        bare = candidate.removeprefix("r#")
        while bare in used:
            # A suffixed keyword is no keyword any more.
            bare += "_"
            candidate = bare
        result[source.relpath] = candidate
        used.add(bare)
    return result


def emit_project(
    project: Project, out_dir: Path, options: CrateOptions, *, name: str | None = None
) -> list[LoweredModule]:
    """Write the crate of a project: one Rust module per Python module.

    A project without other modules is written like a single module
    (`emit_crate`). `name` is the crate's (and executable's) name, by default
    the program's (`project_name`). Returns the lowered modules, the main
    module first.
    """
    if not project.modules and not project.run_module:
        parsed = project.parsed[project.main.relpath]
        return [emit_crate(parsed, out_dir, options)]
    layout = rust_layout(project)
    patches = cross_module_patches(project)
    src = out_dir / "src"
    src.mkdir(parents=True, exist_ok=True)
    lowered_modules: list[LoweredModule] = []
    for source in project.sources():
        rust = layout[source.relpath]
        parsed = project.parsed[source.relpath]
        own_names = [source.name]
        if source is project.main:
            own_names = ["__main__", *(own_names if source.name in project.modules else [])]
        role = ModuleRole(
            own_names=tuple(own_names),
            patched=None
            if source.name in patches and patches[source.name] is None
            else frozenset(patches.get(source.name) or ()),
            submodules=rust.children,
            rust_name=rust.path[-1].removeprefix("r#"),
        )
        lowered = lower_module(
            parsed,
            fallback=options.fallback,
            role=role,
            traceback_lines=options.traceback_lines,
        )
        target = src / rust.file
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(render(lowered))
        lowered_modules.append(lowered)
    (out_dir / "Cargo.toml").write_text(cargo_toml(name or project_name(project), options))
    (out_dir / "rustfmt.toml").write_text(config_text())
    if options.mode is Mode.BIN:
        (out_dir / "build.rs").write_text(rustfmt(build_rs(project.root)))
        (src / "main.rs").write_text(rustfmt(project_main_rs(project, layout)))
    else:
        (src / "lib.rs").write_text(rustfmt(project_lib_rs(project, layout)))
    return lowered_modules


def project_name(project: Project) -> str:
    """The program's name: the script's, or the package's for `-m pkg`."""
    name = project.main.name
    if project.run_module:
        name = name.removesuffix(".__main__").rpartition(".")[2]
    return name


def _modules_table(project: Project, layout: dict[str, RustModule], *, with_main: bool) -> str:
    modules = list(project.modules.values())
    if with_main and project.main.name not in project.modules:
        modules = sorted([*modules, project.main], key=lambda m: m.name)
    rows = []
    for module in modules:
        rust = layout[module.relpath]
        kind = "package" if module.package else "module"
        name = string_literal(module.name)
        rows.append(f"    NativeModule::{kind}({name}, {rust.rust_path}::exec_module),")
    return "\n".join(rows)


def _mod_lines(layout: dict[str, RustModule]) -> str:
    top = sorted(rust.path[0] for rust in layout.values() if len(rust.path) == 1)
    return "\n".join(f"mod {name};" for name in top)


def project_main_rs(project: Project, layout: dict[str, RustModule]) -> str:
    main = layout[project.main.relpath]
    file = string_literal(project.main.relpath)
    if project.run_module:
        what = f"`python -m {project_name(project)}`"
        run = (
            f"steelix_runtime::embed::run_module(&script, MODULES, "
            f"{string_literal(project.main.name)})"
        )
    else:
        what = f"`{project.main.relpath}`"
        run = (
            f"steelix_runtime::embed::run_project(&script, MODULES, {main.rust_path}::exec_module)"
        )
    return f"""\
//! Generated by steelix from {what} and the modules of its project.

{CRATE_ALLOWS}

{_mod_lines(layout)}

use steelix_runtime::NativeModule;

/// The project's modules: `import` runs their transpiled code instead of
/// their Python source.
const MODULES: &[NativeModule] = &[
{_modules_table(project, layout, with_main=project.run_module)}
];

fn main() -> std::process::ExitCode {{
    let script = steelix_runtime::script!({file});
    {run}
}}
"""


def project_lib_rs(project: Project, layout: dict[str, RustModule]) -> str:
    main = layout[project.main.relpath]
    name = project.main.name.rpartition(".")[2]
    return f"""\
//! Generated by steelix from `{project.main.relpath}` and the modules of its project.

{CRATE_ALLOWS}

use pyo3::prelude::*;

{_mod_lines(layout)}

use steelix_runtime::NativeModule;

/// The project's modules: `import` runs their transpiled code instead of
/// their Python source. They are next to the extension module.
const MODULES: &[NativeModule] = &[
{_modules_table(project, layout, with_main=True)}
];

#[pymodule(name = "{name}")]
fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {{
    steelix_runtime::init_extension(m, MODULES)?;
    {main.rust_path}::exec_module(&steelix_runtime::Module::new(m))
}}
"""


def crate_name(module_name: str) -> str:
    return module_name.strip("_").replace("_", "-") or "app"


def cargo_toml(module_name: str, options: CrateOptions) -> str:
    if options.runtime_path is not None:
        runtime = f'{{ path = "{options.runtime_path.as_posix()}" }}'
    else:
        runtime = f'"{RUNTIME_VERSION}"'
    lines = [
        "[package]",
        f'name = "{crate_name(module_name)}"',
        'version = "0.1.0"',
        'edition = "2024"',
        "publish = false",
        "",
    ]
    if options.mode is Mode.BIN:
        lines += ["[[bin]]", f'name = "{module_name}"', 'path = "src/main.rs"', ""]
    else:
        lines += ["[lib]", f'name = "{module_name}"', 'crate-type = ["cdylib"]', ""]
    lines += [
        "[dependencies]",
        f'pyo3 = "{PYO3_VERSION}"',
        f"steelix-runtime = {runtime}",
        "",
    ]
    if options.mode is Mode.BIN:
        lines += ["[build-dependencies]", f'pyo3-build-config = "{PYO3_VERSION}"', ""]
    if options.standalone:
        # Cargo only reads profiles from the workspace root.
        lines += [DEV_PROFILE, "[workspace]", ""]
    return "\n".join(lines)


def build_rs(project: Path) -> str:
    """The build script of an executable: how it finds Python and the project.

    A development build runs with the interpreter selected by `PYO3_PYTHON`
    (and its virtual environment) and the project directory it was
    transpiled from. `steelix build` sets `STEELIX_DIST`, for a program that
    finds both next to its executable instead (design §11).
    """
    return f"""\
/// The project directory the program was transpiled from.
const PROJECT: &str = {string_literal(str(project))};

fn main() {{
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=STEELIX_DIST");
    if std::env::var_os("STEELIX_DIST").is_some() {{
        // A build for distribution (`steelix build`): libpython, the
        // standard library and the project are next to the executable.
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/python/lib");
        return;
    }}
    // Let the executable find the libpython it was linked against, instead
    // of whichever libpython the system loader would pick.
    pyo3_build_config::add_libpython_rpath_link_args();
    // Run with the interpreter installation (and virtual environment) the
    // crate was built against, and the project's Python files.
    let executable = pyo3_build_config::get()
        .executable()
        .expect("PyO3 found no Python interpreter");
    println!("cargo:rustc-env=STEELIX_PYTHON={{executable}}");
    println!("cargo:rustc-env=STEELIX_PROJECT={{PROJECT}}");
}}
"""


def main_rs(parsed: ParsedModule, rust_mod: str) -> str:
    # `sys.argv[0]`, `__file__` and `sys.path[0]` follow from the script's
    # name in its project directory, which the runtime locates (design §11).
    path = parsed.path or Path(parsed.filename)
    file = string_literal(os.path.basename(os.path.realpath(path)))
    return f"""\
//! Generated by steelix from `{parsed.filename}`.

{CRATE_ALLOWS}

mod {rust_mod};

fn main() -> std::process::ExitCode {{
    let script = steelix_runtime::script!({file});
    steelix_runtime::embed::run_main(&script, {rust_mod}::exec_module)
}}
"""


def lib_rs(parsed: ParsedModule, rust_mod: str) -> str:
    return f"""\
//! Generated by steelix from `{parsed.filename}`.

{CRATE_ALLOWS}

use pyo3::prelude::*;

mod {rust_mod};

#[pymodule(name = "{parsed.name}")]
fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {{
    {rust_mod}::exec_module(&steelix_runtime::Module::new(m))
}}
"""


def default_runtime_path() -> Path | None:
    """The runtime crate of a source checkout, if steelix runs from one."""
    candidate = Path(__file__).resolve().parents[3] / "crates" / "steelix-runtime"
    return candidate if (candidate / "Cargo.toml").is_file() else None
