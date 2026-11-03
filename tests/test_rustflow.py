"""Unit tests of the questions asked about lowered Rust code
(`lower.rustflow`) and of the temporary lifetimes pass (`lower.temps`)."""

from __future__ import annotations

import ast

import pytest

from steelix.lower.rustflow import diverges, fallible, returns_as_tail, statement_line
from steelix.lower.temps import settle
from steelix.rustast import nodes as rs
from steelix.rustast.printer import print_expr, print_stmt


def p(text: str) -> rs.Path:
    return rs.Path(text)


def call(name: str, *args: rs.Expr) -> rs.Expr:
    return rs.Try(rs.Call(p(name), args))


def block(*stmts: rs.Stmt, tail: rs.Expr | None = None) -> rs.Block:
    return rs.Block(stmts, tail)


RETURN = rs.Return(p("x"))
BREAK = rs.Break()
CALL = rs.ExprStmt(call("f"))
OK = rs.Call(p("Ok"), (p("x"),))


# --- diverges ----------------------------------------------------------------------


@pytest.mark.parametrize(
    ("stmts", "expected"),
    [
        ([], False),
        ([CALL], False),
        ([CALL, RETURN], True),
        ([rs.Continue()], True),
        ([rs.ExprStmt(rs.MacroCall("unreachable"))], True),
        # Both branches must leave.
        ([rs.If(p("c"), block(RETURN))], False),
        ([rs.If(p("c"), block(RETURN), block(BREAK))], True),
        ([rs.If(p("c"), block(RETURN), rs.If(p("d"), block(RETURN)))], False),
        ([rs.If(p("c"), block(RETURN), rs.If(p("d"), block(RETURN), block(RETURN)))], True),
        # Every arm, without a tail value.
        ([rs.ExprStmt(rs.Match(p("r"), (rs.Arm("_", block(RETURN)),)))], True),
        ([rs.ExprStmt(rs.Match(p("r"), (rs.Arm("_", block(CALL)),)))], False),
        # A loop is left only through a `break` of its own.
        ([rs.Loop(block(CALL))], True),
        ([rs.Loop(block(rs.If(p("c"), block(BREAK))))], False),
        ([rs.Loop(block(rs.Loop(block(BREAK))))], True),
        ([rs.Loop(block(rs.Loop(block(rs.Break("'outer")))), "'outer")], False),
        ([rs.Loop(block(rs.ExprStmt(rs.Closure((), block(BREAK)))))], True),
        # A labeled block falls through its end, or is left by `break 'label`.
        ([rs.LabeledBlock("'b", block(RETURN))], True),
        ([rs.LabeledBlock("'b", block(rs.If(p("c"), block(rs.Break("'b"))), RETURN))], False),
        # An unlabeled `break` in a labeled block belongs to a loop around it.
        ([rs.LabeledBlock("'b", block(BREAK))], True),
    ],
)
def test_diverges(stmts: list[rs.Stmt], expected: bool) -> None:
    assert diverges(stmts) is expected


# --- returns_as_tail -----------------------------------------------------------------


def tail_of(stmts: list[rs.Stmt]) -> tuple[int, str | None]:
    rest, tail = returns_as_tail(stmts)
    return len(rest), None if tail is None else print_expr(tail)


def test_final_return_becomes_the_tail() -> None:
    assert tail_of([CALL, rs.Return(OK)]) == (1, "Ok(x)")
    assert tail_of([CALL]) == (1, None)
    assert tail_of([rs.Return()]) == (1, None)  # `return;` has no value
    assert tail_of([]) == (0, None)


def test_diverging_if_becomes_an_if_expression() -> None:
    stmt = rs.If(
        p("c"),
        block(CALL, rs.Return(OK)),
        rs.If(p("d"), block(rs.Return(p("y"))), block(rs.Return(p("z")))),
    )
    rest, tail = returns_as_tail([stmt])
    assert rest == []
    assert isinstance(tail, rs.IfExpr)
    text = print_expr(tail, 0)
    assert "return" not in text
    assert "f()?;\n    Ok(x)" in text and "} else if d {" in text
    # An `if` that may fall through, or an `if let`, stays.
    assert tail_of([rs.If(p("c"), block(RETURN))]) == (1, None)
    assert tail_of([rs.If(p("c"), block(RETURN), block(RETURN), pattern="Some(v)")]) == (1, None)


def test_diverging_match_becomes_a_match_expression() -> None:
    arms = (
        rs.Arm("Ok(v)", block(rs.Return(p("v")))),
        rs.Arm("Err(e)", block(rs.Return(rs.Call(p("Err"), (p("e"),))))),
    )
    rest, tail = returns_as_tail([CALL, rs.ExprStmt(rs.Match(p("r"), arms))])
    assert len(rest) == 1
    assert isinstance(tail, rs.Match)
    assert "return" not in print_expr(tail)


# --- fallible and statement_line -------------------------------------------------------


@pytest.mark.parametrize(
    ("stmts", "expected"),
    [
        ([rs.Let("x", init=rs.Call(p("f")))], False),
        ([CALL], True),
        ([rs.Return(OK)], False),
        ([rs.Return(rs.Call(p("f")))], True),  # a `PyResult` that may be `Err`
        ([rs.ExprStmt(rs.MethodCall(p("frame"), "line", (rs.IntLit(3),)))], False),
        ([rs.ExprStmt(rs.MethodCall(p("frame"), "catch", (p("err"),)))], True),
        # Code in a nested fn item runs elsewhere.
        ([rs.ItemStmt(rs.Fn("g", (), "PyResult<()>", block(CALL)))], False),
    ],
)
def test_fallible(stmts: list[rs.Stmt], expected: bool) -> None:
    assert fallible(stmts) is expected


def test_statement_line() -> None:
    tree = ast.parse("x = (\n    f()\n)\nreturn (\n    g()\n)\nif (\n    a\n):\n    pass\n")
    assign, ret, if_ = tree.body
    assert statement_line(assign) == 2
    assert statement_line(ret) == 5
    assert statement_line(if_) == 7


# --- temporaries ------------------------------------------------------------------------


def text(stmts: list[rs.Stmt]) -> str:
    return "\n".join(print_stmt(stmt, 0) for stmt in stmts)


def let(name: str, init: rs.Expr) -> rs.Let:
    return rs.Let(name, init=init)


def into(name: str) -> rs.Ref:
    return rs.Ref(p(name), into=True)


def set_item(target: str, value: rs.Expr) -> rs.Stmt:
    return rs.ExprStmt(rs.Try(rs.MethodCall(p(target), "set_item", (rs.StrLit("k"), value))))


def test_a_temporary_moves_into_its_last_use() -> None:
    stmts = [let("value", call("f")), set_item("d", into("value"))]
    assert text(settle(stmts, {"value"})) == 'let value = f()?;\nd.set_item("k", value)?;'


def test_a_moved_temporary_stays_as_is() -> None:
    stmts = [let("value", call("f")), rs.ExprStmt(call("g", p("value")))]
    assert settle(stmts, {"value"}) == stmts


def test_names_that_are_not_temporaries_are_left_alone() -> None:
    stmts = [let("x", call("f")), set_item("d", rs.Ref(p("x")))]
    assert settle(stmts, set()) == stmts


def test_a_temporary_used_last_by_reference_gets_a_block() -> None:
    stmts = [
        let("value", call("f")),
        set_item("d", into("value")),
        set_item("e", into("value")),
    ]
    # Only the last use moves it: no block needed.
    assert text(settle(stmts, {"value"})) == (
        'let value = f()?;\nd.set_item("k", &value)?;\ne.set_item("k", value)?;'
    )
    stmts = [let("value", call("f")), set_item("d", rs.Ref(p("value")))]
    out = settle(stmts, {"value"})
    assert len(out) == 1 and isinstance(out[0], rs.ExprStmt)
    assert isinstance(out[0].expr, rs.BlockExpr)


def test_a_conditional_use_does_not_move() -> None:
    use = rs.If(p("c"), block(set_item("d", into("value"))))
    stmts = [let("value", call("f")), use]
    out = settle(stmts, {"value"})
    assert text(out).startswith("{\n    let value = f()?;\n    if c {")
    # The right side of `&&` may not run either.
    cond = rs.Binary("&&", p("c"), rs.Try(rs.Call(p("g"), (into("value"),))))
    out = settle([let("value", call("f")), rs.If(cond, block(CALL))], {"value"})
    assert isinstance(out[0], rs.ExprStmt)


def test_a_python_local_takes_the_block_as_its_value() -> None:
    stmts = [
        let("item", call("f")),
        let("x", rs.Try(rs.MethodCall(p("o"), "get_item", (rs.Ref(p("item")),)))),
    ]
    out = settle(stmts, {"item"})
    assert text(out) == "let x = {\n    let item = f()?;\n    o.get_item(&item)?\n};"


def test_code_that_leaves_keeps_its_temporaries() -> None:
    stmts = [let("value", call("f")), rs.Return(rs.Call(p("g"), (rs.Ref(p("value")),)))]
    assert settle(stmts, {"value"}) == stmts


def test_receivers_borrow_and_into_methods_move() -> None:
    borrowed = [let("value", call("f")), rs.ExprStmt(rs.MethodCall(p("value"), "len"))]
    assert isinstance(settle(borrowed, {"value"})[0], rs.ExprStmt)
    moved = [let("value", call("f")), rs.Return(rs.MethodCall(p("value"), "into_any"))]
    assert settle(moved, {"value"}) == moved
