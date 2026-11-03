"""Write a complete Cargo crate for a transpiled module.

Two output modes share the generated module file, which only exposes
`exec_module(module)`:

* `bin` (default): an executable that embeds libpython and runs the module as
  `__main__`, like `python app.py`.
* `ext`: a cdylib PyO3 extension module that replaces the Python module.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

from steelix.frontend.parse import ParsedModule
from steelix.lower.module import LoweredModule, lower_module
from steelix.lower.names import Renamer
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
# branches with the same code.
CRATE_ALLOWS = """\
#![allow(non_snake_case, unused_variables, unused_assignments)]
#![allow(clippy::never_loop, clippy::approx_constant, clippy::if_same_then_else)]"""


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


def rust_module_name(name: str) -> str:
    """The Rust module for the Python module `name`, which as a script name
    (`my-tool.py`) need not be an identifier."""
    ident = re.sub(r"\W", "_", name)
    if not ident.isidentifier():
        ident = "m_" + ident
    # `main` and `lib` would collide with the crate root files.
    return Renamer([ident], {"main", "lib", "build"})[ident]


def render_module(parsed: ParsedModule, *, fallback: bool = True) -> str:
    """The formatted Rust source for one Python module."""
    return render(lower_module(parsed, fallback=fallback))


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
    lowered = lower_module(parsed, fallback=options.fallback)
    module_rs = render(lowered)
    rust_mod = rust_module_name(parsed.name)
    src = out_dir / "src"
    src.mkdir(parents=True, exist_ok=True)
    (out_dir / "Cargo.toml").write_text(cargo_toml(parsed.name, options))
    (out_dir / "rustfmt.toml").write_text(config_text())
    (src / f"{rust_mod.removeprefix('r#')}.rs").write_text(module_rs)
    if options.mode is Mode.BIN:
        (out_dir / "build.rs").write_text(rustfmt(BUILD_RS))
        (src / "main.rs").write_text(rustfmt(main_rs(parsed, rust_mod)))
    else:
        (src / "lib.rs").write_text(rustfmt(lib_rs(parsed, rust_mod)))
    return lowered


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


BUILD_RS = """\
fn main() {
    // Let the executable find the libpython it was linked against, instead
    // of whichever libpython the system loader would pick.
    pyo3_build_config::add_libpython_rpath_link_args();
    // Run with the interpreter installation (and virtual environment) the
    // crate was built against, selected by `PYO3_PYTHON`.
    let executable = pyo3_build_config::get()
        .executable()
        .expect("PyO3 found no Python interpreter");
    println!("cargo:rustc-env=STEELIX_PYTHON={executable}");
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
}
"""


def main_rs(parsed: ParsedModule, rust_mod: str) -> str:
    # What `python app.py` would see, fixed at transpile time: `sys.argv[0]`
    # is the path as given, `__file__` the absolute path and `sys.path[0]` the
    # directory with symlinks resolved.
    path = parsed.path or Path(parsed.filename)
    argv0 = string_literal(str(path))
    file = string_literal(os.path.abspath(path))
    directory = string_literal(os.path.dirname(os.path.realpath(path)))
    return f"""\
//! Generated by steelix from `{parsed.filename}`.

{CRATE_ALLOWS}

mod {rust_mod};

fn main() -> std::process::ExitCode {{
    let script = steelix_runtime::embed::Script {{
        argv0: {argv0},
        file: {file},
        dir: {directory},
    }};
    steelix_runtime::embed::run_main(env!("STEELIX_PYTHON"), &script, {rust_mod}::exec_module)
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
