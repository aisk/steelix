from __future__ import annotations

import pytest

from steelix.rustast import nodes as rs
from steelix.rustast.fmt import rustfmt
from steelix.rustast.printer import byte_string_literal, print_expr, print_file, string_literal


def p(text: str) -> rs.Path:
    return rs.Path(text)


@pytest.mark.parametrize(
    ("expr", "expected"),
    [
        (rs.IntLit(7), "7"),
        (rs.IntLit(-7), "-7"),
        (rs.IntLit(2**40, "i64"), "1099511627776i64"),
        (rs.FloatLit(1.5), "1.5"),
        (rs.FloatLit(1e100), "1e+100"),
        (rs.FloatLit(float("inf")), "f64::INFINITY"),
        (rs.FloatLit(float("-inf")), "f64::NEG_INFINITY"),
        (rs.FloatLit(float("nan")), "f64::NAN"),
        (rs.BoolLit(True), "true"),
        (rs.StrLit('a"b\\c\n'), '"a\\"b\\\\c\\n"'),
        (rs.ByteStrLit(b"a\x00\xff"), 'b"a\\0\\xff"'),
        (rs.Call(p("f"), (p("py"), rs.Ref(p("x")))), "f(py, &x)"),
        (rs.Try(rs.MethodCall(p("n"), "lt", (rs.IntLit(2),))), "n.lt(2)?"),
        (rs.Ref(rs.Try(rs.MethodCall(p("n"), "sub", (rs.IntLit(1),)))), "&n.sub(1)?"),
        (rs.MethodCall(rs.Ref(p("x")), "clone"), "(&x).clone()"),
        (rs.MethodCall(rs.IntLit(-1), "abs"), "(-1).abs()"),
        (rs.Tuple((p("a"),)), "(a,)"),
        (rs.Tuple((p("a"), p("b"))), "(a, b)"),
        (rs.Tuple(()), "()"),
        (rs.MacroCall("rt::def", (p("globals"), rs.Raw("fib(n)"))), "rt::def!(globals, fib(n))"),
        (rs.Array((rs.IntLit(1), rs.IntLit(2))), "[1, 2]"),
        (rs.Unary("!", rs.Try(rs.MethodCall(p("x"), "is_truthy"))), "!x.is_truthy()?"),
        (rs.Unary("!", rs.Binary("&&", p("a"), p("b"))), "!(a && b)"),
        (rs.Binary("||", rs.Binary("&&", p("a"), p("b")), p("c")), "a && b || c"),
        (rs.Binary("&&", rs.Binary("||", p("a"), p("b")), p("c")), "(a || b) && c"),
        (rs.Binary(">", rs.Try(rs.MethodCall(p("xs"), "len")), rs.IntLit(3)), "xs.len()? > 3"),
        (
            rs.Binary("&&", p("a"), rs.BlockExpr(rs.Block((rs.Let("x", init=p("y")),), p("x")))),
            "a && {\n    let x = y;\n    x\n}",
        ),
        (
            rs.MethodCall(rs.IfExpr(p("c"), rs.Block((), p("a")), rs.Block((), p("b"))), "f"),
            "(if c {\n    a\n} else {\n    b\n}).f()",
        ),
    ],
)
def test_print_expr(expr: rs.Expr, expected: str) -> None:
    assert print_expr(expr) == expected


def test_string_literal_escapes_controls_but_keeps_unicode() -> None:
    assert string_literal("é中​\x7f") == '"é中\\u{200b}\\u{7f}"'
    assert byte_string_literal(b'"\t') == 'b"\\"\\t"'


def test_print_and_format_function() -> None:
    body = rs.Block(
        (
            rs.Let("x", mutable=True, ty="Bound<'py, PyAny>"),
            rs.If(
                rs.BoolLit(True),
                rs.Block((rs.Assign(p("x"), p("a")),)),
                rs.If(
                    rs.BoolLit(False),
                    rs.Block((rs.Return(p("y")),)),
                    rs.Block((rs.ExprStmt(rs.Call(p("f"))),)),
                ),
            ),
            rs.While(rs.BoolLit(False), rs.Block()),
            rs.Loop(rs.Block((rs.Return(),))),
            rs.LabeledBlock(
                "'for_else",
                rs.Block(
                    (
                        rs.For("x", p("xs"), rs.Block((rs.Break("'for_else"), rs.Continue()))),
                        rs.Let("[a, mut b]", init=p("pair")),
                    )
                ),
            ),
        ),
        tail=None,
    )
    fn = rs.Fn(
        "demo",
        (rs.Param("py", "Python<'py>"),),
        "()",
        body,
        lifetimes=("'py",),
        doc=("Docs.",),
        comments=("demo.py:1",),
    )
    file = rs.File([rs.Use("pyo3::prelude::*"), rs.Use("std::fmt"), fn])
    assert rustfmt(print_file(file)) == (
        "use pyo3::prelude::*;\n"
        "use std::fmt;\n"
        "\n"
        "// demo.py:1\n"
        "/// Docs.\n"
        "pub fn demo<'py>(py: Python<'py>) -> () {\n"
        "    let mut x: Bound<'py, PyAny>;\n"
        "    if true {\n"
        "        x = a;\n"
        "    } else if false {\n"
        "        return y;\n"
        "    } else {\n"
        "        f();\n"
        "    }\n"
        "    while false {}\n"
        "    loop {\n"
        "        return;\n"
        "    }\n"
        "    'for_else: {\n"
        "        for x in xs {\n"
        "            break 'for_else;\n"
        "            continue;\n"
        "        }\n"
        "        let [a, mut b] = pair;\n"
        "    }\n"
        "}\n"
    )


def test_rustfmt_reports_invalid_source() -> None:
    with pytest.raises(RuntimeError, match="rustfmt failed"):
        rustfmt("fn {")
