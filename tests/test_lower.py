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
        ("x = [i for i in y]\n", "list comprehension", 1, 5),
        ("x = [*a]\n", "unpacking in a display", 1, 6),
        ("x = {**a}\n", "unpacking in a display", 1, 8),
        ("x: int = 1\n", "annotated assignment at module level", 1, 1),
        ("def f(a):\n    a.b: int\n", "annotated assignment", 2, 5),
        ("from . import x\n", "relative import", 1, 1),
        ("from os import *\n", "star import", 1, 1),
        ("raise E\n", "raise", 1, 1),
        ("with a:\n    pass\n", "with statement", 1, 1),
        ("class A:\n    pass\n", "class definition", 1, 1),
        ("try:\n    pass\nexcept E:\n    pass\n", "try statement", 1, 1),
        ("if (y := 1):\n    pass\n", "assignment expression", 1, 5),
        ("x = t'{a}'\n", "t-string", 1, 5),
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


def test_pure_operands_need_no_temporaries() -> None:
    out = lower(
        """
        def f(x, xs, d, i):
            d[i] = x
            return x in xs
        """
    )
    assert "d.set_item(i, x)?;" in out
    assert "Ok(rt::bool(py, xs.contains(x)?))" in out
    assert "let " not in out.split("pub fn exec_module")[0]


def test_value_is_evaluated_before_the_assignment_target() -> None:
    out = lower(
        """
        def f(g, h):
            g()[h()] = g()
        """
    )
    assert "let value = g.call0()?;" in out
    assert "g.call0()?.set_item(h.call0()?, &value)?;" in out


def test_in_evaluates_the_item_first() -> None:
    out = lower("def f(g, h):\n    return g() in h()\n")
    assert "let item = g.call0()?;" in out
    assert "h.call0()?.contains(&item)?" in out


def test_method_lookup_happens_before_arguments() -> None:
    out = lower(
        """
        def f(o, g, x):
            o.m(x, 1)
            o.m(g())
        """
    )
    assert 'o.call_method1("m", (x, 1))?;' in out
    assert 'o.getattr("m")?.call1((g.call0()?,))?;' in out


def test_augmented_assignment_evaluates_target_parts_once() -> None:
    out = lower(
        """
        def f(a, g, v):
            a[g()] += v
            a.x -= v
            n = 0
            n *= v
        """
    )
    assert "let index = g.call0()?;" in out
    assert "a.set_item(&index, rt::iadd(&a.get_item(&index)?, v)?)?;" in out
    assert 'a.setattr("x", rt::isub(&a.getattr("x")?, v)?)?;' in out
    assert "n = rt::imul(&n, v)?;" in out


def test_conditions_use_rust_operators() -> None:
    out = lower(
        """
        def f(a, b, xs):
            if a and not b or a is None:
                return 1
            if 0 < len(xs) <= 3 and a not in xs:
                return 2
        """
    )
    assert "if a.is_truthy()? && !b.is_truthy()? || a.is_none() {" in out
    # The middle operand of a chain is evaluated once.
    assert "let len = xs.len()?;\n    if 0 < len && len <= 3 && !xs.contains(a)? {" in out


def test_builtin_special_cases_respect_rebinding() -> None:
    out = lower(
        """
        def f(x):
            return len(x)
        """
    )
    assert "Ok(rt::int(py, x.len()?))" in out
    out = lower(
        """
        def f(x):
            return len(x)
        len = repr
        """
    )
    assert 'rt::global(globals, "len")?.call1((x,))' in out


def test_for_loop_variable_scoped_to_the_body() -> None:
    out = lower(
        """
        def f(xs):
            for x in xs:
                print(x)
        """
    )
    assert "for x in xs.try_iter()? {\n        let x = x?;" in out


def test_imports_bind_names() -> None:
    out = lower(
        """
        import os.path
        import a.b.c as d
        from x.y import z as w, v
        """
    )
    assert 'globals.set_item("os", rt::import(globals, "os.path")?)?;' in out
    assert 'globals.set_item("d", rt::import_module(globals, "a.b.c")?)?;' in out
    assert 'let y = rt::import_names(globals, "x.y", &["z", "v"])?;' in out
    assert 'globals.set_item("w", rt::import_from(&y, "z")?)?;' in out
