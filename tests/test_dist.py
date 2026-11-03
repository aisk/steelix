"""Distribution (`steelix build`, design §11).

Unit tests for the parts of the distribution and the command line, and
end-to-end tests that build programs and run them inside `bwrap` with
nothing but `/usr` and the distribution visible: no home directory, no uv
Python installation, no repository, no virtual environment, an empty
`/tmp` and an empty environment apart from `PATH`. The programs are built
with a virtual environment that has a local package (`greeting`) in its
`site-packages`, made without network access.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tarfile
from pathlib import Path

import pytest
from conftest import DEV_PROFILE, E2E_DIR, PYTHON, ROOT, RUNTIME, Workspace, cargo_env
from test_e2e import Outcome, run

from steelix import dist
from steelix.cli import build_parser, dist_rustflags, main
from steelix.emit.crate import CrateOptions, Mode, emit_crate
from steelix.frontend.parse import parse_file
from steelix.project import find_project

DIST_DIR = ROOT / "tests" / "dist_apps"
PROJECTS_DIR = ROOT / "tests" / "projects"
HELLO = DIST_DIR / "hello" / "app.py"
MPAPP = DIST_DIR / "mp" / "mpapp.py"
MP_DIR = DIST_DIR / "mp"
PKGAPP = DIST_DIR / "pkgapp" / "app.py"
# Arguments that look like the command line of a `multiprocessing` child:
# they belong to the program, which must not run them (design §11).
INJECTION = ["-c", "from multiprocessing.spawn import spawn_main; print('INJECTED')"]
JSON_PROJECT = PROJECTS_DIR / "stdlib_json"

# Where the distribution is mounted inside the sandbox: unrelated to any
# path of the build machine.
MOUNT = "/opt/program"


# Unit tests


def test_build_parser_defaults() -> None:
    args = build_parser().parse_args(["build", "app.py"])
    assert args.command == "build"
    assert args.output == Path("dist")
    assert args.build_dir == Path("build")
    assert args.sources is True
    assert args.full_stdlib is False
    assert args.python is None
    args = build_parser().parse_args(
        ["build", "-m", "pkg", "--no-sources", "--archive", "--python", "py", "--exclude", "x"]
    )
    assert (args.module, args.sources, args.archive, args.exclude) == ("pkg", False, True, ["x"])


def test_inspect_python_finds_the_build_interpreter() -> None:
    install = dist.inspect_python(PYTHON)
    assert install.version == "3.14"
    assert install.libpython.name == "libpython3.14.so.1.0"
    assert (install.stdlib / "os.py").is_file()
    assert install.site_packages == (ROOT / ".venv" / "lib" / "python3.14" / "site-packages",)
    # steelix itself (an editable install) is a build tool, not part of programs.
    names = {path.name for path in install.excluded}
    assert "steelix.pth" in names
    assert "steelix-0.1.0.dist-info" in names


def test_inspect_python_rejects_what_it_can_not_run(tmp_path: Path) -> None:
    with pytest.raises(dist.DistError, match="can not run"):
        dist.inspect_python(tmp_path / "python")


def test_inspect_python_venv_with_system_site_packages(tmp_path: Path) -> None:
    """A virtual environment that includes the system site-packages (which
    are not copied) is reported; a `.dist-info` without `METADATA` does not
    stop the inspection."""
    venv = tmp_path / "venv"
    subprocess.run(
        [str(PYTHON), "-m", "venv", "--without-pip", "--system-site-packages", str(venv)],
        check=True,
    )
    (venv / "lib" / "python3.14" / "site-packages" / "broken-1.0.dist-info").mkdir()
    install = dist.inspect_python(venv / "bin" / "python")
    assert install.system_site_packages == venv
    assert dist.inspect_python(PYTHON).system_site_packages is None


def fake_install(tmp_path: Path) -> dist.PythonInstall:
    """A Python installation laid out like python-build-standalone's."""
    base = tmp_path / "base"
    stdlib = base / "lib" / "python3.14"
    files = [
        "os.py",
        "json/__init__.py",
        "json/__pycache__/__init__.cpython-314.pyc",
        "idlelib/idle.py",
        "tkinter/__init__.py",
        "turtle.py",
        "ensurepip/__init__.py",
        "config-3.14-x86_64-linux-gnu/Makefile",
        "lib-dynload/_dbm.cpython-314-x86_64-linux-gnu.so",
        "lib-dynload/_tkinter.cpython-314-x86_64-linux-gnu.so",
        "site-packages/pip/__init__.py",
    ]
    for name in files:
        (stdlib / name).parent.mkdir(parents=True, exist_ok=True)
        (stdlib / name).write_text("")
    libpython = base / "lib" / "libpython3.14.so.1.0"
    libpython.write_text("")
    site = tmp_path / "venv" / "site-packages"
    for name in ["greeting/__init__.py", "steelix.pth", "steelix-0.1.0.dist-info/RECORD"]:
        (site / name).parent.mkdir(parents=True, exist_ok=True)
        (site / name).write_text("")
    (site / "editable.pth").write_text("/home/someone/src/lib\n")
    # A separate platlib (purelib != platlib), merged into the same directory.
    platlib = tmp_path / "venv" / "platlib"
    (platlib / "native").mkdir(parents=True)
    (platlib / "native" / "__init__.py").write_text("")
    return dist.PythonInstall(
        executable=Path("/nonexistent/python"),
        version="3.14",
        libpython=libpython,
        stdlib=stdlib,
        site_packages=(site, platlib),
        excluded=frozenset({site / "steelix.pth", site / "steelix-0.1.0.dist-info"}),
    )


def files_under(path: Path) -> list[str]:
    return sorted(p.relative_to(path).as_posix() for p in path.rglob("*") if not p.is_dir())


def test_assemble_layout(tmp_path: Path) -> None:
    install = fake_install(tmp_path)
    binary = tmp_path / "target" / "hello"
    binary.parent.mkdir()
    binary.write_text("")
    project_dir = tmp_path / "project"
    project_dir.mkdir()
    (project_dir / "hello.py").write_text("print(1)\n")
    project = find_project(project_dir / "hello.py")
    target = tmp_path / "dist" / "hello"
    options = dist.DistOptions(compile=False)
    warnings = dist.assemble(binary, "hello", install, project, target, options)
    assert files_under(target) == [
        "hello",
        "python/bin/python",
        "python/lib/libpython3.14.so.1.0",
        "python/lib/python3.14/json/__init__.py",
        "python/lib/python3.14/lib-dynload/_dbm.cpython-314-x86_64-linux-gnu.so",
        "python/lib/python3.14/os.py",
        "python/lib/python3.14/site-packages/editable.pth",
        "python/lib/python3.14/site-packages/greeting/__init__.py",
        "python/lib/python3.14/site-packages/native/__init__.py",
        "src/hello.py",
    ]
    # `sys.executable` is the program itself, called as `python`.
    assert os.readlink(target / "python/bin/python") == "../../hello"
    assert warnings == [
        "editable.pth adds /home/someone/src/lib to sys.path (an editable install?), "
        "which is not part of the distribution"
    ]
    # A second build replaces the first; the full standard library keeps
    # everything but the base installation's site-packages and caches.
    options = dist.DistOptions(compile=False, full_stdlib=True, sources=False)
    dist.assemble(binary, "hello", install, project, target, options)
    stdlib = files_under(target / "python/lib/python3.14")
    assert "idlelib/idle.py" in stdlib
    assert "config-3.14-x86_64-linux-gnu/Makefile" in stdlib
    assert "ensurepip/__init__.py" in stdlib
    assert "site-packages/greeting/__init__.py" in stdlib
    assert not any(name.startswith("site-packages/pip") for name in stdlib)
    assert not any("__pycache__" in name for name in stdlib)
    assert not (target / "src" / "hello.py").exists()


def test_assemble_refuses_to_replace_other_directories(tmp_path: Path) -> None:
    install = fake_install(tmp_path)
    target = tmp_path / "important"
    target.mkdir()
    (target / "notes.txt").write_text("keep me")
    project_dir = tmp_path / "project"
    project_dir.mkdir()
    (project_dir / "hello.py").write_text("")
    project = find_project(project_dir / "hello.py")
    with pytest.raises(dist.DistError, match="not a distribution"):
        dist.assemble(tmp_path / "x", "hello", install, project, target, dist.DistOptions())
    assert (target / "notes.txt").read_text() == "keep me"
    with pytest.raises(dist.DistError, match="not a directory"):
        dist.assemble(
            tmp_path / "x", "hello", install, project, target / "notes.txt", dist.DistOptions()
        )
    assert (target / "notes.txt").read_text() == "keep me"


def test_package_data_and_unshipped_modules(tmp_path: Path) -> None:
    """Every file of a package is shipped (data files for
    `importlib.resources`), `--no-sources` leaves out only the transpiled
    modules, and Python files at the top of the project that are neither
    transpiled nor shipped are reported."""
    root = tmp_path / "project"
    files = {
        "app.py": "import pkg\n",
        "dyn.py": "",
        "notes.txt": "",
        "pkg/__init__.py": "",
        "pkg/data.txt": "",
        "pkg/plugin.py": "",
        "pkg/templates/page.html": "",
        "pkg/__pycache__/__init__.cpython-314.pyc": "",
    }
    for name, text in files.items():
        (root / name).parent.mkdir(parents=True, exist_ok=True)
        (root / name).write_text(text)
    project = find_project(root / "app.py")
    shipped = dist.project_files(project, dist.DistOptions(sources=False))
    assert shipped == ["pkg/data.txt", "pkg/plugin.py", "pkg/templates/page.html"]
    with_sources = dist.project_files(project, dist.DistOptions())
    assert with_sources == sorted([*shipped, "app.py", "pkg/__init__.py"])
    [warning] = dist.unshipped_warnings(project, shipped)
    assert warning.startswith("not in the distribution: dyn.py in ")
    assert warning.endswith("add them with --include dyn")
    project = find_project(root / "app.py", include=["dyn"])
    assert dist.unshipped_warnings(project, dist.project_files(project, dist.DistOptions())) == []


def pyc_flags(pyc: Path) -> int:
    """The flags of a `.pyc` header (PEP 552): 0 for timestamps, 1 for an
    unchecked hash, 3 for a checked hash."""
    return int.from_bytes(pyc.read_bytes()[4:8], "little")


def test_compile_tree(tmp_path: Path) -> None:
    """Hash based `.pyc` (copies need not keep modification times); files
    that do not compile are reported, a failing `compileall` is an error."""
    (tmp_path / "good.py").write_text("X = 1\n")
    (tmp_path / "bad.py").write_text("def (:\n")
    warnings = dist.compile_tree(PYTHON, [tmp_path], "unchecked-hash")
    assert warnings == [f"does not compile, importing it fails: {tmp_path / 'bad.py'}"]
    assert pyc_flags(tmp_path / "__pycache__" / "good.cpython-314.pyc") == 1
    assert dist.compile_tree(PYTHON, [tmp_path / "good.py"], "checked-hash") == []
    assert pyc_flags(tmp_path / "__pycache__" / "good.cpython-314.pyc") == 3
    with pytest.raises(dist.DistError, match="compiling the Python files to .pyc failed"):
        dist.compile_tree(PYTHON, [tmp_path], "no-such-mode")


def test_project_files() -> None:
    """The transpiled modules' sources (optional), the other Python files of
    the project's packages, and the modules that import from source."""
    root = PROJECTS_DIR / "inventory"
    project = find_project(root / "app.py", include=[str(root / "plugins")], exclude=["legacy"])
    with_sources = dist.project_files(project, dist.DistOptions())
    transpiled = {source.relpath for source in project.sources()}
    assert transpiled <= set(with_sources)
    assert "legacy.py" in project.left_out
    assert "legacy.py" in with_sources
    without = dist.project_files(project, dist.DistOptions(sources=False))
    assert not transpiled & set(without)
    assert "legacy.py" in without
    with_data = dist.project_files(project, dist.DistOptions(sources=False, data=("legacy.py",)))
    assert with_data == without
    with pytest.raises(dist.DistError, match="not in the project"):
        dist.project_files(project, dist.DistOptions(data=(str(ROOT / "README.md"),)))


def test_dist_rustflags_remap_build_paths() -> None:
    """The remappings are added to the user's rustflags: appended to those
    of the environment, or joined by cargo with those of its configuration
    (`--config build.rustflags`)."""
    env, args = dist_rustflags({"RUSTFLAGS": "-C debuginfo=1", "CARGO_HOME": "/c"}, Path("/r/rt"))
    assert args == []
    assert env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f") == [
        "-C",
        "debuginfo=1",
        "--remap-path-prefix=/c/registry/src=cargo",
        "--remap-path-prefix=/r=steelix",
    ]
    encoded = {"CARGO_ENCODED_RUSTFLAGS": "-Ctarget-cpu=native", "CARGO_HOME": "/c"}
    env, args = dist_rustflags(encoded, None)
    assert env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f") == [
        "-Ctarget-cpu=native",
        "--remap-path-prefix=/c/registry/src=cargo",
    ]
    env, args = dist_rustflags({"CARGO_HOME": "/c"}, None)
    assert env == {}
    assert args == ["--config", 'build.rustflags=["--remap-path-prefix=/c/registry/src=cargo"]']


# End-to-end tests

needs_bwrap = pytest.mark.skipif(shutil.which("bwrap") is None, reason="bwrap is not installed")


def sandbox(dist_dir: Path, args: list[str], data: Path | None = None) -> list[str]:
    """`args` (paths under `MOUNT`) run inside bwrap: only `/usr` (and the
    `/lib`, `/bin` links into it), the loader cache, `/proc`, `/dev`, an
    empty `/tmp`, the distribution at `MOUNT` and `data` at `/data` (the
    working directory) are visible."""
    command = ["bwrap", "--ro-bind", "/usr", "/usr"]
    for name in ("bin", "sbin", "lib", "lib32", "lib64"):
        path = Path("/") / name
        if path.is_symlink():
            command += ["--symlink", os.readlink(path), str(path)]
        elif path.is_dir():
            command += ["--ro-bind", str(path), str(path)]
    if Path("/etc/ld.so.cache").exists():
        command += ["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]
    command += ["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]
    command += ["--ro-bind", str(dist_dir), MOUNT]
    if data is not None:
        command += ["--ro-bind", str(data), "/data"]
    command += ["--unshare-all", "--die-with-parent", "--clearenv"]
    command += ["--setenv", "PATH", "/usr/bin:/bin", "--chdir", "/data" if data else "/"]
    return [*command, "--", *args]


@pytest.fixture(scope="module")
def build_venv() -> Path:
    """A virtual environment of the build interpreter with a local package
    in its `site-packages` (no network). At a fixed path, so that cargo
    keeps the dependencies built for it between test runs."""
    venv = E2E_DIR / "dist-venv"
    python = venv / "bin" / "python"
    if not python.exists():
        subprocess.run([str(PYTHON), "-m", "venv", "--without-pip", str(venv)], check=True)
    site = venv / "lib" / "python3.14" / "site-packages"
    package = site / "greeting"
    package.mkdir(exist_ok=True)
    (package / "__init__.py").write_text('def hello(name):\n    return f"hello, {name}"\n')
    info = site / "greeting-1.0.dist-info"
    info.mkdir(exist_ok=True)
    (info / "METADATA").write_text("Metadata-Version: 2.1\nName: greeting\nVersion: 1.0\n")
    (info / "RECORD").write_text("greeting/__init__.py,,\ngreeting-1.0.dist-info/METADATA,,\n")
    return python


def steelix_build(python: Path, *args: str) -> Path:
    """Runs `steelix build ... -o E2E_DIR/dist` and returns the output
    directory. The crates share a target directory."""
    output = E2E_DIR / "dist"
    with pytest.MonkeyPatch.context() as patch:
        patch.setenv("CARGO_TARGET_DIR", str(E2E_DIR / "target-dist"))
        command = ["build", *args, "-o", str(output), "--build-dir", str(E2E_DIR / "dist-build")]
        command += ["--python", str(python), "--runtime-path", str(RUNTIME), "-q"]
        assert main(command) == 0
    return output


def python_run(python: Path, args: list[str], cwd: Path) -> Outcome:
    """CPython running the sources, without writing `__pycache__` into them."""
    return run([str(python), "-B", *args], cwd)


@pytest.fixture(scope="module")
def hello_dists(build_venv: Path) -> tuple[Path, Path]:
    """The hello program with and without its sources."""
    output = steelix_build(build_venv, str(HELLO), "--archive")
    with_sources = E2E_DIR / "dist-hello"
    if with_sources.exists():
        shutil.rmtree(with_sources)
    (output / "app").rename(with_sources)
    shutil.move(output / "app.tar.gz", E2E_DIR / "dist-hello.tar.gz")
    output = steelix_build(build_venv, str(HELLO), "--no-sources")
    return with_sources, output / "app"


@pytest.mark.e2e
@needs_bwrap
def test_sandbox_hides_the_build_machine(hello_dists: tuple[Path, Path]) -> None:
    dist_dir, _ = hello_dists
    hidden = [Path.home(), ROOT, ROOT / ".venv", dist.inspect_python(PYTHON).stdlib, Path("/home")]
    code = f"import os, sys; print([os.path.exists(p) for p in {[str(p) for p in hidden]!r}])"
    code += "; print(os.listdir('/tmp'), sys.prefix)"
    result = run(sandbox(dist_dir, [f"{MOUNT}/python/bin/python", "-c", code]), ROOT)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [str([False] * len(hidden)), f"[] {MOUNT}/python"]
    # bwrap sets `PWD` (CPython adds `LC_CTYPE` when it coerces the C locale).
    environment = run(sandbox(dist_dir, ["/usr/bin/env"]), ROOT)
    assert environment.stdout.splitlines() == ["PATH=/usr/bin:/bin", "PWD=/"]


@pytest.mark.e2e
@needs_bwrap
def test_built_program_runs_in_a_clean_environment(
    hello_dists: tuple[Path, Path], build_venv: Path
) -> None:
    dist_dir, _ = hello_dists
    assert sorted(p.name for p in dist_dir.iterdir()) == ["app", "python", "src"]
    assert (dist_dir / "src" / "app.py").is_file()
    site = dist_dir / "python" / "lib" / "python3.14" / "site-packages"
    assert sorted(p.name for p in site.iterdir()) == ["greeting", "greeting-1.0.dist-info"]
    # No path of the build machine in the program (rpath, interpreter, project).
    binary = (dist_dir / "app").read_bytes()
    for path in [ROOT, Path.home(), build_venv.parent.parent]:
        assert str(path).encode() not in binary
    expected = python_run(build_venv, [str(HELLO), "x", "-y"], ROOT)
    assert expected.returncode == 0, expected.stderr
    actual = run(sandbox(dist_dir, [f"{MOUNT}/app", "x", "-y"]), ROOT)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)


@pytest.mark.e2e
@needs_bwrap
def test_tracebacks_with_and_without_sources(
    hello_dists: tuple[Path, Path], build_venv: Path
) -> None:
    """The program does not need its sources; with them, tracebacks show
    the source lines (of the `def` line for native frames, design §9)."""
    with_sources, without_sources = hello_dists
    assert not (without_sources / "src" / "app.py").exists()
    expected = python_run(build_venv, [str(HELLO), "crash"], ROOT)
    for dist_dir, source_lines in [(with_sources, True), (without_sources, False)]:
        actual = run(sandbox(dist_dir, [f"{MOUNT}/app", "crash"]), ROOT)
        assert actual.returncode == expected.returncode == 1
        assert actual.last_stderr_line == expected.last_stderr_line == "ValueError: negative: -1"
        frames = [line for line in actual.traceback if line.startswith("  File ")]
        assert frames == [
            f'  File "{MOUNT}/src/app.py", line 1, in <module>',
            f'  File "{MOUNT}/src/app.py", line 32, in crash',
            f'  File "{MOUNT}/src/app.py", line 26, in check',
        ]
        assert ("    def check(value):" in actual.traceback) is source_lines
        assert len(actual.traceback) == (8 if source_lines else 5)
        ok = run(sandbox(dist_dir, [f"{MOUNT}/app"]), ROOT)
        assert ok.returncode == 0, ok.stderr


@pytest.mark.e2e
@needs_bwrap
def test_child_command_line_is_the_programs(
    hello_dists: tuple[Path, Path], build_venv: Path
) -> None:
    """`app -c "from multiprocessing.spawn import ..."` passes the arguments
    to the script, like `python app.py -c ...`: only the bundled
    interpreter runs the children of `multiprocessing`."""
    dist_dir, _ = hello_dists
    expected = python_run(build_venv, [str(HELLO), *INJECTION], ROOT)
    assert expected.returncode == 0, expected.stderr
    assert "INJECTED" not in expected.stdout.splitlines()
    actual = run(sandbox(dist_dir, [f"{MOUNT}/app", *INJECTION]), ROOT)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)


@pytest.mark.e2e
def test_distribution_ignores_the_python_environment(
    hello_dists: tuple[Path, Path], tmp_path: Path
) -> None:
    """The program of a distribution ignores `PYTHON*` variables and the
    user's site-packages, unless `STEELIX_PYTHON_ENV=1`; the bundled
    interpreter honours the variables but, like a virtual environment, not
    the user's site-packages. Outside bwrap, which hides the home directory."""
    dist_dir, _ = hello_dists
    home = tmp_path / "home"
    user_site = home / ".local" / "lib" / "python3.14" / "site-packages"
    user_site.mkdir(parents=True)
    (user_site / "usercustomize.py").write_text("print('USERCUSTOMIZE')\n")
    shadow = tmp_path / "shadow"
    shadow.mkdir()
    (shadow / "json.py").write_text("print('SHADOW json')\n")
    clean = {"PATH": os.environ["PATH"]}
    polluted = {
        **clean,
        "HOME": str(home),
        "PYTHONPATH": str(shadow),
        "PYTHONHOME": "/nonexistent",
        "PYTHONSAFEPATH": "1",
        "PYTHONUTF8": "1",
    }

    def program(env: dict[str, str], *args: str) -> subprocess.CompletedProcess[str]:
        command = [str(dist_dir / "app"), *args]
        return subprocess.run(command, env=env, capture_output=True, text=True, check=False)

    expected = program(clean)
    assert expected.returncode == 0, expected.stderr
    actual = program(polluted)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)
    opt_in = program({**polluted, "STEELIX_PYTHON_ENV": "1"})
    assert opt_in.stdout.splitlines()[:2] == ["USERCUSTOMIZE", "SHADOW json"], opt_in.stderr
    code = "import sys, site; print(site.ENABLE_USER_SITE, sys.flags.ignore_environment)"
    interpreter = subprocess.run(
        [str(dist_dir / "python" / "bin" / "python"), "-c", code],
        env=polluted,
        capture_output=True,
        text=True,
        check=False,
    )
    assert interpreter.stdout.splitlines() == ["False 0"], interpreter.stderr


@pytest.mark.e2e
def test_bytecode_does_not_depend_on_modification_times(hello_dists: tuple[Path, Path]) -> None:
    """The standard library and site-packages are compiled with unchecked
    hashes: a copy that does not keep modification times (`cp -r`, `scp`)
    does not compile them again at every start."""
    dist_dir, _ = hello_dists
    stdlib = dist_dir / "python" / "lib" / "python3.14"
    for pyc in [
        stdlib / "json" / "__pycache__" / "__init__.cpython-314.pyc",
        stdlib / "site-packages" / "greeting" / "__pycache__" / "__init__.cpython-314.pyc",
    ]:
        assert pyc_flags(pyc) == 1, pyc


@pytest.mark.e2e
def test_archive_keeps_the_layout(hello_dists: tuple[Path, Path], tmp_path: Path) -> None:
    with tarfile.open(E2E_DIR / "dist-hello.tar.gz") as tar:
        tar.extractall(tmp_path, filter="tar")
    link = tmp_path / "app" / "python" / "bin" / "python"
    assert os.readlink(link) == "../../app"
    result = run([str(tmp_path / "app" / "app")], tmp_path)
    assert result.returncode == 0, result.stderr
    assert "hello, child" in result.stdout


JSON_COMMANDS = [
    [],
    ["tool", "--sort-keys", "--indent", "1", "sample.json"],
    ["tool", "--json-lines", "--compact", "lines.jsonl"],
    ["tool", "missing.json"],
    ["crash"],
]


@pytest.fixture(scope="module")
def json_data(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """The data files of the json project, without its Python files."""
    data = tmp_path_factory.mktemp("json-data")
    for name in ["sample.json", "lines.jsonl"]:
        shutil.copyfile(JSON_PROJECT / name, data / name)
    return data


@pytest.mark.e2e
@needs_bwrap
def test_json_project_runs_in_a_clean_environment(build_venv: Path, json_data: Path) -> None:
    output = steelix_build(build_venv, str(JSON_PROJECT / "app.py"), "--no-sources")
    dist_dir = output / "app"
    # Only the module of the `json` package that the program does not import
    # (and so is not transpiled) is there, in case it is imported by name.
    shipped = [p.relative_to(dist_dir / "src").as_posix() for p in (dist_dir / "src").rglob("*.py")]
    assert shipped == ["json/__main__.py"]
    for args in JSON_COMMANDS:
        expected = python_run(build_venv, [str(JSON_PROJECT / "app.py"), *args], json_data)
        actual = run(sandbox(dist_dir, [f"{MOUNT}/app", *args], json_data), ROOT)
        assert actual.stdout == expected.stdout, args
        assert actual.returncode == expected.returncode, args
        assert actual.last_stderr_line == expected.last_stderr_line, args


@pytest.mark.e2e
@needs_bwrap
def test_json_module_runs_in_a_clean_environment(build_venv: Path, json_data: Path) -> None:
    """`python -m json` from a distribution: `run_module` in the bundled
    project directory."""
    output = steelix_build(build_venv, "-m", "json", "--root", str(JSON_PROJECT))
    dist_dir = output / "json"
    assert (dist_dir / "src" / "json" / "__main__.py").is_file()
    for args in [["--indent", "1", "sample.json"], ["--no-such-option"]]:
        # CPython in the project directory, where `json` is the project's.
        expected = python_run(build_venv, ["-m", "json", *args], JSON_PROJECT)
        actual = run(sandbox(dist_dir, [f"{MOUNT}/json", *args], json_data), ROOT)
        assert actual.stdout == expected.stdout, args
        assert actual.returncode == expected.returncode, args
        assert actual.last_stderr_line == expected.last_stderr_line, args


@pytest.mark.e2e
@needs_bwrap
def test_package_data_and_bundled_interpreter(
    build_venv: Path, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch
) -> None:
    """Data files of a package through `importlib.resources` and
    `pkgutil.get_data`, and `python -m pkg.cli` run by the bundled
    interpreter in the project directory, without the Python sources: the
    interpreter imports the transpiled modules where CPython would find
    their sources."""
    capsys.readouterr()
    output = steelix_build(build_venv, str(PKGAPP), "--no-sources")
    warnings = capsys.readouterr().err
    assert "steelix: warning: not in the distribution: dyn.py in " in warnings
    dist_dir = output / "app"
    assert not (dist_dir / "src" / "pkg" / "cli.py").exists()
    assert (dist_dir / "src" / "pkg" / "templates" / "greeting.txt").is_file()
    # The child `python -m pkg.cli` does not write `__pycache__` either.
    monkeypatch.setenv("PYTHONDONTWRITEBYTECODE", "1")
    expected = python_run(build_venv, [str(PKGAPP)], ROOT)
    assert expected.returncode == 0, expected.stderr
    actual = run(sandbox(dist_dir, [f"{MOUNT}/app"]), ROOT)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)
    # Elsewhere the project is not on `sys.path`, as for CPython.
    python = f"{MOUNT}/python/bin/python"
    outside = run(sandbox(dist_dir, [python, "-c", "import pkg"]), ROOT)
    assert outside.last_stderr_line == "ModuleNotFoundError: No module named 'pkg'"


@pytest.mark.e2e
@needs_bwrap
@pytest.mark.parametrize("method", ["spawn", "forkserver"])
def test_multiprocessing_from_a_distribution(method: str, build_venv: Path, mp_dist: Path) -> None:
    """The children are the bundled interpreter, running the transpiled
    `__main__` and the functions of the project's package `mptools`, whose
    transpiled modules have no sources in the distribution."""
    expected = python_run(build_venv, [str(MPAPP), method], ROOT)
    assert expected.returncode == 0, expected.stderr
    actual = run(sandbox(mp_dist, [f"{MOUNT}/mpapp", method]), ROOT)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)


@pytest.fixture(scope="module")
def mp_dist(build_venv: Path) -> Path:
    output = steelix_build(build_venv, str(MPAPP), "--no-sources")
    dist_dir = E2E_DIR / "dist-mpapp"
    if dist_dir.exists():
        shutil.rmtree(dist_dir)
    (output / "mpapp").rename(dist_dir)
    assert not (dist_dir / "src" / "mpapp.py").exists()
    # Only the submodule the program does not import is shipped.
    shipped = [p.relative_to(dist_dir / "src").as_posix() for p in (dist_dir / "src").rglob("*.py")]
    assert shipped == ["mptools/cli.py"]
    return dist_dir


@pytest.mark.e2e
@needs_bwrap
@pytest.mark.parametrize("method", ["spawn", "forkserver"])
def test_multiprocessing_from_a_module_distribution(method: str, build_venv: Path) -> None:
    """`python -m mptools.cli` without sources: the children run the
    transpiled module as `__mp_main__` and import its sibling submodule,
    whose package directory is not in the distribution."""
    output = steelix_build(build_venv, "-m", "mptools.cli", "--root", str(MP_DIR), "--no-sources")
    dist_dir = output / "cli"
    assert not (dist_dir / "src" / "mptools").exists()
    expected = python_run(build_venv, ["-m", "mptools.cli", method], MP_DIR)
    assert expected.returncode == 0, expected.stderr
    actual = run(sandbox(dist_dir, [f"{MOUNT}/cli", method]), ROOT)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)


@pytest.mark.e2e
@pytest.mark.parametrize("method", ["spawn", "forkserver"])
def test_multiprocessing_in_development(method: str, tmp_path: Path) -> None:
    """A development build (`cargo build` in the generated crate) also
    starts itself for the children: it works after the source is gone."""
    project = tmp_path / "project"
    project.mkdir()
    shutil.copyfile(MPAPP, project / "mpapp.py")
    shutil.copytree(
        MP_DIR / "mptools", project / "mptools", ignore=shutil.ignore_patterns("__pycache__")
    )
    root = E2E_DIR / f"mp-dev-{method}"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    options = CrateOptions(mode=Mode.BIN, runtime_path=RUNTIME, standalone=False)
    emit_crate(parse_file(project / "mpapp.py"), root / "mpapp", options)
    manifest = f'[workspace]\nresolver = "3"\nmembers = ["mpapp"]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest)
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    result = Workspace(root, ["mpapp"]).cargo("build", "--workspace")
    assert result.returncode == 0, result.stderr
    (project / "mpapp.py").unlink()
    expected = python_run(PYTHON, [str(MPAPP), method], tmp_path)
    binary = Path(cargo_env(extension=False)["CARGO_TARGET_DIR"]) / "debug" / "mpapp"
    actual = run([str(binary), method], tmp_path)
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)
    if method == "spawn":
        # A child command line without the parent's marker is the program's.
        expected = python_run(PYTHON, [str(MPAPP), *INJECTION], tmp_path)
        actual = run([str(binary), *INJECTION], tmp_path)
        assert "INJECTED" not in actual.stdout.splitlines()
        assert actual.returncode == expected.returncode == 1
        assert actual.last_stderr_line == expected.last_stderr_line


@pytest.mark.cargo
def test_runtime_unit_tests() -> None:
    """The runtime's Rust unit tests (recognizing the command lines of
    `multiprocessing` children)."""
    result = subprocess.run(
        ["cargo", "test", "-p", "steelix-runtime", "--lib"],
        cwd=ROOT,
        env={**os.environ, "PYO3_PYTHON": str(PYTHON)},
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "multiprocessing::tests::recognizes_multiprocessing_children ... ok" in result.stdout
