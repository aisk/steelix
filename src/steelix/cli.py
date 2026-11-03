"""Command line interface."""

import argparse
import json
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
    transpile.add_argument(
        "input",
        type=Path,
        nargs="?",
        help="the entry script; the modules it imports from its directory are transpiled too",
    )
    transpile.add_argument(
        "-m",
        dest="module",
        metavar="MODULE",
        default=None,
        help="run the module (a package runs its __main__) like `python -m MODULE`, "
        "from the --root directory; in ext mode, the module to build",
    )
    transpile.add_argument(
        "--root",
        type=Path,
        default=None,
        help="project directory for -m (default: the current directory)",
    )
    transpile.add_argument(
        "--include",
        action="append",
        default=[],
        metavar="MODULE|PATH",
        help="also transpile this project module, file or directory (for imports the "
        "static analysis can not see); repeatable",
    )
    transpile.add_argument(
        "--exclude",
        action="append",
        default=[],
        metavar="MODULE",
        help="leave this module (and its submodules) as Python source; repeatable",
    )
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
        "--no-fallback",
        dest="fallback",
        action="store_false",
        help="fail on unsupported constructs instead of keeping them as Python source",
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
    transpile.add_argument(
        "--runtime-path",
        type=Path,
        default=None,
        help="path of the steelix-runtime crate (default: the one in this source checkout)",
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


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "transpile":
        return transpile(args)
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


def transpile(args: argparse.Namespace) -> int:
    from steelix.coverage import measure, project_summary
    from steelix.emit.crate import CrateOptions, Mode, default_runtime_path, emit_project
    from steelix.project import ProjectError, find_project

    if (args.input is None) == (args.module is None):
        print("steelix: give either an entry script or -m MODULE", file=sys.stderr)
        return 2
    if args.input is not None and args.input.is_dir():
        print(
            "steelix: pass the entry script, or -m PACKAGE to run a package like `python -m`",
            file=sys.stderr,
        )
        return 2
    runtime = args.runtime_path or default_runtime_path()
    options = CrateOptions(
        mode=Mode(args.mode),
        runtime_path=runtime.resolve() if runtime else None,
        fallback=args.fallback,
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
        lowered = emit_project(project, args.output, options)
    except ProjectError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 2
    except (UnsupportedError, SyntaxError, UnicodeDecodeError, OSError) as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
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
            print(report.summary(verbose=args.verbose), file=sys.stderr)
        if len(reports) > 1:
            print(project_summary(reports), file=sys.stderr)
    if args.report is not None:
        if len(reports) == 1:
            data = reports[0].to_json()
        else:
            data = project_report(reports)
        args.report.write_text(json.dumps(data, indent=2) + "\n")
    return 0


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
