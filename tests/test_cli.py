import json
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


def test_traceback_lines_flag(tmp_path: Path) -> None:
    source = tmp_path / "app.py"
    source.write_text("def f(x):\n    y = x + 1\n    return y * 2\n\nprint(f(1))\n")
    assert build_parser().parse_args(["transpile", "a.py", "-o", "o"]).traceback_lines is False
    assert main(["transpile", str(source), "-o", str(tmp_path / "plain"), "-q"]) == 0
    plain = (tmp_path / "plain" / "src" / "app.rs").read_text()
    assert "frame.line(" not in plain
    args = ["transpile", str(source), "-o", str(tmp_path / "lines"), "-q", "--traceback-lines"]
    assert main(args) == 0
    lines = (tmp_path / "lines" / "src" / "app.rs").read_text()
    assert "frame.line(2);" in lines
    assert "frame.line(5);" in lines


def test_transpile_ext_mode(tmp_path: Path) -> None:
    source = tmp_path / "lib_mod.py"
    source.write_text("x = 1\n")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "--mode", "ext"]) == 0
    assert 'crate-type = ["cdylib"]' in (out / "Cargo.toml").read_text()
    assert '#[pymodule(name = "lib_mod")]' in (out / "src" / "lib.rs").read_text()


def test_no_fallback_reports_unsupported(tmp_path: Path, capsys) -> None:
    source = tmp_path / "bad.py"
    source.write_text("async def f():\n    pass\n")
    args = ["transpile", str(source), "-o", str(tmp_path / "out"), "--no-fallback"]
    assert main(args) == 1
    assert "bad.py:1:1: unsupported: async function" in capsys.readouterr().err


def test_mixed_mode_keeps_unsupported_code_and_reports_coverage(tmp_path: Path, capsys) -> None:
    source = tmp_path / "app.py"
    source.write_text(
        "def f(x):\n"
        "    return x\n"
        "\n"
        "\n"
        "class A:\n"
        "    def m(self):\n"
        "        yield self\n"
        "\n"
        "\n"
        "def g(xs):\n"
        "    return (x for x in xs for y in x)\n"
        "\n"
        "\n"
        "print(f(1))\n"
    )
    report = tmp_path / "report.json"
    args = ["transpile", str(source), "-o", str(tmp_path / "out"), "--report", str(report)]
    assert main([*args, "-v"]) == 0
    err = capsys.readouterr().err
    assert "app.py: native 1/3 (33%) functions, 2/2 (100%) module statements" in err
    assert "      app.py:11:12" in err  # the generator expression, with -v
    data = json.loads(report.read_text())
    assert data["functions"] == {"native": 1, "total": 3}
    features = {f["feature"]: f for f in data["features"]}
    assert features["yield"] == {
        "feature": "yield",
        "occurrences": 1,
        "functions": 1,
        "only": 1,
        "statements": 0,
        "locations": ["app.py:7:9"],
    }
    assert features["generator expression with several for clauses"]["only"] == 1
    assert [(p["kind"], p["name"]) for p in data["python"]] == [
        ("function", "A.m"),
        ("function", "g"),
    ]
    module_rs = (tmp_path / "out" / "src" / "app.rs").read_text()
    assert "pub fn f<'py>(" in module_rs
    # The class is native, its method `m` is kept as Python.
    assert 'rt::class!(m, "A"(), a::body, line = 5)' in module_rs
    assert "line = 6," in module_rs
    assert "    def m(self):\n" in module_rs


def test_coverage_merges_modules(tmp_path: Path, capsys) -> None:
    (tmp_path / "a.py").write_text("def f():\n    yield 1\n")
    (tmp_path / "b.py").write_text("def g():\n    yield 2\n\n\ndef h():\n    return 3\n")
    report = tmp_path / "report.json"
    args = ["coverage", str(tmp_path / "a.py"), str(tmp_path / "b.py"), "--report", str(report)]
    assert main(args) == 0
    out = capsys.readouterr().out
    assert "b.py: native 1/2 (50%) functions" in out
    assert "all 2 modules:" in out
    data = json.loads(report.read_text())
    assert [m["module"] for m in data["modules"]] == ["a.py", "b.py"]
    assert data["features"] == [
        {"feature": "yield", "occurrences": 2, "functions": 2, "only": 2, "statements": 0}
    ]
    assert not (tmp_path / "out").exists()


def test_script_name_that_is_not_an_identifier(tmp_path: Path) -> None:
    source = tmp_path / "my-tool.py"
    source.write_text("print(1)\n")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "-q"]) == 0
    assert "mod my_tool;" in (out / "src" / "main.rs").read_text()
    assert (out / "src" / "my_tool.rs").is_file()
    assert 'name = "my-tool"' in (out / "Cargo.toml").read_text()
