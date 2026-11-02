from pathlib import Path

from steelix.cli import build_parser, main


def test_parser_defaults_to_bin_mode() -> None:
    args = build_parser().parse_args(["transpile", "foo.py", "-o", "out"])
    assert args.command == "transpile"
    assert args.mode == "bin"


def test_transpile_writes_bin_crate(tmp_path: Path) -> None:
    source = tmp_path / "app.py"
    source.write_text("print(1)\n")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out)]) == 0
    assert sorted(p.relative_to(out).as_posix() for p in out.rglob("*") if p.is_file()) == [
        "Cargo.toml",
        "build.rs",
        "rustfmt.toml",
        "src/app.rs",
        "src/main.rs",
    ]
    assert "steelix_runtime::embed::run_main" in (out / "src" / "main.rs").read_text()


def test_transpile_ext_mode(tmp_path: Path) -> None:
    source = tmp_path / "lib_mod.py"
    source.write_text("x = 1\n")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "--mode", "ext"]) == 0
    assert 'crate-type = ["cdylib"]' in (out / "Cargo.toml").read_text()
    assert '#[pymodule(name = "lib_mod")]' in (out / "src" / "lib.rs").read_text()


def test_transpile_reports_unsupported(tmp_path: Path, capsys) -> None:
    source = tmp_path / "bad.py"
    source.write_text("import os\n")
    assert main(["transpile", str(source), "-o", str(tmp_path / "out")]) == 1
    assert "bad.py:1:1: unsupported: import" in capsys.readouterr().err
