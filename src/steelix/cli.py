"""Command line interface."""

import argparse
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
        "--runtime-path",
        type=Path,
        default=None,
        help="path of the steelix-runtime crate (default: the one in this source checkout)",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "transpile":
        return transpile(args)
    return 0


def transpile(args: argparse.Namespace) -> int:
    from steelix.emit.crate import CrateOptions, Mode, default_runtime_path, emit_crate
    from steelix.frontend.parse import parse_file

    if args.input.is_dir():
        print("steelix: packages are not supported yet, pass a single .py file", file=sys.stderr)
        return 2
    runtime = args.runtime_path or default_runtime_path()
    options = CrateOptions(
        mode=Mode(args.mode),
        runtime_path=runtime.resolve() if runtime else None,
    )
    try:
        emit_crate(parse_file(args.input), args.output, options)
    except UnsupportedError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
    except SyntaxError as err:
        print(f"steelix: {err}", file=sys.stderr)
        return 1
    return 0
