from __future__ import annotations

import ast
import textwrap

import pytest

from steelix.frontend.bindings import analyze_locals, analyze_module_bindings
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.parse import parse_source
from steelix.frontend.resolve import Direct, Lookup, resolve_global
from steelix.frontend.scope import analyze_scopes
from steelix.lower.docstrings import clean_doc


def parse(source: str) -> tuple[ast.Module, str]:
    source = textwrap.dedent(source)
    return parse_source(source, "m").tree, source


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
    assert resolve_global(b, "f", call_arity=1) == Direct(fn)
    assert resolve_global(b, "f", call_arity=2) is Lookup.GLOBAL  # TypeError at run time
    assert resolve_global(b, "f", call_arity=1, module_index=0) is Lookup.GLOBAL  # before def
    assert resolve_global(b, "f", call_arity=1, module_index=2) == Direct(fn)
    assert resolve_global(b, "LIMIT") is Lookup.GLOBAL
    assert resolve_global(b, "len") is Lookup.BUILTIN
    assert resolve_global(b, "__name__") is Lookup.GLOBAL
    assert resolve_global(b, "undefined_name") is Lookup.GLOBAL


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
    scope = analyze_scopes(tree, src, "m.py").of(fn)
    return analyze_locals(fn, scope), fn


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


# --- call graph --------------------------------------------------------------


def test_call_graph_cycles_and_globals() -> None:
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
        """
    )
    b = analyze_module_bindings(tree)
    graph = build_call_graph(tree, b, analyze_scopes(tree, source, "m.py"))
    names = lambda fns: {f.name for f in fns}  # noqa: E731
    assert names(graph.recursive) == {"ping", "pong", "selfish"}
    assert names(graph.needs_globals) == {"leaf", "middle"}


# --- docstrings --------------------------------------------------------------


def test_clean_doc_matches_cpython() -> None:
    source = 'def f():\n    """  First\n\tTabbed\n      deeper\n\n    """\n'
    namespace: dict[str, object] = {}
    exec(compile(source, "m", "exec"), namespace)
    raw = ast.get_docstring(ast.parse(source).body[0], clean=False)  # type: ignore[arg-type]
    assert raw is not None
    assert clean_doc(raw) == namespace["f"].__doc__  # type: ignore[attr-defined]
