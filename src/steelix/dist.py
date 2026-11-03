"""Assemble a program built by `steelix build` into a directory that runs
on another machine without Python installed (design §11).

```
dist/app/app                        the executable, rpath $ORIGIN/python/lib
dist/app/python/lib/libpython3.14.so.1.0
dist/app/python/lib/python3.14/     the standard library, compiled to .pyc
dist/app/python/lib/python3.14/site-packages/
dist/app/python/bin/python         sys.executable, a link to ../../app
dist/app/src/                       the project directory, sys.path[0]
```

The runtime (`steelix_runtime::embed`) finds these relative to its
executable; the names here must stay the same as its `DIST_*` constants.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tarfile
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path

from steelix.project import Project

DIST_PYTHON = "python"
DIST_EXECUTABLE = "python/bin/python"
DIST_PROJECT = "src"

# Parts of the standard library a program run by the bundled interpreter
# does not need: the GUI (Tk is not bundled), IDLE, the turtle demos,
# bootstrapping pip, and the files for building extension modules. CPython's
# test suite is not in the installation to begin with (python-build-standalone
# `install_only`), `test` is listed for installations that have it.
STDLIB_EXCLUDED = frozenset(
    {
        "idlelib",
        "tkinter",
        "turtle.py",
        "turtledemo",
        "ensurepip",
        "test",
    }
)

# Distributions of the build interpreter's `site-packages` that are not
# copied: steelix itself (a build tool), and pip, which installs packages
# into an environment rather than being used by programs.
EXCLUDED_DISTRIBUTIONS = frozenset({"steelix", "pip"})

_INSPECT = """
import importlib.metadata, json, os, sys, sysconfig

excluded = set(json.loads(sys.argv[1]))
files = []
for dist in importlib.metadata.distributions():
    name = (dist.metadata["Name"] or "").lower().replace("_", "-")
    if name in excluded:
        files += [os.path.abspath(dist.locate_file(f)) for f in dist.files or ()]
        info_dir = getattr(dist, "_path", None)
        if info_dir is not None:
            files.append(os.path.abspath(info_dir))
site = []
for key in ("purelib", "platlib"):
    path = sysconfig.get_path(key)
    if path not in site and os.path.isdir(path):
        site.append(path)
print(json.dumps({
    "version": sysconfig.get_python_version(),
    "base_prefix": sys.base_prefix,
    "stdlib": sysconfig.get_path("stdlib"),
    "soname": sysconfig.get_config_var("INSTSONAME"),
    "shared": bool(sysconfig.get_config_var("Py_ENABLE_SHARED")),
    "site_packages": site,
    "excluded": files,
}))
"""


class DistError(Exception):
    """A problem with the inputs of `steelix build`."""


@dataclass(frozen=True)
class PythonInstall:
    """The interpreter a program is built with, and what of it goes into the
    distribution: its base installation's libpython and standard library,
    and the `site-packages` of the interpreter itself (a virtual environment
    usually)."""

    executable: Path
    version: str
    """`3.14`."""
    libpython: Path
    stdlib: Path
    site_packages: tuple[Path, ...]
    excluded: frozenset[Path]
    """Files of `site-packages` that are not copied (`EXCLUDED_DISTRIBUTIONS`)."""


def inspect_python(executable: Path) -> PythonInstall:
    """Asks the interpreter `executable` where its parts are."""
    try:
        proc = subprocess.run(
            [str(executable), "-I", "-c", _INSPECT, json.dumps(sorted(EXCLUDED_DISTRIBUTIONS))],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as err:
        raise DistError(f"can not run the Python interpreter {executable}: {err}") from err
    info = json.loads(proc.stdout)
    running = f"{sys.version_info.major}.{sys.version_info.minor}"
    if info["version"] != running:
        raise DistError(
            f"{executable} is Python {info['version']}, but steelix transpiles for {running}"
        )
    libpython = Path(info["base_prefix"]) / "lib" / (info["soname"] or "")
    if not info["shared"] or not libpython.is_file():
        raise DistError(
            f"{executable} has no shared libpython (looked for {libpython}); use a "
            "python-build-standalone or other relocatable installation built with "
            "--enable-shared"
        )
    return PythonInstall(
        executable=executable,
        version=info["version"],
        libpython=libpython,
        stdlib=Path(info["stdlib"]),
        site_packages=tuple(Path(p) for p in info["site_packages"]),
        excluded=frozenset(Path(p) for p in info["excluded"]),
    )


@dataclass(frozen=True)
class DistOptions:
    sources: bool = True
    """Ship the Python files of the transpiled modules (for traceback source
    lines, `inspect.getsource` and `linecache`). The program runs without them."""
    full_stdlib: bool = False
    """Copy the whole standard library instead of leaving out `STDLIB_EXCLUDED`."""
    compile: bool = True
    """Compile the standard library and `site-packages` to `.pyc`, so that
    the program does not compile them at every start when the directory is
    not writable."""
    data: tuple[str, ...] = ()
    """More files or directories of the project to copy, relative to it."""


def assemble(
    binary: Path,
    name: str,
    python: PythonInstall,
    project: Project,
    dist_dir: Path,
    options: DistOptions,
) -> list[str]:
    """Writes the distribution of the program `binary` into `dist_dir`.
    Returns warnings about things that will not work on another machine."""
    _prepare_dir(dist_dir)
    warnings: list[str] = []
    shutil.copy2(binary, dist_dir / name)
    home = dist_dir / DIST_PYTHON
    lib = home / "lib"
    lib.mkdir(parents=True)
    shutil.copy2(python.libpython, lib / python.libpython.name)
    stdlib = lib / f"python{python.version}"
    excluded = set() if options.full_stdlib else STDLIB_EXCLUDED
    shutil.copytree(python.stdlib, stdlib, ignore=_stdlib_ignore(python.stdlib, excluded))
    site_packages = stdlib / "site-packages"
    site_packages.mkdir()
    for source in python.site_packages:
        shutil.copytree(
            source,
            site_packages,
            ignore=_site_ignore(python.excluded),
            dirs_exist_ok=True,
            symlinks=True,
        )
        warnings += _pth_warnings(site_packages)
    executable = dist_dir / DIST_EXECUTABLE
    executable.parent.mkdir(parents=True)
    executable.symlink_to(os.path.relpath(dist_dir / name, executable.parent))
    _copy_project(project, dist_dir / DIST_PROJECT, options)
    if options.compile:
        compile_tree(python.executable, stdlib)
    return warnings


def _prepare_dir(dist_dir: Path) -> None:
    """Empties `dist_dir`, which must be new or an earlier distribution."""
    if dist_dir.exists():
        if not (dist_dir / DIST_PYTHON).is_dir() and any(dist_dir.iterdir()):
            raise DistError(f"{dist_dir} exists and is not a distribution made by steelix")
        shutil.rmtree(dist_dir)
    dist_dir.mkdir(parents=True)


def _stdlib_ignore(root: Path, excluded: Iterable[str]) -> Callable[[str, list[str]], set[str]]:
    excluded = set(excluded)

    def ignore(directory: str, names: list[str]) -> set[str]:
        ignored = {n for n in names if n == "__pycache__"}
        if Path(directory) == root:
            # The base installation's packages (pip) are not the program's:
            # its site-packages come from the build interpreter.
            ignored |= {n for n in names if n in excluded or n == "site-packages"}
            if excluded:
                ignored |= {n for n in names if n.startswith("config-")}
        if excluded and Path(directory).name == "lib-dynload":
            ignored |= {n for n in names if n.startswith("_tkinter.")}
        return ignored

    return ignore


def _site_ignore(excluded: frozenset[Path]) -> Callable[[str, list[str]], set[str]]:
    def ignore(directory: str, names: list[str]) -> set[str]:
        base = Path(os.path.abspath(directory))
        return {n for n in names if n == "__pycache__" or base / n in excluded}

    return ignore


def _pth_warnings(site_packages: Path) -> list[str]:
    """`.pth` files that add directories outside `site-packages` (editable
    installs), which do not exist on another machine."""
    warnings = []
    for pth in sorted(site_packages.glob("*.pth")):
        for line in pth.read_text(errors="replace").splitlines():
            line = line.strip()
            if line and not line.startswith(("#", "import ")) and os.path.isabs(line):
                warnings.append(
                    f"{pth.name} adds {line} to sys.path (an editable install?), which "
                    "is not part of the distribution"
                )
    return warnings


def project_files(project: Project, options: DistOptions) -> list[str]:
    """The files of the project that go into the distribution, relative to
    the project directory: the transpiled modules' sources (with
    `options.sources`), every other Python file of the project's packages
    (submodules imported by computed names), the modules that import from
    source (`Project.left_out`), and `options.data`."""
    root = project.root
    transpiled = {source.relpath for source in project.sources()}
    files: set[str] = set(transpiled) if options.sources else set()
    packages = {Path(source.relpath).parent for source in project.sources() if source.package}
    for package in packages:
        for path in (root / package).rglob("*.py"):
            relpath = path.relative_to(root).as_posix()
            if "__pycache__" in path.parts or relpath in transpiled:
                continue
            files.add(relpath)
    for item in [*sorted(project.left_out), *options.data]:
        path = Path(item) if Path(item).is_absolute() else root / item
        try:
            path.resolve().relative_to(root)
        except ValueError:
            raise DistError(f"{item} is not in the project directory {root}") from None
        if path.is_dir():
            files |= {
                p.relative_to(root).as_posix()
                for p in path.rglob("*")
                if p.is_file() and "__pycache__" not in p.parts
            }
        elif path.is_file():
            files.add(path.relative_to(root).as_posix())
        else:
            raise DistError(f"{item}: no such file or directory in {root}")
    return sorted(files)


def _copy_project(project: Project, target: Path, options: DistOptions) -> None:
    target.mkdir()
    for relpath in project_files(project, options):
        destination = target / relpath
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(project.root / relpath, destination)


def compile_tree(python: Path, directory: Path) -> None:
    """Compiles the Python files under `directory` to `.pyc` with the
    interpreter of the program. Files that do not compile (test data of some
    packages) are skipped, as when importing them."""
    subprocess.run(
        [str(python), "-I", "-m", "compileall", "-q", "-j", "0", str(directory)],
        stdout=subprocess.DEVNULL,
        check=False,
    )


def archive(dist_dir: Path) -> Path:
    """Packs the distribution into `<dist_dir>.tar.gz`, keeping the link."""
    target = dist_dir.with_name(dist_dir.name + ".tar.gz")
    with tarfile.open(target, "w:gz") as tar:
        tar.add(dist_dir, arcname=dist_dir.name)
    return target


def tree_size(path: Path) -> tuple[int, int]:
    """Bytes and number of files under `path`, not following links."""
    total = files = 0
    for directory, _, names in os.walk(path):
        for name in names:
            stat = os.lstat(os.path.join(directory, name))
            total += stat.st_size
            files += 1
    return total, files


def format_size(size: int) -> str:
    return f"{size / 1e6:.1f} MB"
