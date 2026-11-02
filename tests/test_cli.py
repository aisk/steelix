from steelix.cli import build_parser


def test_parser_accepts_transpile() -> None:
    args = build_parser().parse_args(["transpile", "foo.py", "-o", "out"])
    assert args.command == "transpile"
    assert args.mode == "ext"
