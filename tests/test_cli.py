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
    main_rs = (out / "src" / "main.rs").read_text()
    assert "steelix_runtime::embed::run_main" in main_rs
    # `sys.argv[0]`, `__file__` and `sys.path[0]` as `python app.py` sets them.
    assert f'argv0: "{source}",' in main_rs
    assert f'file: "{source}",' in main_rs
    assert f'dir: "{tmp_path}",' in main_rs


def test_script_path_is_kept_as_given(tmp_path: Path, monkeypatch) -> None:
    (tmp_path / "app.py").write_text("print(1)\n")
    monkeypatch.chdir(tmp_path)
    assert main(["transpile", "app.py", "-o", "out"]) == 0
    main_rs = (tmp_path / "out" / "src" / "main.rs").read_text()
    assert 'argv0: "app.py",' in main_rs
    assert f'file: "{tmp_path / "app.py"}",' in main_rs


def test_transpile_ext_mode(tmp_path: Path) -> None:
    source = tmp_path / "lib_mod.py"
    source.write_text("x = 1\n")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "--mode", "ext"]) == 0
    assert 'crate-type = ["cdylib"]' in (out / "Cargo.toml").read_text()
    assert '#[pymodule(name = "lib_mod")]' in (out / "src" / "lib.rs").read_text()


def test_transpile_reports_unsupported(tmp_path: Path, capsys) -> None:
    source = tmp_path / "bad.py"
    source.write_text("class A:\n    pass\n")
    assert main(["transpile", str(source), "-o", str(tmp_path / "out")]) == 1
    assert "bad.py:1:1: unsupported: class definition" in capsys.readouterr().err
