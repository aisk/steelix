from __future__ import annotations

import textwrap

import pytest

from steelix.diagnostics import UnsupportedError
from steelix.emit.crate import render_module
from steelix.frontend.parse import parse_source
from steelix.lower.names import Renamer, rust_ident


def lower(source: str, *, fallback: bool = True) -> str:
    return render_module(parse_source(textwrap.dedent(source), "m"), fallback=fallback)


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
        (
            "x = (i for i in y for j in i)\n",
            "generator expression with several for clauses",
            1,
            5,
        ),
        ("x: int = 1\n", "annotated assignment at module level", 1, 1),
        ("def f(a):\n    a.b: int\n", "annotated assignment", 2, 5),
        ("from os import *\n", "star import", 1, 1),
        ("async def f():\n    yield 1\n", "asynchronous generator", 1, 1),
        ("async def f(xs):\n    return [await x for x in xs]\n", "await in a comprehension", 2, 13),
        ("def f():\n    return lambda: (yield)\n", "yield in a lambda", 2, 21),
        ("@types.coroutine\ndef f():\n    yield\n", "types.coroutine generator", 2, 1),
        (
            "async def f():\n    try:\n        pass\n    except await g():\n        pass\n",
            "await in an except clause",
            4,
            12,
        ),
        (
            "async def f():\n    def g(x=await h()):\n        pass\n",
            "await in a decorator or default",
            2,
            13,
        ),
        ("match x:\n    case [a] | [b]:\n        pass\n", "OR pattern with captures", 2, 10),
        (
            "match x:\n    case 1 | _:\n        pass\n",
            "OR pattern with an irrefutable alternative",
            2,
            10,
        ),
        ("def f():\n    class A:\n        pass\n", "class definition in a function", 2, 5),
        ("class A[T]:\n    pass\n", "type parameters", 1, 1),
        (
            "class A:\n    if x:\n        y: int\n",
            "annotation in a compound statement of a class body",
            3,
            9,
        ),
        ("try:\n    pass\nexcept* E:\n    pass\n", "try/except*", 1, 1),
        (
            "def f():\n    try:\n        g()\n    except E as e:\n        pass\n    return e\n",
            "except-as name used outside its handler",
            4,
            5,
        ),
        ("x = t'{a}'\n", "t-string", 1, 5),
        (
            "print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)\n",
            "call with more than 12 arguments",
            1,
            1,
        ),
        (
            "print(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, sep=s)\n",
            "call with more than 12 arguments",
            1,
            1,
        ),
        (
            "f(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, k=v)\n",
            "call with more than 12 arguments",
            1,
            1,
        ),
        (
            "def f():\n    raise ValueError(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)\n",
            "call with more than 12 arguments",
            2,
            11,
        ),
    ],
)
def test_unsupported_constructs_report_location(
    source: str, feature: str, line: int, col: int
) -> None:
    with pytest.raises(UnsupportedError) as info:
        lower(source, fallback=False)
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
    assert 'm.global("f")?.call1((1,))?' in out
    assert "f(m, &rt::int(py, 1))?" in out


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
    assert 'm.set_global("x", m.globals())?;' in out


def test_pure_operands_need_no_temporaries() -> None:
    out = lower(
        """
        def f(x, xs, d, i):
            d[i] = x
            return x in xs
        """
    )
    assert "d.set_item(i, x)?;" in out
    assert "Ok(rt::bool(m.py(), xs.contains(x)?))" in out
    assert "let " not in out.split("pub fn exec_module")[0]


def test_value_is_evaluated_before_the_assignment_target() -> None:
    out = lower(
        """
        def f(g, h):
            g()[h()] = g()
        """
    )
    assert "let value = g.call0()?;" in out
    # Moved into its last use, which drops it with the statement.
    assert "g.call0()?.set_item(h.call0()?, value)?;" in out


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
    assert 'o.method1("m", (x, 1))?;' in out
    assert 'o.attr("m")?.call1((g.call0()?,))?;' in out


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
    assert 'a.set_attr("x", rt::isub(&a.attr("x")?, v)?)?;' in out
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
    assert "let len = xs.len()?;\n        if 0 < len && len <= 3 && !xs.contains(a)? {" in out


def test_builtin_special_cases_respect_rebinding() -> None:
    out = lower(
        """
        def f(x):
            return len(x)
        """
    )
    assert "Ok(rt::int(m.py(), x.len()?))" in out
    out = lower(
        """
        def f(x):
            return len(x)
        len = repr
        """
    )
    assert 'm.global("len")?.call1((x,))' in out


def test_for_loop_variable_scoped_to_the_body() -> None:
    out = lower(
        """
        def f(xs):
            for x in xs:
                print(x)
        """
    )
    assert "for x in xs.try_iter()? {\n            let x = x?;" in out


def test_imports_bind_names() -> None:
    out = lower(
        """
        import os.path
        import a.b.c as d
        from x.y import z as w, v
        """
    )
    assert 'm.set_global("os", rt::import(m, "os.path")?)?;' in out
    assert 'm.set_global("d", rt::import_module(m, "a.b.c")?)?;' in out
    assert 'let y = rt::import_names(m, "x.y", &["z", "v"])?;' in out
    assert 'm.set_global("w", rt::import_from(&y, "z")?)?;' in out


# --- the module context ------------------------------------------------------


def test_py_is_bound_once_or_spelled_in_place() -> None:
    out = lower(
        """
        def once():
            return None

        def twice(x):
            return [x, 1, "a"]
        """
    )
    assert "pub fn once<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {" in out
    assert "Ok(rt::none(m.py()))" in out
    assert 'let py = m.py();\n    rt::frame!(m, "twice", 5, |_| {' in out
    assert "Ok(PyList::new(py, [x, &rt::int(py, 1)" in out


def test_local_named_m_keeps_its_name() -> None:
    out = lower(
        """
        import re

        def first_word(text):
            m = re.match("[a-z]+", text)
            return m.group(0)
        """
    )
    assert "module: &rt::Module<'py>," in out
    assert 'let m = module.global("re")?' in out
    assert 'm.method1("group", (0,))' in out


def test_literal_defaults_are_passed_in_place() -> None:
    out = lower(
        """
        def f(a, b="x", c=None, d=[]):
            return a

        f(1)
        """
    )
    args = ["&rt::int(py, 1)", '&rt::str(py, "x")', "&rt::none(py)", '&m.default("f", "d")?']
    assert ",\n            ".join(args) in out


def test_assigned_defaults_are_read_from_the_function() -> None:
    out = lower(
        """
        def f(a, b="x"):
            return a

        f.__defaults__ = ("y",)
        f(1)
        """
    )
    assert 'f(m, &rt::int(py, 1), &m.default("f", "b")?)' in out


def test_python_kept_def_is_called_through_globals() -> None:
    out = lower(
        """
        async def gen(n):
            yield n

        def use():
            return gen(1)
        """
    )
    assert "pub fn gen" not in out
    assert 'm.global("gen")?.call1((1,))' in out
    assert "// m.py:2, kept as Python: asynchronous generator at 2:1\n" in out
    assert "line = 2,\n" in out


def test_each_python_def_or_class_has_its_own_block() -> None:
    """Consecutive simple statements kept as Python share a block; a `def`
    or `class` gets its own, with its own location and reason."""
    out = lower(
        """
        x = 1
        # about the class
        class A[T]:
            # inside the class
            pass
        ys = (i for i in x for j in x)  # squares
        zs = t"{ys}"
        async def gen():
            yield 1
        print(x)
        """
    )
    assert out.count("rt::exec_python!(") == 3
    comments = "// about the class\n        // m.py:4, kept as Python: type parameters at 4:1\n"
    assert comments in out
    assert "// inside the class" not in out
    assert '"\nclass A[T]:\n    # inside the class\n    pass\n"' in out
    simple = 'ys = (i for i in x for j in x)  # squares\nzs = t"{ys}"\n'
    assert f'"\n{simple}"' in out
    reasons = "generator expression with several for clauses at 7:6, t-string at 8:6"
    assert f"// m.py:7, kept as Python: {reasons}\n" in out
    assert "// m.py:9, kept as Python: asynchronous generator at 9:1\n" in out


def test_python_statement_after_semicolon() -> None:
    out = lower("x = 1; y = t'ab'; print(y)\n")
    assert 'm.set_global("x", 1)?;' in out
    assert "line = 1,\n            r#\"\ny = t'ab'\n\"#" in out
    assert 'rt::builtins::print(m.py(), (m.global("y")?,))?;' in out


def test_raw_string_delimiter_avoids_the_source() -> None:
    out = lower("""x = (t'', '"#', '"##')\n""")
    assert 'r###"\n' in out


def test_unsupported_found_while_lowering_is_kept_as_python() -> None:
    source = """
        def names():
            return locals()
        """
    out = lower(source)
    assert "// m.py:2, kept as Python: locals() in a function at 3:12" in out
    with pytest.raises(UnsupportedError) as info:
        lower(source, fallback=False)
    assert info.value.feature == "locals() in a function"


def test_future_annotations_reach_python_code() -> None:
    out = lower(
        """
        from __future__ import annotations
        class A[T]:
            x: int
        """
    )
    assert "line = 3,\n            future_annotations,\n" in out


def kept_lines(source: str) -> list[int]:
    """The first lines of the statements kept as Python."""
    out = lower(source)
    return [
        int(line.split("line = ")[1].rstrip(",")) for line in out.splitlines() if "line = " in line
    ]


def test_module_level_if_keeps_only_unsupported_children() -> None:
    out = lower(
        """
        import sys
        if len(sys.argv) > 1:
            x = 1
            async def f():
                yield x
        else:
            ys = [t'']
            print(ys)
        """
    )
    assert 'if m.global("sys")?.attr("argv")?.len()? > 1 {' in out
    assert 'm.set_global("x", 1)?;' in out
    assert 'rt::builtins::print(m.py(), (m.global("ys")?,))?;' in out
    # Indented as in the file.
    assert 'r#"\n    async def f():\n        yield x\n"#' in out
    assert "r#\"\n    ys = [t'']\n\"#" in out


def test_unsupported_header_keeps_the_whole_statement() -> None:
    assert kept_lines(
        """
        if t'':
            print(1)
        for x in (i for i in range(3) for j in ()):
            print(x)
        """
    ) == [2, 4]


def test_loop_with_jump_in_kept_statement_is_kept_whole() -> None:
    # `break` alone does not compile, so a loop with a kept statement
    # containing one is kept whole.
    assert kept_lines(
        """
        for i in range(3):
            print(i)
            try:
                break
            except* ValueError:
                pass
        """
    ) == [2]
    # Jumps in a kept loop belong to it; native jumps stay native.
    assert kept_lines(
        """
        for i in range(3):
            if i:
                ys = [t'{i}']
                break
            for j in range(i):
                try:
                    continue
                except* ValueError:
                    pass
            else:
                print(i)
        """
    ) == [4, 6]


def test_jump_in_kept_loop_else_keeps_the_outer_loop() -> None:
    assert kept_lines(
        """
        while True:
            for j in range(3):
                pass
            else:
                try:
                    break
                except* ValueError:
                    pass
        """
    ) == [2]


def test_unsupported_found_while_lowering_in_a_block_keeps_the_child() -> None:
    out = lower(
        """
        if True:
            print(1)
            f = lambda: locals()
        """
    )
    assert 'm.global("print")?.call1((1,))?;' in out
    assert "// m.py:4, kept as Python: locals() in a function at 4:17" in out
    assert 'r#"\n    f = lambda: locals()\n"#' in out


def test_exec_without_namespace_gets_the_module_globals() -> None:
    out = lower(
        """
        exec("y = 1")
        def f():
            return eval("y")
        """
    )
    call = 'm.global("exec")?.call1(("y = 1", m.globals()))?;'
    assert call in out
    assert "// m.py:3, kept as Python: eval() without a namespace in a function at 4:12" in out


TRACEBACK_SOURCE = """
def total(values):
    out = 0
    for v in values:
        out += v
    return out

def check(x):
    if x < 0:
        raise ValueError("negative")
    return x
"""


def test_no_line_markers_by_default() -> None:
    """Native frames report their `def` line (design §9): no `frame.line(n)`,
    and `|_|` where the body does not use the frame."""
    out = lower(TRACEBACK_SOURCE)
    assert "frame.line(" not in out
    assert 'rt::frame!(m, "total", 2, |_| {' in out
    assert 'rt::frame!(m, "check", 8, |frame| {' in out
    assert "frame.raise(PyValueError::new_err(" in out
    assert 'rt::frame!(m, "<module>", 1, |_| {' in out


def test_traceback_lines_marks_statements() -> None:
    parsed = parse_source(textwrap.dedent(TRACEBACK_SOURCE), "m")
    out = render_module(parsed, traceback_lines=True)
    assert 'rt::frame!(m, "total", 2, |frame| {' in out
    for line in (4, 5, 9, 10):
        assert f"frame.line({line});" in out
    # Without the markers the code is the same.
    default = render_module(parse_source(textwrap.dedent(TRACEBACK_SOURCE), "m"))
    stripped = "\n".join(line for line in out.splitlines() if "frame.line(" not in line)
    assert stripped.replace("|frame|", "|_|") == default.replace("|frame|", "|_|").rstrip("\n")


def test_single_expression_frame_keeps_braces() -> None:
    out = lower("def double(x):\n    return x * 2\n")
    assert 'rt::frame!(m, "double", 1, |_| { x.mul(2) })' in out


def test_coroutines_are_awaited_in_place_only_by_await() -> None:
    out = lower(
        """
        import asyncio
        async def f(x):
            await asyncio.sleep(0)
            return await f(x)
        def g():
            yield 1
        async def use():
            coro = f(1)
            yield_from = g()
            return await f(2), await coro
        def gen():
            yield from g()
            return list(g())
        """
    )
    assert "pub async fn f<'py>(" in out
    # Bound first: `m.global("asyncio")?` would live across the await.
    assert 'let sleep = m.global("asyncio")?.method1("sleep", (0,))?;\n' in out
    assert "rt::await_(sleep).await?;" in out
    assert "rt::delegate(Box::pin(f(m, x))).await" in out
    assert 'let coro = m.global("f")?.call1((1,))?;' in out
    assert 'm.global("g")?.call0()?' in out
    assert "rt::delegate(f(m, &rt::int(py, 2))).await?" in out
    assert "rt::await_(&coro).await?" in out
    assert "rt::delegate(g(m)).await?;" in out
    assert "rt::def!(m, async f(x))?;" in out
    assert "rt::def!(m, gen g())?;" in out
