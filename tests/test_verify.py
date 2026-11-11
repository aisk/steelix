"""The generated-code verifier: each check on hand-built Rust, and every case
and corpus script passing it."""

from __future__ import annotations

from pathlib import Path

import pytest
from conftest import CASES_DIR, CORPUS_DIR

from steelix.frontend.parse import parse_file
from steelix.lower.module import lower_module
from steelix.rustast import nodes as rs
from steelix.rustast.verify import check

USES = [rs.Use("pyo3::prelude::*"), rs.Use("pyo3::types::PyList"), rs.Use("steelix_runtime as rt")]
PY = rs.Param("py", "Python<'py>")


def p(text: str) -> rs.Path:
    return rs.Path(text)


def problems(*stmts: rs.Stmt, params: tuple[rs.Param, ...] = ()) -> list[str]:
    fn = rs.Fn("f", (PY, *params), "PyResult<()>", rs.Block(stmts))
    return check(rs.File([*USES, fn]))


def test_clean_function_passes() -> None:
    assert not problems(
        rs.Let("x", init=rs.Call(p("rt::int"), (p("py"), rs.IntLit(1)))),
        rs.ExprStmt(rs.MethodCall(p("x"), "is_truthy")),
    )


def test_local_out_of_scope() -> None:
    # `a && { let rhs = ..; .. } && rhs.lt(d)`
    block = rs.BlockExpr(rs.Block((rs.Let("rhs", init=p("py")),), rs.BoolLit(True)))
    cond = rs.Binary("&&", block, rs.MethodCall(p("rhs"), "is_truthy"))
    [problem] = problems(rs.If(cond, rs.Block()))
    assert "`rhs` is not in scope" in problem


def test_unknown_path_root() -> None:
    [problem] = problems(rs.ExprStmt(rs.Call(p("PyDict::new"), (p("py"),))))
    assert "`PyDict` is not in scope" in problem


def test_nested_fn_does_not_see_locals() -> None:
    inner = rs.Fn("g", (PY,), "PyResult<()>", rs.Block((), p("x")))
    [problem] = problems(rs.Let("x", init=p("py")), rs.ItemStmt(inner))
    assert "`x` is not in scope" in problem


def test_nested_fn_item_scope() -> None:
    """An item declared in a loop body is not visible in the loop header."""
    item = rs.ItemStmt(rs.Fn("lambda", (PY,), "PyResult<()>", rs.Block()))
    loop = rs.For("x", rs.Call(p("lambda"), (p("py"),)), rs.Block((item,)))
    [problem] = problems(loop)
    assert "`lambda` is not in scope" in problem
    assert not problems(item, rs.ExprStmt(rs.Call(p("lambda"), (p("py"),))))


def test_tokens_expression_parts_are_checked() -> None:
    macro = rs.MacroCall("rt::call", (p("py"), rs.Tokens(("key = ", p("missing")))))
    [problem] = problems(rs.ExprStmt(macro))
    assert "`missing`" in problem


def test_unlabeled_jump_out_of_labeled_block() -> None:
    inner = rs.LabeledBlock("'for_else", rs.Block((rs.Continue(),)))
    [problem] = problems(rs.Loop(rs.Block((inner,))))
    assert "E0695" in problem
    labeled = rs.LabeledBlock("'for_else", rs.Block((rs.Continue("'outer"),)))
    assert not problems(rs.Loop(rs.Block((labeled,)), "'outer"))


def test_jump_labels() -> None:
    assert "outside of a loop" in problems(rs.Break())[0]
    assert "no enclosing label" in problems(rs.Loop(rs.Block((rs.Break("'x"),))))[0]
    block = rs.LabeledBlock("'b", rs.Block((rs.Loop(rs.Block((rs.Continue("'b"),))),)))
    assert "names a block" in problems(block)[0]
    assert not problems(rs.LabeledBlock("'b", rs.Block((rs.Break("'b"),))))


def test_closure_sees_locals_but_not_labels() -> None:
    """A closure (the body of a `try`) reads the enclosing locals, but a
    jump can not leave it: `break` to an outer loop is a `Flow`."""
    body = rs.Block((rs.ExprStmt(rs.MethodCall(p("x"), "is_truthy")), rs.Break("'outer")))
    closure = rs.Closure((), body, "PyResult<()>")
    loop = rs.Loop(rs.Block((rs.ExprStmt(rs.Call(closure, ())),)), "'outer")
    [problem] = problems(rs.Let("x", init=p("py")), loop)
    assert "no enclosing label `'outer`" in problem


def test_closure_parameter_is_in_scope() -> None:
    closure = rs.Closure(("file",), rs.Block((), rs.MethodCall(p("file"), "is_truthy")))
    assert not problems(
        rs.ExprStmt(rs.MethodCall(p("frame"), "with", (p("py"), closure))),
        params=(rs.Param("frame", "&rt::Frame<'_, 'py>"),),
    )
    [problem] = problems(
        rs.ExprStmt(rs.Call(rs.Closure(("file",), rs.Block()), (p("py"),))),
        rs.ExprStmt(rs.MethodCall(p("file"), "is_truthy")),
    )
    assert "`file` is not in scope" in problem


def test_match_arm_bindings() -> None:
    """`Err(err) if guard => body` binds `err` in the guard and the body
    of its arm only; `Ok`, `Err` and `rt::Flow::..` bind nothing."""
    guard = rs.MethodCall(p("err"), "is_instance_of::<PyValueError>", (p("py"),))
    arms = (
        rs.Arm(
            "Ok(rt::Flow::Return(value))", rs.Block((rs.Return(rs.Call(p("Ok"), (p("value"),))),))
        ),
        rs.Arm("Ok(_)", rs.Block()),
        rs.Arm("Err(err)", rs.Block((rs.ExprStmt(rs.MethodCall(p("err"), "is_truthy")),)), guard),
        rs.Arm("Err(other)", rs.Block((rs.ExprStmt(rs.MethodCall(p("value"), "is_truthy")),))),
    )
    [problem] = problems(rs.ExprStmt(rs.Match(p("py"), arms)))
    assert "`value` is not in scope" in problem


def test_if_let_binds_in_then_block() -> None:
    then = rs.Block((rs.Return(rs.Call(p("Ok"), (p("value"),))),))
    stmt = rs.If(p("py"), then, pattern="rt::Flow::Return(value)")
    assert not problems(stmt)
    [problem] = problems(stmt, rs.ExprStmt(rs.MethodCall(p("value"), "is_truthy")))
    assert "`value` is not in scope" in problem


@pytest.mark.parametrize(
    ("receiver", "method", "bad"),
    [
        (rs.Try(rs.Call(p("PyList::new"), (p("py"), rs.Array(())))), "get_item", True),
        (rs.Try(rs.Call(p("PyList::new"), (p("py"), rs.Array(())))), "contains", False),
        (rs.Try(rs.MethodCall(p("py"), "try_iter")), "eq", True),
        (p("kwargs"), "get_item", True),
        (p("kwargs"), "set_item", False),
        (p("args"), "len", False),
        (rs.MethodCall(p("args"), "as_any"), "get_item", False),
        (p("plain"), "get_item", False),
    ],
)
def test_typed_receivers(receiver: rs.Expr, method: str, bad: bool) -> None:
    params = (
        rs.Param("args", "&Bound<'py, PyTuple>"),
        rs.Param("kwargs", "&Bound<'py, PyDict>"),
        rs.Param("plain", "&Bound<'py, PyAny>"),
    )
    found = problems(rs.ExprStmt(rs.MethodCall(receiver, method, (rs.IntLit(0),))), params=params)
    assert bool(found) == bad, found


def test_typed_locals() -> None:
    new = rs.Try(rs.Call(p("PyList::new"), (p("py"), rs.Array(()))))
    get = rs.ExprStmt(rs.MethodCall(p("xs"), "get_item", (rs.IntLit(0),)))
    assert problems(rs.Let("xs", init=new), get)
    assert not problems(rs.Let("xs", init=rs.MethodCall(new, "into_any")), get)
    assert problems(rs.Let("xs", ty="Bound<'py, PyList>"), get)
    assert not problems(rs.Let("xs", ty="Bound<'py, PyAny>"), get)
    either = rs.IfExpr(p("flag"), rs.Block((), new), rs.Block((), new))
    assert problems(rs.Let("xs", init=either), get)


SOURCES = sorted(CASES_DIR.glob("*.py")) + sorted(CORPUS_DIR.glob("*.py"))


@pytest.mark.parametrize("source", SOURCES, ids=lambda path: path.stem)
def test_generated_code_verifies(source: Path) -> None:
    assert check(lower_module(parse_file(source)).file) == []


def exec_python(*args: rs.Expr) -> rs.Stmt:
    return rs.ExprStmt(rs.Try(rs.MacroCall("rt::exec_python", args)))


M = rs.Param("m", "&rt::Module<'py>")


def test_exec_python_passes() -> None:
    line = rs.Tokens(("line = 3",))
    source = rs.RawStrLit("\nclass A:\n    pass\n")
    future = rs.Tokens(("future_annotations",))
    assert not problems(exec_python(p("m"), line, source), params=(M,))
    assert not problems(exec_python(p("m"), line, future, source), params=(M,))


def test_exec_python_source_must_compile() -> None:
    line = rs.Tokens(("line = 3",))
    [problem] = problems(exec_python(p("m"), line, rs.RawStrLit("\nclass A\n")), params=(M,))
    assert "does not compile" in problem
    [problem] = problems(exec_python(p("m"), line, rs.RawStrLit("x = 1\n")), params=(M,))
    assert "must start on a new line" in problem


def test_exec_python_shape() -> None:
    source = rs.RawStrLit("\nx = 1\n")
    [problem] = problems(exec_python(p("m"), rs.Tokens(("line = 0",)), source), params=(M,))
    assert "malformed" in problem
    [problem] = problems(exec_python(p("m"), source), params=(M,))
    assert "malformed" in problem
    [problem] = problems(exec_python(p("module"), rs.Tokens(("line = 1",)), source), params=(M,))
    assert "`module` is not in scope" in problem


def test_class_mod_items() -> None:
    """A class's `mod` sees the items of the file through `use super::*`
    and its own items, but its items are not visible outside of it."""
    helper = rs.Fn("helper", (PY,), "PyResult<()>", rs.Block())
    call_helper = rs.ExprStmt(rs.Try(rs.Call(p("helper"), (p("py"),))))
    method = rs.Fn("method", (PY,), "PyResult<()>", rs.Block((call_helper,)))
    call_method = rs.ExprStmt(rs.Try(rs.Call(p("method"), (p("py"),))))
    body = rs.Fn("body", (PY,), "PyResult<()>", rs.Block((call_method,)))
    mod = rs.Mod("point", (body, method))
    assert not check(rs.File([*USES, helper, mod]))
    by_path = rs.ExprStmt(rs.Try(rs.Call(p("point::body"), (p("py"),))))
    user = rs.Fn("user", (PY,), "PyResult<()>", rs.Block((by_path,)))
    assert not check(rs.File([*USES, helper, mod, user]))
    bare = rs.Fn("user", (PY,), "PyResult<()>", rs.Block((call_method,)))
    [problem] = check(rs.File([*USES, helper, mod, bare]))
    assert "`method` is not in scope" in problem
