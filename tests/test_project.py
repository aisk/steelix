"""Finding the modules of a project and laying them out as Rust modules
(design §7). The programs themselves run in `tests/test_projects.py`."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from steelix.cli import main
from steelix.emit.crate import rust_layout
from steelix.frontend.parse import parse_source
from steelix.lower.module import ModuleRole, lower_module
from steelix.project import ProjectError, cross_module_patches, find_project, resolve_relative


def write(root: Path, files: dict[str, str]) -> None:
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)


def test_discovers_imported_project_modules(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "app.py": (
                "import json\n"
                "import helpers\n"
                "import pkg.sub\n"
                "from pkg import mod, attribute\n"
                "def later():\n"
                "    import lazy\n"
                "importlib.import_module('dynamic')\n"
                "__import__('dunder')\n"
            ),
            "helpers.py": "",
            "lazy.py": "",
            "dynamic.py": "",
            "dunder.py": "",
            "unused.py": "",
            "pkg/__init__.py": "attribute = 1\n",
            "pkg/sub.py": "from . import sibling\nfrom .deeper import x\nfrom .. import beyond\n",
            "pkg/mod.py": "",
            "pkg/sibling.py": "",
            "pkg/deeper/__init__.py": "",
            "pkg/deeper/x.py": "",
        },
    )
    project = find_project(tmp_path / "app.py")
    assert list(project.modules) == [
        "dunder",
        "dynamic",
        "helpers",
        "lazy",
        "pkg",
        "pkg.deeper",
        "pkg.deeper.x",
        "pkg.mod",
        "pkg.sibling",
        "pkg.sub",
    ]
    assert project.modules["pkg"].relpath == "pkg/__init__.py"
    assert project.modules["pkg"].package
    assert project.main.name == "app"
    assert project.root == tmp_path.resolve()


def test_entry_is_a_module_only_when_imported_by_name(tmp_path: Path) -> None:
    write(tmp_path, {"app.py": "import helpers\n", "helpers.py": "import app\n"})
    project = find_project(tmp_path / "app.py")
    assert list(project.modules) == ["app", "helpers"]
    # One source file, one Rust module.
    assert [s.relpath for s in project.sources()] == ["app.py", "helpers.py"]


def test_what_is_left_to_python(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "app.py": (
                "import sys, broken, nspkg.inner, excluded, plain\n"
                "import importlib\n"
                "from plain import thing\n"
            ),
            # A builtin module takes precedence over the project's.
            "sys.py": "",
            "broken.py": "def f(:\n",
            "nspkg/inner.py": "",
            "excluded.py": "",
            "plain/__init__.py": "",
            # The package wins over a module of the same name.
            "plain.py": "",
        },
    )
    project = find_project(tmp_path / "app.py", exclude=["excluded"])
    assert list(project.modules) == ["plain"]
    assert project.modules["plain"].relpath == "plain/__init__.py"
    notes = "\n".join(project.notes)
    assert "broken.py: not transpiled, it does not compile" in notes
    assert "nspkg/: namespace package, not transpiled" in notes
    assert "excluded.py: excluded, imports from its Python source" in notes


def test_include_modules_and_directories(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "app.py": "",
            "plugins/__init__.py": "",
            "plugins/a.py": "",
            "plugins/b.py": "import shared\n",
            "shared.py": "",
            "other.py": "",
            "notapackage/c.py": "",
        },
    )
    project = find_project(
        tmp_path / "app.py",
        include=[str(tmp_path / "plugins"), "other", str(tmp_path / "notapackage")],
    )
    assert list(project.modules) == ["other", "plugins", "plugins.a", "plugins.b", "shared"]
    with pytest.raises(ProjectError, match="no such module"):
        find_project(tmp_path / "app.py", include=["missing"])


def test_run_a_package_like_python_m(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "pkg/__init__.py": "",
            "pkg/__main__.py": "from .cli import run\n",
            "pkg/cli.py": "",
            "nomain/__init__.py": "",
        },
    )
    project = find_project(module="pkg", root=tmp_path)
    assert project.main.name == "pkg.__main__"
    assert project.run_module
    assert list(project.modules) == ["pkg", "pkg.__main__", "pkg.cli"]
    with pytest.raises(ProjectError, match="has no __main__.py"):
        find_project(module="nomain", root=tmp_path)
    # An extension module of a package includes all of it.
    library = find_project(module="nomain", root=tmp_path, run_main=False)
    assert library.main.name == "nomain"


def test_resolve_relative() -> None:
    assert resolve_relative("x", 1, "pkg.sub") == "pkg.sub.x"
    assert resolve_relative(None, 2, "pkg.sub") == "pkg"
    assert resolve_relative("y", 2, "pkg.sub") == "pkg.y"
    assert resolve_relative("y", 3, "pkg.sub") is None
    assert resolve_relative("y", 1, "") is None


def test_rust_layout_names(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "main.py": (
                "import main, pkg.type, pkg.mod, pkg.ok\n"
                "importlib.import_module('my-tool')\n"
                "import my_tool\n"
            ),
            "pkg/__init__.py": "",
            "pkg/type.py": "",
            "pkg/mod.py": "",
            "pkg/ok.py": "",
            "my-tool.py": "",
            "my_tool.py": "",
        },
    )
    project = find_project(tmp_path / "main.py")
    layout = {
        rust.source.name: (rust.rust_path, rust.file) for rust in rust_layout(project).values()
    }
    assert layout == {
        # `main` would be the crate root.
        "main": ("main_", "main_.rs"),
        "my-tool": ("my_tool_", "my_tool_.rs"),
        "my_tool": ("my_tool", "my_tool.rs"),
        "pkg": ("pkg", "pkg/mod.rs"),
        # `pkg/mod.rs` is the package itself.
        "pkg.mod": ("pkg::mod_", "pkg/mod_.rs"),
        "pkg.ok": ("pkg::ok", "pkg/ok.rs"),
        "pkg.type": ("pkg::r#type", "pkg/type.rs"),
    }


def test_cross_module_patches(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "app.py": (
                "import helpers, tools as t\n"
                "from pkg import mod\n"
                "helpers.f = None\n"
                "setattr(t, name, 1)\n"
                "mock.patch('pkg.mod.g')\n"
                "monkeypatch.setattr(mod, 'h', 1)\n"
                "getattr(mod, 'k')\n"
            ),
            "helpers.py": "def f(): pass\ndef other(): pass\n",
            "tools.py": "def f(): pass\n",
            "pkg/__init__.py": "",
            "pkg/mod.py": "def g(): pass\ndef h(): pass\ndef k(): pass\n",
        },
    )
    project = find_project(tmp_path / "app.py")
    assert cross_module_patches(project) == {
        "helpers": {"f"},
        "tools": None,
        "pkg.mod": {"g", "h"},
    }


def test_patched_function_is_called_through_globals() -> None:
    source = "def f():\n    return 1\n\n\ndef g():\n    return f()\n"
    parsed = parse_source(source, "helpers")
    direct = lower_module(parsed, role=ModuleRole(own_names=("helpers",)))
    assert "|_| { f(m) })\n" in _fn_text(direct)
    parsed = parse_source(source, "helpers")
    patched = lower_module(
        parsed, role=ModuleRole(own_names=("helpers",), patched=frozenset({"f"}))
    )
    assert 'm.global("f")?' in _fn_text(patched)


def test_module_patching_itself_by_its_import_name() -> None:
    source = (
        "import helpers\n\ndef f():\n    return 1\n\n\ndef g():\n    return f()\n\nhelpers.f = 2\n"
    )
    lowered = lower_module(parse_source(source, "helpers"), role=ModuleRole(own_names=("helpers",)))
    assert 'm.global("f")?' in _fn_text(lowered)


def _fn_text(lowered) -> str:
    from steelix.emit.crate import render

    return render(lowered)


def test_cli_transpiles_a_project(tmp_path: Path, capsys) -> None:
    write(
        tmp_path,
        {
            "app.py": "import pkg\nimport plugins.extra\n",
            "pkg/__init__.py": "from .core import run\n",
            "pkg/core.py": "def run():\n    yield 1\n",
            "plugins/__init__.py": "",
            "plugins/extra.py": "",
            "plugins/more.py": "",
        },
    )
    out = tmp_path / "out"
    report = tmp_path / "report.json"
    args = ["transpile", str(tmp_path / "app.py"), "-o", str(out), "--report", str(report)]
    assert main([*args, "--include", str(tmp_path / "plugins" / "more.py")]) == 0
    err = capsys.readouterr().err
    assert "transpiled modules pkg, pkg.core, plugins, plugins.extra, plugins.more" in err
    assert "pkg/core.py: native 0/1 (0%) functions" in err
    assert "all 6 modules: native 0/1 (0%) functions" in err
    files = sorted(p.relative_to(out / "src").as_posix() for p in (out / "src").rglob("*.rs"))
    assert files == [
        "app.rs",
        "main.rs",
        "pkg/core.rs",
        "pkg/mod.rs",
        "plugins/extra.rs",
        "plugins/mod.rs",
        "plugins/more.rs",
    ]
    assert "pub mod core;" in (out / "src" / "pkg" / "mod.rs").read_text()
    main_rs = (out / "src" / "main.rs").read_text()
    assert 'NativeModule::package("pkg", pkg::exec_module),' in main_rs
    assert "run_project(" in main_rs
    data = json.loads(report.read_text())
    assert [m["module"] for m in data["modules"]][:2] == ["app.py", "pkg/__init__.py"]
    assert data["total"]["functions"] == {"native": 0, "total": 1}
    assert data["features"][0]["feature"] == "yield"


def test_cli_python_m(tmp_path: Path, monkeypatch) -> None:
    write(tmp_path, {"pkg/__init__.py": "", "pkg/__main__.py": "print(1)\n"})
    monkeypatch.chdir(tmp_path)
    assert main(["transpile", "-m", "pkg", "-o", "out", "-q"]) == 0
    main_rs = (tmp_path / "out" / "src" / "main.rs").read_text()
    assert 'run_module(env!("STEELIX_PYTHON"), &script, MODULES, "pkg.__main__")' in main_rs
    assert f'argv0: "{tmp_path.resolve() / "pkg" / "__main__.py"}",' in main_rs
    assert 'name = "pkg"' in (tmp_path / "out" / "Cargo.toml").read_text()
    assert (tmp_path / "out" / "src" / "pkg" / "__main__.rs").is_file()


def test_cli_rejects_bad_inputs(tmp_path: Path, capsys) -> None:
    write(tmp_path, {"app.py": "", "pkg/__init__.py": ""})
    out = str(tmp_path / "out")
    assert main(["transpile", "-o", out]) == 2
    assert main(["transpile", str(tmp_path / "app.py"), "-m", "pkg", "-o", out]) == 2
    assert main(["transpile", str(tmp_path / "pkg"), "-o", out]) == 2
    assert "-m PACKAGE" in capsys.readouterr().err
    assert main(["transpile", "-m", "nothing", "--root", str(tmp_path), "-o", out]) == 2


def test_ext_mode_package(tmp_path: Path) -> None:
    write(tmp_path, {"pkg/__init__.py": "from . import a\n", "pkg/a.py": "", "pkg/b.py": ""})
    out = tmp_path / "out"
    args = ["transpile", "--mode", "ext", "-m", "pkg", "--root", str(tmp_path), "-o", str(out)]
    assert main([*args, "-q"]) == 0
    lib_rs = (out / "src" / "lib.rs").read_text()
    assert '#[pymodule(name = "pkg")]' in lib_rs
    assert "steelix_runtime::init_extension(m, MODULES)?;" in lib_rs
    assert 'NativeModule::module("pkg.b", pkg::b::exec_module),' in lib_rs
