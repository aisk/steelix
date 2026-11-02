"""Command line interface."""

import argparse
from pathlib import Path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="steelix", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    transpile = sub.add_parser("transpile", help="transpile Python source into a Rust crate")
    transpile.add_argument("input", type=Path, help="Python file or package directory")
    transpile.add_argument(
        "-o", "--output", type=Path, required=True, help="output crate directory"
    )
    transpile.add_argument(
        "--mode",
        choices=["ext", "bin"],
        default="ext",
        help="ext: PyO3 extension module, bin: executable embedding libpython",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "transpile":
        # TODO(M1): wire up frontend -> lower -> rustast -> emit.
        print("steelix: transpile is not implemented yet")
        return 1
    return 0
