"""Command line interface."""

import argparse
import json
import sys
from pathlib import Path

from steelix.diagnostics import UnsupportedError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="steelix", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    transpile = sub.add_parser("transpile", help="transpile Python source into a Rust crate")
    transpile.add_argument("input", type=Path, help="Python file")
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
    from steelix.coverage import measure
    from steelix.emit.crate import CrateOptions, Mode, default_runtime_path, emit_crate
    from steelix.frontend.parse import parse_file

    if args.input.is_dir():
        print("steelix: packages are not supported yet, pass a single .py file", file=sys.stderr)
        return 2
    runtime = args.runtime_path or default_runtime_path()
    options = CrateOptions(
        mode=Mode(args.mode),
        runtime_path=runtime.resolve() if runtime else None,
        fallback=args.fallback,
    )
    try:
        parsed = parse_file(args.input)
        lowered = emit_crate(parsed, args.output, options)
    except UnsupportedError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
    except SyntaxError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
    coverage = measure(parsed.tree, lowered.plan, parsed.filename)
    if not args.quiet:
        print(coverage.summary(verbose=args.verbose), file=sys.stderr)
    if args.report is not None:
        args.report.write_text(json.dumps(coverage.to_json(), indent=2) + "\n")
    return 0
