"""Command line interface."""

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

from steelix.diagnostics import UnsupportedError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="steelix", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    transpile = sub.add_parser(
        "transpile",
        help="transpile a Python program (the script and its project's modules) into a Rust crate",
    )
    add_program_arguments(transpile)
    transpile.add_argument(
        "-o", "--output", type=Path, required=True, help="output crate directory"
    )
    transpile.add_argument(
        "--mode",
        choices=["bin", "ext"],
        default="bin",
        help="bin: executable embedding libpython (default), ext: PyO3 extension module",
    )
    transpile.add_argument(
        "--report",
        type=Path,
        default=None,
        metavar="FILE",
        help="write the coverage report as JSON to FILE",
    )
    transpile.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="list the location of every unsupported construct in the report",
    )
    transpile.add_argument(
        "-q",
        "--quiet",
        action="store_true",
        help="do not print the coverage report",
    )

    build = sub.add_parser(
        "build",
        help="transpile and build a program into a directory that runs without Python "
        "installed (the executable, the Python runtime, site-packages and the sources)",
    )
    add_program_arguments(build)
    build.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("dist"),
        help="put the program into OUTPUT/NAME (default: dist)",
    )
    build.add_argument(
        "--build-dir",
        type=Path,
        default=Path("build"),
        help="keep the generated crate in BUILD_DIR/NAME, for faster rebuilds (default: build)",
    )
    build.add_argument(
        "--python",
        type=Path,
        default=None,
        help="the interpreter (or virtual environment's) to build with and to bundle with "
        "its site-packages (default: $PYO3_PYTHON, or the one running steelix)",
    )
    build.add_argument(
        "--no-sources",
        dest="sources",
        action="store_false",
        help="do not ship the Python files of transpiled modules (tracebacks then show "
        "no source lines)",
    )
    build.add_argument(
        "--full-stdlib",
        action="store_true",
        help="ship the whole standard library, including tkinter, idlelib and ensurepip",
    )
    build.add_argument(
        "--add-data",
        action="append",
        default=[],
        metavar="PATH",
        help="also copy this file or directory of the project; repeatable",
    )
    build.add_argument(
        "--archive", action="store_true", help="also pack the result into OUTPUT/NAME.tar.gz"
    )
    build.add_argument(
        "-q", "--quiet", action="store_true", help="only print errors and the result"
    )

    test = sub.add_parser(
        "test",
        help="build a transpiled program and run the project's pytest suite with it as the "
        "Python interpreter, so that the tests import the native modules",
        description="Build the crate (`cargo build`, with PYO3_PYTHON set) and run `python -m "
        "pytest ARGS` in the current directory, with the program as `python`: every import "
        "of a transpiled module gets its native code. The run fails if one of them was "
        "imported from its Python source. Pytest must be installed in the interpreter's "
        "environment.",
    )
    test.add_argument("crate", type=Path, help="the crate `steelix transpile` wrote")
    test.add_argument(
        "--python",
        type=Path,
        default=None,
        help="the interpreter (or virtual environment's) to build with; it provides "
        "pytest and the project's dependencies (default: $PYO3_PYTHON, or the one "
        "running steelix)",
    )
    test.add_argument("--release", action="store_true", help="build and test the release profile")
    test.add_argument(
        "pytest_args",
        nargs=argparse.REMAINDER,
        metavar="...",
        help="arguments for pytest (after `--` if they could be taken for steelix's)",
    )

    attrs = sub.add_parser(
        "attrs",
        help="list who reads and writes the attributes of the project's classes, or draft "
        "a #[pyclass] for one of them",
        description="A static index of the attributes of every class in the program's "
        "modules: which functions read, write, augment or delete each instance attribute "
        "(file:line), accesses through receivers of unknown class whose name matches, and "
        "dynamic access (getattr/setattr with computed names, vars(), __dict__). Use it "
        "before rewriting a class that several modules share (design §15).",
    )
    add_discovery_arguments(attrs)
    attrs.add_argument(
        "--class",
        dest="classes",
        action="append",
        default=[],
        metavar="NAME",
        help="only report this class (`Point`, `shapes.Point` or `pkg.shapes.Point`); repeatable",
    )
    attrs.add_argument("--json", action="store_true", help="print the index as JSON")
    attrs.add_argument(
        "--pyclass",
        metavar="NAME",
        default=None,
        help="print a draft #[pyclass] for this class instead, with what blocks or "
        "complicates the upgrade (advice, never part of the build)",
    )
    attrs.add_argument(
        "-o", "--output", type=Path, default=None, help="write to this file instead of stdout"
    )

    coverage = sub.add_parser(
        "coverage",
        help="report what is native and what stays Python, without writing crates",
    )
    coverage.add_argument("inputs", type=Path, nargs="+", help="Python files")
    coverage.add_argument(
        "--report", type=Path, default=None, metavar="FILE", help="write the reports as JSON"
    )
    coverage.add_argument(
        "-v", "--verbose", action="store_true", help="list every unsupported construct"
    )
    return parser


def add_program_arguments(parser: argparse.ArgumentParser) -> None:
    """The arguments that select and transpile a program, shared by `transpile`
    and `build`."""
    add_discovery_arguments(parser)
    parser.add_argument(
        "--no-fallback",
        dest="fallback",
        action="store_false",
        help="fail on unsupported constructs instead of keeping them as Python source",
    )
    parser.add_argument(
        "--traceback-lines",
        action="store_true",
        help="mark the line of every statement that may raise, so tracebacks of native "
        "frames show statement lines instead of the def line (more verbose Rust)",
    )
    parser.add_argument(
        "--runtime-path",
        type=Path,
        default=None,
        help="path of the steelix-runtime crate (default: the one in this source checkout)",
    )


def add_discovery_arguments(parser: argparse.ArgumentParser) -> None:
    """The arguments that select a program and the project modules it
    imports, shared by `transpile`, `build` and `attrs`."""
    parser.add_argument(
        "input",
        type=Path,
        nargs="?",
        help="the entry script; the modules it imports from its directory are transpiled too",
    )
    parser.add_argument(
        "-m",
        dest="module",
        metavar="MODULE",
        default=None,
        help="run the module (a package runs its __main__) like `python -m MODULE`, "
        "from the --root directory; in ext mode, the module to build",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help="project directory for -m (default: the current directory)",
    )
    parser.add_argument(
        "--include",
        action="append",
        default=[],
        metavar="MODULE|PATH",
        help="also transpile this project module, file or directory (for imports the "
        "static analysis can not see); repeatable",
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        metavar="MODULE",
        help="leave this module (and its submodules) as Python source; repeatable",
    )


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "transpile":
        return transpile(args)
    if args.command == "build":
        return build(args)
    if args.command == "test":
        return test(args)
    if args.command == "coverage":
        return coverage(args)
    if args.command == "attrs":
        return attrs(args)
    return 0


def coverage(args: argparse.Namespace) -> int:
    """Coverage of several modules, with the features ranked over all of them."""
    from steelix.coverage import measure, merge
    from steelix.frontend.parse import parse_file
    from steelix.lower.module import lower_module

    reports = []
    for path in args.inputs:
        try:
            parsed = parse_file(path)
            plan = lower_module(parsed).plan
        except (UnsupportedError, SyntaxError) as err:
            print(f"steelix: {err}", file=sys.stderr)
            return 1
        report = measure(parsed.tree, plan, parsed.filename)
        print(report.summary(verbose=args.verbose))
        reports.append(report.to_json())
    features = merge(reports)
    if len(reports) > 1:
        width = max((len(f["feature"]) for f in features), default=0)
        print(f"all {len(reports)} modules:")
        print(f"  {'kept as Python by':<{width}}  functions  only")
        for f in features:
            print(f"  {f['feature']:<{width}}  {f['functions']:>9}  {f['only']:>4}")
    if args.report is not None:
        data = {"modules": reports, "features": features}
        args.report.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return 0


class CommandError(Exception):
    """An error reported as `steelix: message`, exiting with `code`."""

    def __init__(self, message: str, code: int = 1) -> None:
        super().__init__(message)
        self.code = code


def emit_program(args: argparse.Namespace, mode: str, output: Path | None):
    """Finds the program's project and writes its crate to `output` (for
    `None`, `BUILD_DIR/NAME`). Returns the project, the crate directory and
    the lowered modules."""
    from steelix.emit.crate import (
        CrateOptions,
        Mode,
        default_runtime_path,
        emit_project,
        project_name,
    )
    from steelix.project import ProjectError

    runtime = args.runtime_path or default_runtime_path()
    options = CrateOptions(
        mode=Mode(mode),
        runtime_path=runtime.resolve() if runtime else None,
        fallback=args.fallback,
        traceback_lines=args.traceback_lines,
    )
    project = discover_project(args, run_main=options.mode is Mode.BIN)
    try:
        crate = output if output is not None else args.build_dir / project_name(project)
        lowered = emit_project(project, crate, options)
    except ProjectError as err:
        raise CommandError(str(err), 2) from err
    except (UnsupportedError, SyntaxError, UnicodeDecodeError, OSError) as err:
        raise CommandError(str(err)) from err
    return project, crate, lowered


def discover_project(args: argparse.Namespace, run_main: bool = True):
    """The project selected by the discovery arguments."""
    from steelix.project import ProjectError, find_project

    if (args.input is None) == (args.module is None):
        raise CommandError("give either an entry script or -m MODULE", 2)
    if args.input is not None and args.input.is_dir():
        raise CommandError(
            "pass the entry script, or -m PACKAGE to run a package like `python -m`", 2
        )
    try:
        return find_project(
            args.input,
            module=args.module,
            root=args.root,
            include=args.include,
            exclude=args.exclude,
            run_main=run_main,
        )
    except ProjectError as err:
        raise CommandError(str(err), 2) from err
    except (SyntaxError, UnicodeDecodeError, OSError) as err:
        raise CommandError(str(err)) from err


def attrs(args: argparse.Namespace) -> int:
    """`steelix attrs`: the attribute index, or a `#[pyclass]` draft (design §15)."""
    from steelix.attrs import build_index, format_text, select, to_json
    from steelix.pyclass import draft

    try:
        project = discover_project(args)
        index = build_index(project)
        classes = select(index, args.classes) if args.classes else None
        if args.pyclass is not None:
            (cls,) = select(index, [args.pyclass])
            words = ["steelix", "attrs", *selection_arguments(args), "--pyclass", cls.full_name]
            text = draft(index, cls, shlex.join(words))
        elif args.json:
            text = json.dumps(to_json(index, classes), indent=2) + "\n"
        else:
            text = format_text(index, classes)
    except LookupError as err:
        print(f"steelix: {err.args[0]}", file=sys.stderr)
        return 2
    except CommandError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return err.code
    if args.output is not None:
        args.output.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    return 0


def selection_arguments(args: argparse.Namespace) -> list[str]:
    """The discovery arguments as given on the command line."""
    words = [str(args.input)] if args.input is not None else ["-m", args.module]
    if args.root is not None:
        words += ["--root", str(args.root)]
    for item in args.include:
        words += ["--include", item]
    for item in args.exclude:
        words += ["--exclude", item]
    return words


def print_report(args: argparse.Namespace, project, lowered: list) -> list:
    """Prints the coverage report of the transpiled program (unless
    `--quiet`) and returns the reports of its modules."""
    from steelix.coverage import measure, project_summary

    reports = [
        measure(project.parsed[source.relpath].tree, module.plan, source.relpath)
        for source, module in zip(project.sources(), lowered, strict=True)
    ]
    if not args.quiet:
        if len(reports) > 1:
            names = ", ".join(project.modules)
            print(f"project {project.root}: transpiled modules {names}", file=sys.stderr)
        for note in project.notes:
            print(f"  {note}", file=sys.stderr)
        for report in reports:
            print(report.summary(verbose=getattr(args, "verbose", False)), file=sys.stderr)
        if len(reports) > 1:
            print(project_summary(reports), file=sys.stderr)
    return reports


def transpile(args: argparse.Namespace) -> int:
    try:
        project, _, lowered = emit_program(args, args.mode, args.output)
    except CommandError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return err.code
    reports = print_report(args, project, lowered)
    if args.report is not None:
        if len(reports) == 1:
            data = reports[0].to_json()
        else:
            data = project_report(reports)
        args.report.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return 0


def build(args: argparse.Namespace) -> int:
    """`steelix build`: transpile, `cargo build --release` for distribution,
    and assemble the program's directory (design §11)."""
    from steelix import dist
    from steelix.emit.crate import default_runtime_path, project_name

    try:
        if sys.platform == "win32":
            raise dist.DistError(
                "building a distribution is not supported on Windows yet; "
                "`steelix transpile` and `cargo build` make a program that runs on this machine"
            )
        python = args.python or Path(os.environ.get("PYO3_PYTHON") or sys.executable)
        install = dist.inspect_python(Path(os.path.abspath(python)))
        project, crate, lowered = emit_program(args, "bin", None)
        print_report(args, project, lowered)
        name = project_name(project)
        runtime = args.runtime_path or default_runtime_path()
        binary = cargo_build_dist(crate, name, install.executable, runtime, quiet=args.quiet)
        target = args.output / name
        warnings = dist.assemble(
            binary,
            name,
            install,
            project,
            target,
            dist.DistOptions(
                sources=args.sources, full_stdlib=args.full_stdlib, data=tuple(args.add_data)
            ),
        )
    except dist.DistError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
    except CommandError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return err.code
    for warning in warnings:
        print(f"steelix: warning: {warning}", file=sys.stderr)
    size, files = dist.tree_size(target)
    print(f"{target / name}  ({dist.format_size(size)} in {files} files under {target})")
    if args.archive:
        packed = dist.archive(target)
        print(f"{packed}  ({dist.format_size(packed.stat().st_size)})")
    return 0


def test(args: argparse.Namespace) -> int:
    """`steelix test`: run the project's tests with the program in the
    interpreter role (design §15)."""
    try:
        python = args.python or Path(os.environ.get("PYO3_PYTHON") or sys.executable)
        program = cargo_build_program(args.crate, Path(os.path.abspath(python)), args.release)
    except CommandError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return err.code
    command, env = pytest_command(program, args.pytest_args)
    return subprocess.run(command, env=env, check=False).returncode


def pytest_command(program: Path, pytest_args: list[str]) -> tuple[list[str], dict[str, str]]:
    """The command line and environment that run pytest with `program` as
    the Python interpreter: it is started through a link named `python`,
    with `steelix_native` (`pytest_native.py`) as a plugin.

    On Windows, where a symbolic link needs a privilege, `python.exe` is a
    hard link to the program (or a copy), next to links to the DLLs the
    build copied next to the program (`python314.dll`), which Windows looks
    for in the directory of the executable it starts."""
    home = program.parent / "steelix-test"
    bin_dir = home / "bin"
    plugins = home / "plugins"
    bin_dir.mkdir(parents=True, exist_ok=True)
    plugins.mkdir(exist_ok=True)
    if sys.platform == "win32":
        link = bin_dir / "python.exe"
        for dll in program.parent.glob("python*.dll"):
            _hard_link(dll, bin_dir / dll.name)
        _hard_link(program, link)
    else:
        link = bin_dir / "python"
        if link.is_symlink() or link.exists():
            link.unlink()
        link.symlink_to(program)
    shutil.copyfile(Path(__file__).with_name("pytest_native.py"), plugins / "steelix_native.py")
    env = dict(os.environ)
    env["STEELIX_NATIVE_IMPORTS"] = "1"
    env["PYTHONPATH"] = os.pathsep.join([str(plugins), *filter(None, [env.get("PYTHONPATH")])])
    return [str(link), "-m", "pytest", "-p", "steelix_native", *pytest_args], env


def _hard_link(source: Path, link: Path) -> None:
    """Makes `link` a hard link to `source`, or a copy where the file system
    has none."""
    if link.exists():
        if link.samefile(source):
            return
        link.unlink()
    try:
        os.link(source, link)
    except OSError:
        shutil.copy2(source, link)


def cargo_build_program(crate: Path, python: Path, release: bool) -> Path:
    """Builds the executable of a crate that `steelix transpile` wrote (a
    development build, with `python`) and returns it."""
    import tomllib

    try:
        manifest = tomllib.loads((crate / "Cargo.toml").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise CommandError(
            f"{crate} is not a crate written by `steelix transpile`: {err}", 2
        ) from err
    bins = manifest.get("bin") or []
    if not bins:
        raise CommandError(
            f"{crate} has no executable: transpile in bin mode (the default), not --mode ext",
            2,
        )
    runtime = manifest.get("dependencies", {}).get("steelix-runtime")
    if isinstance(runtime, dict) and "path" in runtime:
        pin_dependencies(crate, Path(runtime["path"]))
    env = dict(os.environ)
    env["PYO3_PYTHON"] = str(python)
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    env.pop("STEELIX_DIST", None)
    command = ["cargo", "build", "--message-format=json-render-diagnostics"]
    if release:
        command.append("--release")
    return cargo_executable(command, crate, env, bins[0]["name"], quiet=False)


def dist_rustflags(env: dict[str, str], runtime: Path | None) -> tuple[dict[str, str], list[str]]:
    """Remappings so that the source locations in panic messages do not show
    the build machine's directories, added to the user's rustflags: the
    environment variables and the cargo arguments that do it.

    Rustflags from the environment (`RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`)
    replace those of cargo's configuration, so the remappings are appended
    to them. Otherwise they go to `build.rustflags` with `--config`, which
    cargo joins with the `build.rustflags` of the user's configuration. A
    `target.<triple>.rustflags` in the configuration takes precedence over
    both, and the paths are then not remapped (which only changes panic
    messages)."""
    cargo_home = Path(env.get("CARGO_HOME") or Path.home() / ".cargo")
    remaps = [(cargo_home / "registry" / "src", "cargo")]
    if runtime is not None:
        remaps.append((runtime.parent, "steelix"))
    flags = [f"--remap-path-prefix={source}={target}" for source, target in remaps]
    if "CARGO_ENCODED_RUSTFLAGS" in env:
        user = [f for f in env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f") if f]
    elif "RUSTFLAGS" in env:
        user = env["RUSTFLAGS"].split()
    else:
        return {}, ["--config", f"build.rustflags={json.dumps(flags)}"]
    return {"CARGO_ENCODED_RUSTFLAGS": "\x1f".join(user + flags)}, []


def pin_dependencies(crate: Path, runtime: Path | None) -> None:
    """Pins the dependencies of a new crate to the versions the runtime is
    tested with (and resolves them offline), for a runtime in a source
    checkout of steelix."""
    lock = crate / "Cargo.lock"
    if runtime is not None and not lock.exists():
        checkout_lock = runtime.parents[1] / "Cargo.lock"
        if checkout_lock.is_file():
            shutil.copyfile(checkout_lock, lock)


def cargo_build_dist(
    crate: Path, package: str, python: Path, runtime: Path | None, *, quiet: bool
) -> Path:
    """Builds the crate in release mode for distribution (`STEELIX_DIST`)
    with `python`, and returns the executable."""
    pin_dependencies(crate, runtime)
    env = dict(os.environ)
    env["PYO3_PYTHON"] = str(python)
    env["STEELIX_DIST"] = "1"
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    rustflags_env, rustflags_args = dist_rustflags(env, runtime)
    env.update(rustflags_env)
    command = ["cargo", "build", "--release", "--message-format=json-render-diagnostics"]
    command += rustflags_args
    return cargo_executable(command, crate, env, package, quiet=quiet)


def cargo_executable(
    command: list[str], crate: Path, env: dict[str, str], package: str, *, quiet: bool
) -> Path:
    """Runs the cargo `command` (with `--message-format=json..`) in the crate
    and returns the executable it built for `package`."""
    proc = subprocess.run(
        command,
        cwd=crate,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL if quiet else None,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if proc.returncode != 0:
        raise CommandError(f"cargo build failed in {crate}")
    for line in proc.stdout.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if (
            message.get("reason") == "compiler-artifact"
            and message.get("target", {}).get("name") == package
            and message.get("executable")
        ):
            return Path(message["executable"])
    raise CommandError(f"cargo built no executable for {package} in {crate}")


def project_report(reports: list) -> dict:
    """The JSON report of a project: each module's, and the totals."""
    from steelix.coverage import merge

    modules = [r.to_json() for r in reports]
    total = {
        key: {
            "native": sum(m[key]["native"] for m in modules),
            "total": sum(m[key]["total"] for m in modules),
        }
        for key in ("functions", "statements", "lines")
    }
    return {"modules": modules, "total": total, "features": merge(modules)}
