"""The attribute index and the `#[pyclass]` draft (`steelix attrs`, design
§15). The expected reports are goldens under `tests/golden/attrs/`."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import pytest
from conftest import E2E_DIR, GOLDEN_DIR, ROOT, cargo_env
from test_golden import check_golden

from steelix.attrs import AttributeIndex, build_index, format_text, select, to_json
from steelix.cli import main
from steelix.project import find_project
from steelix.pyclass import draft

SHAPES = ROOT / "tests" / "attrs" / "shapes"
SKIME = ROOT / "tests" / "projects" / "skime"
GOLDEN = GOLDEN_DIR / "attrs"
DRAFTS = ["Point", "Size", "Odd"]


@pytest.fixture(scope="module")
def shapes() -> AttributeIndex:
    return build_index(find_project(SHAPES / "app.py"))


@pytest.fixture(scope="module")
def skime() -> AttributeIndex:
    return build_index(find_project(module="skime", root=SKIME))


def accesses(index: AttributeIndex, cls: str, attribute: str) -> list[tuple[str, str, int, str]]:
    (found,) = select(index, [cls])
    return [(a.effect, a.function, a.line, a.via) for a in found.members[attribute].accesses]


def test_index_golden(shapes: AttributeIndex, update_golden: bool) -> None:
    check_golden(format_text(shapes), GOLDEN / "shapes.txt", update_golden)


@pytest.mark.parametrize("name", DRAFTS)
def test_draft_golden(shapes: AttributeIndex, name: str, update_golden: bool) -> None:
    (cls,) = select(shapes, [name])
    text = draft(shapes, cls, f"steelix attrs app.py --pyclass {cls.full_name}")
    check_golden(text, GOLDEN / f"{name.lower()}.rs", update_golden)


def test_self_and_inferred_receivers(shapes: AttributeIndex) -> None:
    assert accesses(shapes, "Point", "x") == [
        ("write", "Point.__init__", 12, "self"),
        ("read", "Point.norm", 18, "self"),
        ("augmented", "Point.move", 21, "self"),
        # An inherited attribute read in a subclass belongs to the base.
        ("read", "Point3.flat", 37, "self"),
        ("augmented", "scale", 10, "annotation"),
    ]
    # Nested functions and lambdas see `self`; private names are mangled.
    assert ("read", "Point.later.<locals>.inner", 26, "self") in accesses(shapes, "Point", "y")
    assert accesses(shapes, "Point", "_Point__secret")[-1] == ("read", "Point.later", 28, "self")
    # A local constructed from the class, through an import alias.
    assert accesses(shapes, "Point", "label") == [("write", "origin", 16, "constructed")]
    (point,) = select(shapes, ["shapes.Point"])
    assert not point.members["label"].static
    assert [s.full_name for s in point.subclasses] == ["shapes.Point3"]


def test_conservative_receivers(shapes: AttributeIndex) -> None:
    unknown = {(a.function, a.receiver, a.attribute) for a in shapes.unknown}
    # Rebound to `None`, rebound by a nested function, a class name bound twice.
    assert ("maybe", "q", "x") in unknown
    assert ("closure", "r", "y") in unknown
    assert ("far", "p", "x") in unknown
    assert ("show", "thing", "width") in unknown


def test_class_attributes_and_dynamic_access(shapes: AttributeIndex) -> None:
    assert accesses(shapes, "Point", "count") == [
        ("augmented", "Point.__init__", 15, "class"),
        ("write", "reset", 49, "class"),
    ]
    assert accesses(shapes, "Odd", "LIMIT") == [
        ("write", "Odd.build", 78, "cls"),
        ("del", "reset", 50, "class"),
    ]
    (point,) = select(shapes, ["Point"])
    assert [(a.kind, a.function) for a in point.dynamic] == [
        ("vars", "configure"),
        ("setattr", "configure"),
    ]
    # A literal name is an ordinary access.
    assert ("read", "configure", 43, "annotation") in accesses(shapes, "Point", "y")
    (odd,) = select(shapes, ["Odd"])
    assert [a.kind for a in odd.dynamic] == ["vars", "__dict__"]
    assert odd.hooks == ["__getattr__"]
    assert odd.slots == ("a", "_b")
    assert odd.body_logic == [(57, "if LIMIT > 5:")]
    assert [(a.kind, a.function) for a in shapes.dynamic] == [
        ("getattr", "show"),
        ("getattr", "Odd.__getattr__"),
    ]


def test_json(shapes: AttributeIndex) -> None:
    data = json.loads(json.dumps(to_json(shapes, select(shapes, ["Point"]))))
    (point,) = data["classes"]
    assert point["name"] == "shapes.Point"
    assert point["subclasses"] == ["shapes.Point3"]
    assert point["outside_modules"] == ["app", "shapes"]
    label = next(a for a in point["attributes"] if a["name"] == "label")
    assert label["accesses"] == [
        {
            "attribute": "label",
            "kind": "write",
            "module": "app",
            "file": "app.py",
            "line": 16,
            "function": "origin",
            "receiver": "o",
            "via": "constructed",
            "evidence": "o = P(..)",
        }
    ]
    assert {a["attribute"] for a in data["unknown"]} == {"x", "y"}


def test_cli(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    app = str(SHAPES / "app.py")
    assert main(["attrs", app, "--class", "Point3"]) == 0
    out = capsys.readouterr().out
    assert out.startswith("class shapes.Point3  (shapes.py:31, top level)\n")
    assert "class shapes.Point " not in out
    assert main(["attrs", app, "--json"]) == 0
    assert len(json.loads(capsys.readouterr().out)["classes"]) == 6
    output = tmp_path / "point.rs"
    assert main(["attrs", app, "--pyclass", "shapes.Point", "-o", str(output)]) == 0
    assert output.read_text().startswith("// DRAFT of a `#[pyclass]` for class `Point`")
    assert main(["attrs", "-m", "app", "--root", str(SHAPES), "--class", "Size"]) == 0
    assert "class shapes.Size" in capsys.readouterr().out
    assert main(["attrs", app, "--class", "Nope"]) == 2
    assert "no class 'Nope'" in capsys.readouterr().err
    (tmp_path / "a.py").write_text("import b\n\nclass C:\n    pass\n")
    (tmp_path / "b.py").write_text("class C:\n    pass\n")
    assert main(["attrs", str(tmp_path / "a.py"), "--pyclass", "C"]) == 2
    assert "'C' is ambiguous: a.C, b.C" in capsys.readouterr().err
    assert main(["attrs", str(tmp_path / "a.py"), "--pyclass", "b.C"]) == 0


def test_skime_context(skime: AttributeIndex, update_golden: bool) -> None:
    """The M10 question: who reads and writes `Context` (m10-skime.md §6)."""
    (context,) = select(skime, ["skime.ctx.Context"])
    names = {m.name for m in context.instance_attributes()}
    assert names == {"bytecode", "env", "form", "ip", "parent", "stack", "vm"}
    # `ctx` is an unannotated parameter in `insns.py`, so these accesses are
    # found by name, with the modules a rewrite has to cover.
    modules = {a.module for a in skime.unknown if a.attribute == "ip"}
    assert modules == {"skime.insns", "skime.call_cc"}
    # `form.py` and `vm.py` only construct it.
    assert [a.function for a in context.constructed] == [
        "Context.clone",
        "Form.eval",
        "invoke",
        "VM.__init__",
    ]
    pushes = {a.module for a in skime.unknown_methods if a.attribute == "push"}
    assert pushes == {"skime.insns", "skime.invoke"}
    check_golden(format_text(skime, [context]), GOLDEN / "skime_context.txt", update_golden)
    command = "steelix attrs -m skime --root tests/projects/skime --pyclass skime.ctx.Context"
    check_golden(draft(skime, context, command), GOLDEN / "skime_context.rs", update_golden)


@pytest.mark.cargo
def test_drafts_are_valid_pyo3() -> None:
    """The stubs of the drafts type-check against PyO3 (with `todo!()`
    bodies), so that finishing one starts from code that compiles. `extends`
    needs the base to be a `#[pyclass]` too, so it is taken out."""
    import subprocess

    crate = E2E_DIR / "attrs-drafts"
    if crate.exists():
        shutil.rmtree(crate)
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "attrs-drafts"\nversion = "0.0.0"\nedition = "2024"\n\n'
        '[dependencies]\npyo3 = "0.30"\n\n[workspace]\n'
    )
    shutil.copyfile(ROOT / "Cargo.lock", crate / "Cargo.lock")
    names = [*(n.lower() for n in DRAFTS), "skime_context"]
    for name in names:
        text = (GOLDEN / f"{name}.rs").read_text().replace(", extends = Base", "")
        (crate / "src" / f"{name}.rs").write_text(text)
    (crate / "src" / "lib.rs").write_text("".join(f"mod {n};\n" for n in names))
    result = subprocess.run(
        ["cargo", "check", "--offline"],
        cwd=crate,
        env=cargo_env(extension=False),
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert "error" not in result.stderr
