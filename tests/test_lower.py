from __future__ import annotations

import textwrap

import pytest

from steelix.diagnostics import UnsupportedError
from steelix.emit.crate import render_module
from steelix.frontend.parse import parse_source
from steelix.lower.names import Renamer, rust_ident


def lower(source: str) -> str:
    return render_module(parse_source(textwrap.dedent(source), "m"))


def test_rust_ident() -> None:
    assert rust_ident("n") == "n"
    assert rust_ident("type") == "r#type"
    assert rust_ident("gen") == "r#gen"
    assert rust_ident("self") == "slf"
    assert rust_ident("crate") == "crate_"


def test_renamer_avoids_reserved_and_existing_names() -> None:
    names = Renamer(["py", "py_", "self", "slf", "match"], reserved={"py"})
    assert names["py_"] == "py_"
    assert names["py"] == "py__"
    assert names["slf"] == "slf"
    assert names["self"] == "slf_"
    assert names["match"] == "r#match"


@pytest.mark.parametrize(
    ("source", "feature", "line", "col"),
    [
        ("for i in x:\n    pass\n", "for loop", 1, 1),
        ("x = 1\nx += 1\n", "augmented assignment", 2, 1),
        ("import os\n", "import", 1, 1),
        ("def f(a=1):\n    pass\n", "default argument", 1, 9),
        ("def f(*, a):\n    pass\n", "keyword-only parameter", 1, 1),
        ("def f():\n    def g():\n        pass\n", "nested function", 2, 5),
        ("print(x.y)\n", "attribute access", 1, 7),
        ("print(f(a=1))\n", "keyword argument", 1, 9),
        ("x = 1 < 2 < 3\n", "chained comparison", 1, 5),
        ("x = a in b\n", "'in' comparison", 1, 5),
        ("x = not a\n", "unary operator", 1, 5),
        ("x = [1]\n", "list display", 1, 5),
        ("while x:\n    pass\nelse:\n    pass\n", "while/else", 4, 5),
        ("class A:\n    pass\n", "class definition", 1, 1),
        ("try:\n    pass\nexcept E:\n    pass\n", "try statement", 1, 1),
        ("x = 1j\n", "complex constant", 1, 5),
    ],
)
def test_unsupported_constructs_report_location(
    source: str, feature: str, line: int, col: int
) -> None:
    with pytest.raises(UnsupportedError) as info:
        lower(source)
    assert info.value.feature == feature
    assert (info.value.location.line, info.value.location.col + 1) == (line, col)
    assert str(info.value).startswith(f"m.py:{line}:{col}: unsupported: {feature}")


def test_literals_are_passed_directly_where_pyo3_accepts_them() -> None:
    out = lower(
        """
        def f(n):
            return n + 2147483648 + 1.5 + "s" + True
        """
    )
    assert 'n.add(2147483648i64)?.add(1.5)?.add("s")?.add(true)' in out


def test_value_comparison_uses_rich_compare() -> None:
    out = lower("def f(a, b):\n    return a < b\n")
    assert "use pyo3::pyclass::CompareOp;" in out
    assert "a.rich_compare(b, CompareOp::Lt)" in out


def test_calls_before_def_go_through_globals() -> None:
    out = lower(
        """
        print(f(1))
        def f(x):
            return x
        print(f(1))
        """
    )
    assert 'rt::global(globals, "f")?.call1((1,))?' in out
    assert "f(py, &rt::int(py, 1))?" in out


def test_option_local_and_none_return() -> None:
    out = lower(
        """
        def f(flag):
            if flag:
                x = 1
            print(x)
        """
    )
    assert "let mut x: Option<Bound<'py, PyAny>> = None;" in out
    assert "x = Some(rt::int(py, 1));" in out
    assert 'rt::local(&x, "x")?' in out
    assert "Ok(rt::none(py))" in out


def test_globals_builtin_reads_the_module_dict() -> None:
    out = lower("x = globals()\n")
    assert 'globals.set_item("x", globals.clone().into_any())?;' in out
