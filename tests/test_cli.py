import json
import os
import sys
from pathlib import Path

import pytest

from steelix.cli import build_parser, main, pytest_command
from steelix.rustast.printer import string_literal


def test_parser_defaults_to_bin_mode() -> None:
    args = build_parser().parse_args(["transpile", "foo.py", "-o", "out"])
    assert args.command == "transpile"
    assert args.mode == "bin"


def test_test_parser_passes_pytest_arguments_through() -> None:
    args = build_parser().parse_args(["test", "out", "tests/", "-x", "-k", "vm"])
    assert args.crate == Path("out")
    assert args.pytest_args == ["tests/", "-x", "-k", "vm"]
    args = build_parser().parse_args(["test", "--release", "out", "--", "--release"])
    assert args.release
    # argparse drops the `--`.
    assert args.pytest_args == ["--release"]


def test_test_command_runs_the_program_as_python(tmp_path: Path, monkeypatch) -> None:
    program = tmp_path / "debug" / "app"
    program.parent.mkdir()
    program.write_text("", encoding="utf-8")
    monkeypatch.setenv("PYTHONPATH", "/elsewhere")
    command, env = pytest_command(program, ["tests", "-x"])
    bin_dir = tmp_path / "debug" / "steelix-test" / "bin"
    plugins = tmp_path / "debug" / "steelix-test" / "plugins"
    if sys.platform == "win32":
        link = bin_dir / "python.exe"
        assert link.samefile(program)
    else:
        link = bin_dir / "python"
        assert link.resolve() == program.resolve()
    assert command == [str(link), "-m", "pytest", "-p", "steelix_native", "tests", "-x"]
    assert (plugins / "steelix_native.py").is_file()
    assert env["STEELIX_NATIVE_IMPORTS"] == "1"
    assert env["PYTHONPATH"] == f"{plugins}{os.pathsep}/elsewhere"
    # Again, over the link of the first time.
    assert pytest_command(program, [])[0][-1] == "steelix_native"


def test_test_refuses_an_extension_crate(tmp_path: Path, capsys) -> None:
    source = tmp_path / "app.py"
    source.write_text("print(1)\n", encoding="utf-8")
    out = tmp_path / "out"
    assert main(["transpile", "--mode", "ext", str(source), "-o", str(out), "-q"]) == 0
    assert main(["test", str(out)]) == 2
    assert "has no executable" in capsys.readouterr().err


def test_transpile_writes_bin_crate(tmp_path: Path) -> None:
    source = tmp_path / "app.py"
    source.write_text("print(1)\n", encoding="utf-8")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out)]) == 0
    assert sorted(p.relative_to(out).as_posix() for p in out.rglob("*") if p.is_file()) == [
        "Cargo.toml",
        "build.rs",
        "rustfmt.toml",
        "src/app.rs",
        "src/main.rs",
    ]
    main_rs = (out / "src" / "main.rs").read_text(encoding="utf-8")
    # The script is named relative to its project directory, which the
    # runtime locates: the build script has the directory for development.
    assert 'let script = steelix_runtime::script!("app.py");' in main_rs
    assert "steelix_runtime::embed::run_main(&script, app::exec_module)" in main_rs
    assert str(tmp_path) not in main_rs
    build_rs = " ".join((out / "build.rs").read_text(encoding="utf-8").split())
    assert f"const PROJECT: &str = {string_literal(str(tmp_path))};" in build_rs
    assert "STEELIX_PROJECT={PROJECT}" in build_rs
    assert "$ORIGIN/python/lib" in build_rs


def test_symlinked_script_runs_from_its_target(tmp_path: Path, monkeypatch) -> None:
    (tmp_path / "real").mkdir()
    (tmp_path / "real" / "tool.py").write_text("print(1)\n", encoding="utf-8")
    try:
        (tmp_path / "app.py").symlink_to(tmp_path / "real" / "tool.py")
    except OSError as err:  # Windows without the privilege
        pytest.skip(f"can not create a symbolic link: {err}")
    monkeypatch.chdir(tmp_path)
    assert main(["transpile", "app.py", "-o", "out", "-q"]) == 0
    main_rs = (tmp_path / "out" / "src" / "main.rs").read_text(encoding="utf-8")
    assert 'steelix_runtime::script!("tool.py")' in main_rs
    build_rs = " ".join((tmp_path / "out" / "build.rs").read_text(encoding="utf-8").split())
    assert f"const PROJECT: &str = {string_literal(str(tmp_path / 'real'))};" in build_rs


def test_traceback_lines_flag(tmp_path: Path) -> None:
    source = tmp_path / "app.py"
    source.write_text(
        "def f(x):\n    y = x + 1\n    return y * 2\n\nprint(f(1))\n", encoding="utf-8"
    )
    assert build_parser().parse_args(["transpile", "a.py", "-o", "o"]).traceback_lines is False
    assert main(["transpile", str(source), "-o", str(tmp_path / "plain"), "-q"]) == 0
    plain = (tmp_path / "plain" / "src" / "app.rs").read_text(encoding="utf-8")
    assert "frame.line(" not in plain
    args = ["transpile", str(source), "-o", str(tmp_path / "lines"), "-q", "--traceback-lines"]
    assert main(args) == 0
    lines = (tmp_path / "lines" / "src" / "app.rs").read_text(encoding="utf-8")
    assert "frame.line(2);" in lines
    assert "frame.line(5);" in lines


def test_transpile_ext_mode(tmp_path: Path) -> None:
    source = tmp_path / "lib_mod.py"
    source.write_text("x = 1\n", encoding="utf-8")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "--mode", "ext"]) == 0
    assert 'crate-type = ["cdylib"]' in (out / "Cargo.toml").read_text(encoding="utf-8")
    assert '#[pymodule(name = "lib_mod")]' in (out / "src" / "lib.rs").read_text(encoding="utf-8")
    assert "add_extension_module_link_args()" in (out / "build.rs").read_text(encoding="utf-8")


def test_no_fallback_reports_unsupported(tmp_path: Path, capsys) -> None:
    source = tmp_path / "bad.py"
    source.write_text("async def f():\n    yield 1\n", encoding="utf-8")
    args = ["transpile", str(source), "-o", str(tmp_path / "out"), "--no-fallback"]
    assert main(args) == 1
    assert "bad.py:1:1: unsupported: asynchronous generator" in capsys.readouterr().err


def test_mixed_mode_keeps_unsupported_code_and_reports_coverage(tmp_path: Path, capsys) -> None:
    source = tmp_path / "app.py"
    source.write_text(
        "def f(x):\n"
        "    return x\n"
        "\n"
        "\n"
        "class A:\n"
        "    async def m(self):\n"
        "        yield self\n"
        "\n"
        "\n"
        "def g(xs):\n"
        "    return (x for x in xs for y in x)\n"
        "\n"
        "\n"
        "print(f(1))\n",
        encoding="utf-8",
    )
    report = tmp_path / "report.json"
    args = ["transpile", str(source), "-o", str(tmp_path / "out"), "--report", str(report)]
    assert main([*args, "-v"]) == 0
    err = capsys.readouterr().err
    assert "app.py: native 1/3 (33%) functions, 2/2 (100%) module statements" in err
    assert "      app.py:11:12" in err  # the generator expression, with -v
    data = json.loads(report.read_text(encoding="utf-8"))
    assert data["functions"] == {"native": 1, "total": 3}
    features = {f["feature"]: f for f in data["features"]}
    assert features["asynchronous generator"] == {
        "feature": "asynchronous generator",
        "occurrences": 1,
        "functions": 1,
        "only": 1,
        "statements": 0,
        "locations": ["app.py:6:5"],
    }
    assert features["generator expression with several for clauses"]["only"] == 1
    assert [(p["kind"], p["name"]) for p in data["python"]] == [
        ("function", "A.m"),
        ("function", "g"),
    ]
    module_rs = (tmp_path / "out" / "src" / "app.rs").read_text(encoding="utf-8")
    assert "pub fn f<'py>(" in module_rs
    # The class is native, its method `m` is kept as Python.
    assert 'rt::class!(m, "A"(), a::body, line = 5)' in module_rs
    assert "line = 6," in module_rs
    assert "    async def m(self):\n" in module_rs


def test_coverage_merges_modules(tmp_path: Path, capsys) -> None:
    (tmp_path / "a.py").write_text("async def f():\n    yield 1\n", encoding="utf-8")
    (tmp_path / "b.py").write_text(
        "async def g():\n    yield 2\n\n\ndef h():\n    return 3\n", encoding="utf-8"
    )
    report = tmp_path / "report.json"
    args = ["coverage", str(tmp_path / "a.py"), str(tmp_path / "b.py"), "--report", str(report)]
    assert main(args) == 0
    out = capsys.readouterr().out
    assert "b.py: native 1/2 (50%) functions" in out
    assert "all 2 modules:" in out
    data = json.loads(report.read_text(encoding="utf-8"))
    assert [m["module"] for m in data["modules"]] == ["a.py", "b.py"]
    assert data["features"] == [
        {
            "feature": "asynchronous generator",
            "occurrences": 2,
            "functions": 2,
            "only": 2,
            "statements": 0,
        }
    ]
    assert not (tmp_path / "out").exists()


def test_script_name_that_is_not_an_identifier(tmp_path: Path) -> None:
    source = tmp_path / "my-tool.py"
    source.write_text("print(1)\n", encoding="utf-8")
    out = tmp_path / "out"
    assert main(["transpile", str(source), "-o", str(out), "-q"]) == 0
    assert "mod my_tool;" in (out / "src" / "main.rs").read_text(encoding="utf-8")
    assert (out / "src" / "my_tool.rs").is_file()
    assert 'name = "my-tool"' in (out / "Cargo.toml").read_text(encoding="utf-8")
