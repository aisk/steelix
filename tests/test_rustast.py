from __future__ import annotations

import re
from pathlib import Path

import pytest

from steelix.rustast import nodes as rs
from steelix.rustast.cleanup import declare_late, move_returned_locals
from steelix.rustast.fmt import rustfmt
from steelix.rustast.printer import byte_string_literal, print_expr, print_file, string_literal
from steelix.rustast.rewrite import count_path, replace_path


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
        (
            rs.MacroCall("rt::def", (p("m"), rs.Tokens((p("fib"), "(n)")))),
            "rt::def!(m, fib(n))",
        ),
        # rustfmt does not lay out macros whose arguments are not expressions.
        (
            rs.MacroCall(
                "rt::function",
                (p("globals"), rs.Tokens(("f[a, b](x, /, y)",)), rs.Tokens(("q" * 40,))),
            ),
            f"rt::function!(\n    globals,\n    f[a, b](x, /, y),\n    {'q' * 40}\n)",
        ),
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


def test_long_macro_call_is_indented_for_its_line() -> None:
    # rustfmt leaves the whole statement alone when it can not lay out the
    # macro (`slf as "self"`), so the printed indentation is the final one.
    call = rs.MacroCall("rt::function", (p("ns"), rs.Tokens((f'f(slf as "self", {"x" * 60})',))))
    stmt = rs.ExprStmt(rs.Try(rs.MethodCall(p("ns"), "set", (rs.StrLit("f"), rs.Try(call)))))
    block = rs.Block((stmt,))
    assert print_expr(rs.BlockExpr(block), 1) == (
        "{\n"
        '        ns.set("f", rt::function!(\n'
        "            ns,\n"
        f'            f(slf as "self", {"x" * 60})\n'
        "        )?)?;\n"
        "    }"
    )


def test_docstring_of_several_lines_keeps_its_value() -> None:
    doc = 'Summary "q".\n\n  indented\n\ttab \\ end\n'
    option = rs.Tokens(("doc = ", rs.StrLit(doc, lines=True)))
    text = print_expr(rs.MacroCall("rt::def", (p("m"), rs.Tokens(("f()",)), option)), 1)
    assert text == (
        "rt::def!(\n"
        "        m,\n"
        "        f(),\n"
        '        doc = "Summary \\"q\\".\\n\\\n'
        "            \\n\\\n"
        "            \\x20 indented\\n\\\n"
        '            \\ttab \\\\ end\\n"\n'
        "    )"
    )
    # What Rust makes of the literal: a `\` at the end of a line drops the
    # line break and the whitespace after it.
    literal = text[text.index("doc = ") + 6 : text.rindex('"') + 1]
    unescaped = re.sub(r"\\\n\s*", "", literal)[1:-1]
    escapes = {"\\n": "\n", "\\t": "\t", "\\x20": " ", '\\"': '"', "\\\\": "\\"}
    value = re.sub(r'\\(n|t|x20|"|\\)', lambda m: escapes[m.group(0)], unescaped)
    assert value == doc
    # One line stays one literal.
    assert print_expr(rs.StrLit("one line\n", lines=True)) == '"one line\\n"'


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


def test_path_rewrites_skip_nested_fns() -> None:
    p = rs.Path
    nested = rs.Fn("inner", (), "()", rs.Block((), p("py")))
    block = rs.Block(
        (rs.ItemStmt(nested), rs.ExprStmt(rs.Call(p("f"), (p("py"), p("x"))))),
        rs.Tokens(("g(", p("py"), ")")),
    )
    assert count_path(block, "py") == 2
    replaced = replace_path(block, "py", rs.MethodCall(p("m"), "py"))
    assert count_path(replaced, "py") == 0
    assert replaced.stmts[0] == rs.ItemStmt(nested)
    assert replaced.tail == rs.Tokens(("g(", rs.MethodCall(p("m"), "py"), ")"))


# --- cleanup -------------------------------------------------------------------


def _ok(expr: rs.Expr) -> rs.Expr:
    return rs.Call(p("Ok"), (expr,))


def _clone(name: str) -> rs.Expr:
    return rs.MethodCall(p(name), "clone")


def _cleaned(body: rs.Block, params: tuple[rs.Param, ...] = ()) -> str:
    fn = rs.Fn("f", params, "PyResult<Bound<'py, PyAny>>", body, ("'py",))
    return print_file(move_returned_locals(rs.File([fn])))


def test_locals_declared_in_the_body_are_moved_when_returned() -> None:
    x = rs.Let("x", init=rs.Try(rs.Call(p("make"), ())))
    early = rs.If(p("c"), rs.Block((rs.Return(_ok(_clone("x"))),)))
    out = _cleaned(rs.Block((x, early), _ok(_clone("x"))))
    assert "return Ok(x);" in out
    assert out.endswith("    Ok(x)\n}\n")
    assert "clone" not in out


def test_parameters_and_captured_locals_keep_their_clone() -> None:
    outer = rs.Let("x", init=rs.Try(rs.Call(p("make"), ())))
    closure = rs.Closure((), rs.Block((), _ok(_clone("x"))), "PyResult<Bound<'py, PyAny>>")
    call = rs.Let("r", init=rs.Call(closure, ()))
    param = rs.Param("n", "&Bound<'py, PyAny>")
    body = rs.Block((outer, call, rs.Return(_ok(_clone("n")))), _ok(_clone("x")))
    out = _cleaned(body, (param,))
    assert "        Ok(x.clone())\n    })();" in out
    assert "return Ok(n.clone());" in out
    # Shadowed by an owned local, the parameter is moved too.
    shadow = rs.Let("n", True, init=_clone("n"))
    assert "Ok(n)" in _cleaned(rs.Block((shadow,), _ok(_clone("n"))), (param,))


def test_locals_borrowed_by_a_guard_and_option_locals_keep_their_clone() -> None:
    err = rs.Let("err", init=rs.MethodCall(p("frame"), "catch", (p("e"),)))
    guard = rs.Let("_handling", init=rs.Call(p("rt::Handling::new"), (rs.Ref(p("err")),)))
    out = _cleaned(rs.Block((err, guard), _ok(_clone("err"))))
    assert "Ok(err.clone())" in out
    option = rs.Let("y", True, "Option<Bound<'py, PyAny>>", p("None"))
    assert "Ok(y.clone())" in _cleaned(rs.Block((option,), _ok(_clone("y"))))


def test_if_let_and_nested_returns() -> None:
    some = rs.Call(p("Some"), (_clone("x"),))
    then = rs.Block((rs.Return(_ok(some)),))
    next_ = rs.Try(rs.Call(p("rt::next"), (p("it"),)))
    stmt = rs.If(next_, then, pattern="Some(x)")
    out = _cleaned(rs.Block((stmt,), _ok(p("None"))))
    assert "return Ok(Some(x));" in out


BOUND = "Bound<'py, PyAny>"


# The function's locals; anything else is a temporary.
LOCALS = {"a", "x", "t", "y"}


def _merged(body: rs.Block) -> str:
    fn = rs.Fn("f", (), "PyResult<Bound<'py, PyAny>>", declare_late(body, LOCALS), ("'py",))
    return print_file(rs.File([fn]))


def _set(name: str, value: rs.Expr) -> rs.Assign:
    return rs.Assign(p(name), value)


def _call(name: str) -> rs.Expr:
    return rs.Try(rs.Call(p(name), ()))


def test_late_init_local_is_declared_with_the_if_chain() -> None:
    let = rs.Let("x", ty=BOUND)
    third = rs.Block((rs.ExprStmt(_call("log")), _set("x", _call("c"))))
    leaves = rs.Block((rs.Return(_ok(_call("d"))),))
    chain = rs.If(
        p("a"),
        rs.Block((_set("x", _call("b")),)),
        rs.If(p("p"), third, rs.If(p("q"), rs.Block((_set("x", p("y")),)), leaves)),
    )
    out = _merged(rs.Block((let, rs.Comment("chain"), chain), _ok(p("x"))))
    assert "let x = if a {\n        b()?\n    } else if p {\n        log()?;\n        c()?\n" in out
    assert "} else if q {\n        y\n    } else {\n        return Ok(d()?);\n    };" in out
    assert "// chain\n    let x = if" in out
    assert "x =" not in out.replace("let x =", "")


def test_late_init_mut_only_when_assigned_again() -> None:
    chain = rs.If(p("a"), rs.Block((_set("x", p("b")),)), rs.Block((_set("x", p("c")),)))
    again = _set("x", _call("d"))
    out = _merged(rs.Block((rs.Let("x", True, BOUND), chain, again), _ok(p("x"))))
    assert "let mut x = if a {" in out
    out = _merged(rs.Block((rs.Let("x", True, BOUND), chain), _ok(p("x"))))
    assert "let x = if a {" in out
    # A plain assignment as the first use.
    out = _merged(rs.Block((rs.Let("x", ty=BOUND), rs.ExprStmt(_call("g")), again), p("x")))
    assert "    g()?;\n    let x = d()?;\n" in out


def test_late_init_local_is_declared_with_the_match() -> None:
    arms = (
        rs.Arm("Ok(v)", rs.Block((_set("x", p("v")),))),
        rs.Arm("Err(e)", rs.Block((rs.ExprStmt(_call("log")), _set("x", p("none")))), p("g")),
        rs.Arm("Err(e)", rs.Block((rs.Return(rs.Call(p("Err"), (p("e"),))),))),
    )
    stmt = rs.ExprStmt(rs.Match(_call("r"), arms))
    out = _merged(rs.Block((rs.Let("x", ty=BOUND), stmt), _ok(p("x"))))
    assert "let x = match r()? {\n        Ok(v) => v,\n" in out
    assert "Err(e) if g => {\n            log()?;\n            none\n        }" in out
    # An arm binding the name assigns another variable.
    shadow = rs.Arm("Ok(x)", rs.Block((_set("x", p("v")),)))
    stmt = rs.ExprStmt(rs.Match(_call("r"), (shadow, *arms[1:])))
    assert "let x: Bound" in _merged(rs.Block((rs.Let("x", ty=BOUND), stmt), _ok(p("x"))))


@pytest.mark.parametrize(
    "stmts",
    [
        # No `else`.
        [rs.If(p("a"), rs.Block((_set("x", p("b")),)))],
        # Read or assigned in a branch before the end.
        [
            rs.If(
                p("a"),
                rs.Block((_set("x", p("b")), _set("x", p("c")))),
                rs.Block((_set("x", p("c")),)),
            )
        ],
        [
            rs.If(
                p("a"),
                rs.Block((_set("x", p("b")),)),
                rs.Block((_set("y", p("x")), _set("x", p("c")))),
            )
        ],
        # Assigns something else last.
        [
            rs.If(
                p("a"),
                rs.Block((_set("x", p("b")), _set("y", p("c")))),
                rs.Block((_set("x", p("c")),)),
            )
        ],
        # Every branch leaves.
        [rs.If(p("a"), rs.Block((rs.Break(),)), rs.Block((rs.Continue(),)))],
        # Read in the condition.
        [rs.If(rs.Ref(p("x")), rs.Block((_set("x", p("b")),)), rs.Block((_set("x", p("c")),)))],
        # `if let`.
        [rs.If(p("a"), rs.Block((_set("x", p("b")),)), rs.Block((_set("x", p("c")),)), "Some(z)")],
    ],
)
def test_late_init_local_kept(stmts: list[rs.Stmt]) -> None:
    out = _merged(rs.Block((rs.Let("x", ty=BOUND), *stmts), _ok(p("x"))))
    assert f"let x: {BOUND};" in out


def test_late_init_moves_past_other_declarations() -> None:
    # Past another local, whatever the drop order then is (design §12), or a
    # temporary.
    stmts = (rs.Let("x", ty=BOUND), rs.Let("a", init=_call("a")), _set("x", p("b")))
    assert "let a = a()?;\n    let x = b;\n" in _merged(rs.Block(stmts, p("x")))
    stmts = (rs.Let("x", ty=BOUND), rs.Let("t", ty=BOUND), _set("x", p("b")))
    assert f"let t: {BOUND};\n    let x = b;\n" in _merged(rs.Block(stmts, p("x")))
    stmts = (rs.Let("x", ty=BOUND), rs.Let("tmp", init=_call("g")), _set("x", p("b")))
    assert "let tmp = g()?;\n    let x = b;\n" in _merged(rs.Block(stmts, p("x")))
    # Rust does not drop a `bool`.
    stmts = (rs.Let("x", ty="bool"), rs.Let("t", init=_call("t")), _set("x", rs.BoolLit(True)))
    assert "let t = t()?;\n    let x = true;\n" in _merged(rs.Block(stmts, p("x")))
    stmts = (rs.Let("x", ty=BOUND), rs.Let("t", ty="bool"), _set("x", p("b")))
    assert "let t: bool;\n    let x = b;\n" in _merged(rs.Block(stmts, p("x")))


def test_declarations_move_down_to_their_first_use() -> None:
    late = rs.Let("x", ty=BOUND)
    option = rs.Let("y", True, f"Option<{BOUND}>", p("None"))
    no_else = rs.If(p("c"), rs.Block((_set("x", p("b")), _set("y", p("d")))))
    stmts = (late, option, rs.ExprStmt(_call("g")), rs.Comment("note"), no_else)
    out = _merged(rs.Block(stmts, rs.Tuple((p("x"), p("y")))))
    assert (
        f"g()?;\n    let x: {BOUND};\n    let mut y: Option<{BOUND}> = None;\n    // note\n" in out
    )
    # Past other declarations too.
    stmts = (late, rs.ExprStmt(_call("g")), rs.Let("t", init=_call("t")), no_else)
    out = _merged(rs.Block(stmts, p("x")))
    assert f"g()?;\n    let t = t()?;\n    let x: {BOUND};\n" in out
    # Only the tail uses it; a local nothing uses stays.
    stmts = (late, rs.ExprStmt(_call("g")))
    assert f"g()?;\n    let x: {BOUND};\n    x\n" in _merged(rs.Block(stmts, p("x")))
    assert f"let x: {BOUND};\n    g()?;" in _merged(rs.Block(stmts))
    # An `Option` local gets its first value too; the type stays when the
    # value may be a bare `None`.
    some = _set("y", rs.Call(p("Some"), (p("b"),)))
    out = _merged(rs.Block((option, some, _set("y", p("None"))), p("y")))
    assert "let mut y = Some(b);\n    y = None;\n" in out
    choice = rs.If(p("c"), rs.Block((some,)), rs.Block((_set("y", p("None")),)))
    out = _merged(rs.Block((option, choice), p("y")))
    assert f"let y: Option<{BOUND}> = if c {{" in out
    # Nested fns have their own locals.
    inner = rs.Fn("inner", (), "()", rs.Block((late, rs.ExprStmt(_call("g")), no_else)))
    out = _merged(rs.Block((rs.ItemStmt(inner),)))
    assert f"let x: {BOUND};\n        g()?;" in out


_LATE_LET = re.compile(r"^( *)let (?:mut )?((?:r#)?\w+): [^=]+;$")


def _late_init_if_chains(text: str) -> list[str]:
    """`let x: T;` followed (comments aside) by an `if` with an `else` whose
    every branch ends by assigning `x` or leaving, in formatted Rust."""
    lines = text.splitlines()
    found = []
    for i, line in enumerate(lines):
        if (match := _LATE_LET.match(line)) is None:
            continue
        indent, name = match.groups()
        j = i + 1
        while j < len(lines) and lines[j].strip().startswith("//"):
            j += 1
        if not lines[j].startswith(f"{indent}if "):
            continue
        # The branches: lines one level deeper, up to the closing `}` lines.
        branches: list[list[str]] = [[]]
        has_else = False
        for line in lines[j + 1 :]:
            if line == f"{indent}}}":
                break
            if line.startswith(f"{indent}}} else"):
                has_else |= line == f"{indent}}} else {{"
                branches.append([])
            elif line.startswith(f"{indent}    ") and not line.startswith(f"{indent}     "):
                if not line.strip().startswith((")", "]", "}", ".", "?", "//")):
                    branches[-1].append(line.strip())
        ends = [b[-1] if b else "" for b in branches]
        leaves = ("return", "break", "continue")
        if has_else and all(e.startswith((f"{name} = ", *leaves)) for e in ends):
            if any(e.startswith(f"{name} = ") for e in ends):
                found.append(f"{name}: {lines[j].strip()}")
    return found


def test_late_init_scan_finds_the_pattern() -> None:
    text = """
    let kind: Bound<'py, PyAny>;
    // comment
    if n.lt(0)? {
        kind = rt::str(py, "negative");
    } else if n.eq(0)? {
        return Ok(n.clone());
    } else {
        f(
            a,
        )?;
        kind = rt::str(py, "large");
    }
    let other: Bound<'py, PyAny>;
    if c {
        other = a;
    }
"""
    assert _late_init_if_chains(text) == ["kind: if n.lt(0)? {"]


def test_goldens_have_no_late_init_if_chains() -> None:
    root = Path(__file__).parent / "golden"
    found = {
        str(path.relative_to(root)): chains
        for path in sorted(root.rglob("*.rs"))
        if (chains := _late_init_if_chains(path.read_text()))
    }
    assert found == {}
