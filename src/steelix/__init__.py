"""Steelix: transpile Python source into readable Rust code built on PyO3."""

__version__ = "0.3.0"


def main() -> None:
    from steelix.cli import main as cli_main

    raise SystemExit(cli_main())
