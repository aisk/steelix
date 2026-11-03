"""Command line interface."""

import argparse
import json
import os
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


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "transpile":
        return transpile(args)
    if args.command == "build":
        return build(args)
    if args.command == "coverage":
        return coverage(args)
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
        args.report.write_text(json.dumps(data, indent=2) + "\n")
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
    from steelix.project import ProjectError, find_project

    if (args.input is None) == (args.module is None):
        raise CommandError("give either an entry script or -m MODULE", 2)
    if args.input is not None and args.input.is_dir():
        raise CommandError(
            "pass the entry script, or -m PACKAGE to run a package like `python -m`", 2
        )
    runtime = args.runtime_path or default_runtime_path()
    options = CrateOptions(
        mode=Mode(mode),
        runtime_path=runtime.resolve() if runtime else None,
        fallback=args.fallback,
        traceback_lines=args.traceback_lines,
    )
    try:
        project = find_project(
            args.input,
            module=args.module,
            root=args.root,
            include=args.include,
            exclude=args.exclude,
            run_main=options.mode is Mode.BIN,
        )
        crate = output if output is not None else args.build_dir / project_name(project)
        lowered = emit_project(project, crate, options)
    except ProjectError as err:
        raise CommandError(str(err), 2) from err
    except (UnsupportedError, SyntaxError, UnicodeDecodeError, OSError) as err:
        raise CommandError(str(err)) from err
    return project, crate, lowered


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
        args.report.write_text(json.dumps(data, indent=2) + "\n")
    return 0


def build(args: argparse.Namespace) -> int:
    """`steelix build`: transpile, `cargo build --release` for distribution,
    and assemble the program's directory (design §11)."""
    from steelix import dist
    from steelix.emit.crate import default_runtime_path, project_name

    try:
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


def dist_rustflags(env: dict[str, str], runtime: Path | None) -> list[str]:
    """The user's rustflags, and remappings so that the source locations in
    panic messages do not show the build machine's directories."""
    if "CARGO_ENCODED_RUSTFLAGS" in env:
        flags = [f for f in env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f") if f]
    else:
        flags = env.get("RUSTFLAGS", "").split()
    cargo_home = Path(env.get("CARGO_HOME") or Path.home() / ".cargo")
    remaps = [(cargo_home / "registry" / "src", "cargo")]
    if runtime is not None:
        remaps.append((runtime.parent, "steelix"))
    return flags + [f"--remap-path-prefix={source}={target}" for source, target in remaps]


def cargo_build_dist(
    crate: Path, package: str, python: Path, runtime: Path | None, *, quiet: bool
) -> Path:
    """Builds the crate in release mode for distribution (`STEELIX_DIST`)
    with `python`, and returns the executable."""
    lock = crate / "Cargo.lock"
    if runtime is not None and not lock.exists():
        # Pin the dependencies to the versions the runtime is tested with
        # (and resolve them offline).
        checkout_lock = runtime.parents[1] / "Cargo.lock"
        if checkout_lock.is_file():
            shutil.copyfile(checkout_lock, lock)
    env = dict(os.environ)
    env["PYO3_PYTHON"] = str(python)
    env["STEELIX_DIST"] = "1"
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(dist_rustflags(env, runtime))
    command = ["cargo", "build", "--release", "--message-format=json-render-diagnostics"]
    proc = subprocess.run(
        command,
        cwd=crate,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL if quiet else None,
        text=True,
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
