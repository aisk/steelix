from __future__ import annotations

import ast
import textwrap

import pytest

from steelix.frontend.bindings import analyze_locals, analyze_module_bindings
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.parse import parse_source
from steelix.frontend.resolve import (
    Direct,
    Lookup,
    Slot,
    bind_static,
    resolve_call,
    resolve_global,
)
from steelix.frontend.scope import analyze_scopes
from steelix.lower.docstrings import clean_doc


def parse(source: str) -> tuple[ast.Module, str]:
    source = textwrap.dedent(source)
    return parse_source(source, "m").tree, source


def call(source: str) -> ast.Call:
    expr = ast.parse(source, mode="eval").body
    assert isinstance(expr, ast.Call)
    return expr


def function(tree: ast.Module, name: str) -> ast.FunctionDef:
    return next(s for s in tree.body if isinstance(s, ast.FunctionDef) and s.name == name)


# --- module bindings ---------------------------------------------------------


def test_binding_counts_and_direct_functions() -> None:
    tree, _ = parse(
        """
        def f(): pass
        def g(): pass
        g = 1
        X = 1
        if X:
            X = 2
        def h():
            global Y
            Y = 3
        """
    )
    b = analyze_module_bindings(tree)
    assert b.counts["f"] == 1
    assert b.counts["g"] == 2
    assert b.counts["X"] == 2
    assert b.counts["Y"] == 1
    assert b.direct_function("f") is function(tree, "f")
    assert b.direct_function("g") is None  # rebound
    assert b.direct_function("X") is None  # not a def
    assert not b.dynamic


@pytest.mark.parametrize(
    "source",
    ["from os import *", "x = globals()", "def f():\n    return vars()"],
)
def test_dynamic_modules_disable_direct_calls(source: str) -> None:
    tree, _ = parse("def f(): pass\n" + source)
    b = analyze_module_bindings(tree)
    assert b.dynamic
    assert b.direct_function("f") is None


@pytest.mark.parametrize(
    "source",
    [
        'import sys\nsetattr(sys.modules[__name__], "f", len)',
        'import sys\ndelattr(sys.modules["__main__"], "f")',
        'from sys import modules\nsetattr(modules[__name__], "f", len)',
        'import sys\nsetattr(sys.modules.get(__name__), "f", len)',
        'import __main__\nsetattr(__main__, "f", len)',
        "import __main__ as me\nme.f = len",
        "import sys\nme = sys.modules[__name__]\nalso = me\ndef g():\n    also.f = len",
        'import sys\nsys.modules[__name__].__dict__["f"] = len',
        'import sys\nvars(sys.modules[__name__])["f"] = len',
        'import types\nsetattr(types, "f", len)',
        'exec("f = len")',
        'import sys\ntry:\n    setattr(sys.modules[__name__], "f", len)\nexcept OSError:\n    pass',
        "def g(src):\n    return eval(src)",
    ],
)
def test_patching_the_own_module_disables_direct_calls(source: str) -> None:
    tree, _ = parse("def f(): pass\n" + source)
    b = analyze_module_bindings(tree)
    assert b.patches_itself
    assert b.direct_function("f") is None


@pytest.mark.parametrize(
    "source",
    [
        'import sys\nsetattr(sys.modules["json"], "dumps", len)',
        'import sys\nsetattr(sys, "g", len)',
        "import sys\nprint(sys.modules[__name__].f)",
        'exec("x = 1", {})',
        'exec("x = 1", globals=dict(a=1))',
        "x = vars(object())",
        "def g(obj):\n    return vars(obj)",
    ],
)
def test_other_namespaces_keep_direct_calls(source: str) -> None:
    tree, _ = parse("def f(): pass\n" + source)
    b = analyze_module_bindings(tree)
    assert not b.patches_itself
    assert not b.dynamic
    assert b.direct_function("f") is not None


def test_parameter_named_globals_is_not_namespace_access() -> None:
    tree, _ = parse("def f(globals):\n    return globals\n")
    assert not analyze_module_bindings(tree).dynamic


def test_resolve_global() -> None:
    tree, _ = parse(
        """
        f(1)
        def f(a): pass
        f(1)
        LIMIT = 3
        """
    )
    b = analyze_module_bindings(tree)
    fn = function(tree, "f")
    one = call("f(1)")
    assert resolve_call(b, "f", one) == Direct(fn, (Slot("arg", (0,)),))
    assert resolve_call(b, "f", call("f(1, 2)")) is Lookup.GLOBAL  # TypeError at run time
    assert resolve_call(b, "f", one, module_index=0) is Lookup.GLOBAL  # before def
    assert resolve_call(b, "f", one, module_index=2) == Direct(fn, (Slot("arg", (0,)),))
    assert resolve_global(b, "LIMIT") is Lookup.GLOBAL
    assert resolve_global(b, "len") is Lookup.BUILTIN
    assert resolve_global(b, "__name__") is Lookup.GLOBAL
    assert resolve_global(b, "undefined_name") is Lookup.GLOBAL


def test_bind_static() -> None:
    (fn,) = parse("def f(a, /, b=1, *rest, c, d=2, **kw): pass")[0].body
    assert isinstance(fn, ast.FunctionDef)
    slots = bind_static(fn.args, call("f(1, 2, 3, 4, d=5, c=6, e=7)"))
    assert slots == (
        Slot("arg", (0,)),
        Slot("arg", (1,)),
        Slot("varargs", (2, 3)),
        Slot("keyword", (1,)),
        Slot("keyword", (0,)),
        Slot("varkw", (2,)),
    )
    assert bind_static(fn.args, call("f(1, c=2)")) == (
        Slot("arg", (0,)),
        Slot("default"),
        Slot("varargs"),
        Slot("keyword", (0,)),
        Slot("default"),
        Slot("varkw"),
    )
    # Left to the function object: unpacking and calls that raise TypeError.
    assert bind_static(fn.args, call("f(*xs, c=1)")) is None
    assert bind_static(fn.args, call("f(1)")) is None
    assert bind_static(fn.args, call("f(a=1, c=2)")) is None


# --- scopes ------------------------------------------------------------------


def test_scopes() -> None:
    tree, source = parse(
        """
        def f(a, b):
            c = a + G
            return len(c)
        """
    )
    scope = analyze_scopes(tree, source, "m.py").of(function(tree, "f"))
    assert scope.params == ("a", "b")
    assert scope.locals == {"a", "b", "c"}
    assert scope.globals == {"G", "len"}


# --- definite assignment -----------------------------------------------------


def locals_of(source: str, name: str = "f"):
    tree, src = parse(source)
    fn = function(tree, name)
    scopes = analyze_scopes(tree, src, "m.py")
    return analyze_locals(scopes.of(fn), scopes), fn


def test_straight_line_locals_are_declared_in_place() -> None:
    info, fn = locals_of(
        """
        def f(n):
            a = 1
            b = a
            a = b
            return a
        """
    )
    assert info.optional == set()
    assert set(info.declared_at) == {"a", "b"}
    assert info.declared_at["a"] is fn.body[0]
    assert info.mutable == {"a"}
    assert info.declared_at_top == []


def test_assigned_in_both_branches_is_definite() -> None:
    info, _ = locals_of(
        """
        def f(n):
            if n:
                x = 1
            else:
                x = 2
            return x
        """
    )
    assert info.optional == set()
    assert info.declared_at_top == ["x"]
    assert info.mutable == set()


def test_conditionally_assigned_local_is_optional() -> None:
    info, _ = locals_of(
        """
        def f(n):
            if n:
                x = 1
            return x
        """
    )
    assert info.optional == {"x"}
    assert info.declared_at_top == ["x"]


def test_branch_that_returns_does_not_spoil_definiteness() -> None:
    info, _ = locals_of(
        """
        def f(n):
            if n:
                return 0
            else:
                x = 1
            return x
        """
    )
    assert info.optional == set()


def test_assignment_in_loop_is_mutable_and_not_definite_after_it() -> None:
    info, _ = locals_of(
        """
        def f(n):
            while n:
                x = n
            return x
        """
    )
    assert info.optional == {"x"}


def test_loop_reassignment_needs_mut() -> None:
    info, _ = locals_of(
        """
        def f(n):
            x = 0
            while n:
                if n:
                    y = 1
                else:
                    y = 2
                x = y
            return x
        """
    )
    assert info.mutable == {"x", "y"}
    assert info.optional == set()


def test_read_before_assignment_is_optional() -> None:
    info, _ = locals_of(
        """
        def f():
            x = x + 1
            return x
        """
    )
    assert info.optional == {"x"}
    assert info.declared_at == {}


def test_unreachable_code_and_reassigned_params() -> None:
    info, fn = locals_of(
        """
        def f(n):
            n = n + 1
            while True:
                return n
            print(n)
        """
    )
    assert info.assigned_params == {"n"}
    assert fn.body[2] in info.unreachable
    assert not info.end_reachable


def test_loop_variables_only_read_in_the_body_are_loop_scoped() -> None:
    info, fn = locals_of(
        """
        def f(xs):
            for x in xs:
                print(x)
            for k, (a, b) in xs:
                print(k, a, b)
            for y in xs:
                pass
            print(y)
            for z in xs:
                z = 1
        """
    )
    assert set(info.loop_scoped) == {"x", "k", "a", "b"}
    assert info.loop_scoped["x"] is fn.body[0]
    assert "y" in info.optional and "y" not in info.loop_scoped
    assert "z" not in info.loop_scoped
    assert "x" not in info.declared_at_top


def test_break_continue_and_loop_else() -> None:
    info, fn = locals_of(
        """
        def f(xs):
            for x in xs:
                if x:
                    found = x
                    break
                continue
                print("unreachable")
            else:
                found = None
            return found
        """
    )
    loop = fn.body[0]
    assert loop.body[2] in info.unreachable
    assert info.optional == set()  # assigned before `break` and in `else`


def test_while_true_with_break_is_left_through_the_break() -> None:
    info, fn = locals_of(
        """
        def f():
            while True:
                x = 1
                break
            return x
        """
    )
    assert info.optional == set()
    assert info.end_reachable is False
    assert info.mutable == set()  # the body runs at most once


def test_deleted_locals_are_optional() -> None:
    info, fn = locals_of(
        """
        def f(a):
            x = 1
            del x
            del a
            if a:
                y = 2
            del y
        """
    )
    assert {"x", "a", "y"} <= info.optional
    deletes = [t for s in fn.body if isinstance(s, ast.Delete) for t in s.targets]
    assert {t.id for t in info.assigned_deletes} == {"x", "a"}  # type: ignore[attr-defined]
    assert deletes[2] not in info.assigned_deletes


def test_tuple_targets_and_imports_declare_in_place() -> None:
    info, fn = locals_of(
        """
        def f(p):
            a, *b = p
            import os.path
            c, a = p
            return a, b, c, os
        """
    )
    assert info.declared_at["a"] is fn.body[0]
    assert info.declared_at["b"] is fn.body[0]
    assert info.declared_at["os"] is fn.body[1]
    assert info.declared_at_top == ["c"]  # `c, a` can not be a single `let`
    assert info.mutable == {"a"}


def test_global_statement_names_are_not_locals() -> None:
    tree, source = parse(
        """
        def f():
            global counter
            counter = 1
            del counter
        """
    )
    scope = analyze_scopes(tree, source, "m.py").of(function(tree, "f"))
    assert scope.locals == set()
    assert "counter" in scope.globals


# --- call graph --------------------------------------------------------------


def test_call_graph_cycles() -> None:
    tree, source = parse(
        """
        SCALE = 2
        def leaf(x):
            return x * SCALE
        def middle(x):
            return leaf(x)
        def ping(n):
            return pong(n)
        def pong(n):
            return ping(n)
        def selfish(n):
            return selfish(n)
        def pure(x):
            return len(x)
        def importer():
            import os
        def writer():
            global SCALE
            SCALE = 3
        """
    )
    b = analyze_module_bindings(tree)
    graph = build_call_graph(tree, b, analyze_scopes(tree, source, "m.py"))
    names = lambda fns: {f.name for f in fns}  # noqa: E731
    assert names(graph.recursive) == {"ping", "pong", "selfish"}


# --- docstrings --------------------------------------------------------------


def test_clean_doc_matches_cpython() -> None:
    source = 'def f():\n    """  First\n\tTabbed\n      deeper\n\n    """\n'
    namespace: dict[str, object] = {}
    exec(compile(source, "m", "exec"), namespace)
    raw = ast.get_docstring(ast.parse(source).body[0], clean=False)  # type: ignore[arg-type]
    assert raw is not None
    assert clean_doc(raw) == namespace["f"].__doc__  # type: ignore[attr-defined]


# --- closures ----------------------------------------------------------------


def test_cells_and_frozen_captures() -> None:
    tree, source = parse(
        """
        def f(a, b):
            c = 1
            late = 0

            def g():
                nonlocal b
                b += 1
                return a + c + late

            def h():
                return early

            early = 2
            late = 3
            return g, h
        """
    )
    scopes = analyze_scopes(tree, source, "m.py")
    outer = scopes.of(function(tree, "f"))
    inner = outer.children[0]
    assert inner.qualname == "f.<locals>.g"
    assert set(inner.frees) == {"a", "b", "c", "late"}
    assert outer.cells == {"a", "b", "c", "late", "early"}
    info = analyze_locals(outer, scopes)
    # `b` is rebound by `g`, `late` after the capture, and `early` is
    # captured before it is assigned: those need real cells.
    assert info.frozen == {"a", "c"}
