from __future__ import annotations

import ast
import textwrap

import pytest

from steelix.frontend.booleans import bool_locals
from steelix.frontend.callgraph import build_call_graph
from steelix.frontend.containers import container_locals
from steelix.frontend.locals import analyze_locals
from steelix.frontend.module_bindings import analyze_module_bindings
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


def test_captures_only_read_in_their_case_are_case_scoped() -> None:
    info, _ = locals_of(
        """
        def f(v):
            match v:
                case [x, (a, b)] if a:
                    print(x, b)
                case {"k": k, **rest}:
                    pass
                case [y, *_] if y:
                    pass
                case {"y": y}:
                    print(y)
                case z:
                    z = 1
            match v:
                case [late]:
                    pass
            print(late)
        """
    )
    assert info.case_scoped == {"x", "a", "b", "k", "rest", "y"}
    assert "late" in info.optional


def test_failed_guard_leaves_captures_assigned() -> None:
    info, _ = locals_of(
        """
        def f(v):
            match v:
                case x if x:
                    pass
                case _:
                    pass
            return x
        """
    )
    assert "x" not in info.optional


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


# --- classes -------------------------------------------------------------------


def test_mangle_matches_cpython() -> None:
    from steelix.frontend.desugar import mangle

    assert mangle("Point", "__x") == "_Point__x"
    assert mangle("_Point", "__x") == "_Point__x"
    assert mangle("Point", "__x__") == "__x__"
    assert mangle("Point", "_x") == "_x"
    assert mangle("___", "__x") == "__x"
    assert mangle("Point", "__a.b") == "__a.b"


def test_private_names_are_mangled_in_classes() -> None:
    from steelix.frontend.desugar import binding_name, desugar

    tree, _ = parse(
        """
        __x = 0
        class A:
            __y = __x
            def __m(self, __p):
                return self.__y, f(__k=1)
            class __B:
                __z = 1
        """
    )
    desugar(tree)
    cls = tree.body[1]
    assert isinstance(cls, ast.ClassDef)
    assign, method, inner = cls.body
    assert ast.unparse(assign) == "_A__y = _A__x"
    assert isinstance(method, ast.FunctionDef) and isinstance(inner, ast.ClassDef)
    assert method.name == "__m" and binding_name(method) == "_A__m"
    assert method.args.args[1].arg == "_A__p"
    assert ast.unparse(method.body[0]) == "return (self._A__y, f(__k=1))"
    assert inner.name == "__B" and binding_name(inner) == "_A__B"
    assert ast.unparse(inner.body[0]) == "_B__z = 1"
    assert ast.unparse(tree.body[0]) == "__x = 0"


def test_class_is_split_by_method() -> None:
    from steelix.diagnostics import SourceLocation
    from steelix.frontend.fallback import split
    from steelix.frontend.subset import Issue

    tree, _ = parse(
        """
        class A:
            x = 1
            def f(self):
                return [i for i in self]
            def g(self):
                return 1
            if x:
                y = (i for i in "ab")
                z = 2
        """
    )
    cls = tree.body[0]
    assert isinstance(cls, ast.ClassDef)

    def kept(*lines: int) -> list[int]:
        issues = [Issue("comprehension", SourceLocation("m", line, 8)) for line in lines]
        return sorted(stmt.lineno for stmt in split(cls, issues))

    assert kept(5) == [4]
    assert kept(5, 9) == [4, 9]
    # In the header: the whole class.
    assert kept(2) == [2]


# --- comprehensions --------------------------------------------------------------


def test_comprehension_variables_are_not_function_variables() -> None:
    tree, source = parse(
        """
        def f(xs):
            ys = [x * 2 for x in xs if x]
            g = (y for y in ys)
            return [lambda: x for x in ys], g
        """
    )
    scopes = analyze_scopes(tree, source, "m.py")
    outer = scopes.of(function(tree, "f"))
    genexpr, lam = outer.children
    assert genexpr.name == "<genexpr>"
    assert genexpr.qualname == "f.<locals>.<genexpr>"
    assert genexpr.params == ()
    assert genexpr.locals == {"y"}
    # The lambda captures the comprehension's `x`, not a cell of `f`.
    assert lam.frees == ("x",)
    assert lam.comprehension_frees == {"x"}
    assert outer.cells == set()
    info = analyze_locals(outer, scopes)
    assert set(info.declared_at) == {"ys", "g"}


def test_walrus_in_a_comprehension_assigns_the_function_local() -> None:
    info, fn = locals_of(
        """
        def f(xs):
            ys = [y for x in xs if (y := x)]
            return y
        """
    )
    # The comprehension may not run its condition at all.
    assert info.optional == {"y"}


def test_walrus_in_a_condition_is_definite_after_it() -> None:
    info, _ = locals_of(
        """
        def f(xs):
            if (n := len(xs)) > 1:
                pass
            return n
        """
    )
    assert info.optional == set()


@pytest.mark.parametrize(
    ("source", "feature"),
    [
        ("g = (x for x in a for y in x)", "generator expression with several for clauses"),
        (
            "g = (lambda: x for x in a)",
            "generator expression variable captured by a nested function",
        ),
        ("ys = [y for x in a for y in y]", "comprehension variable used before it is bound"),
    ],
)
def test_unsupported_comprehensions(source: str, feature: str) -> None:
    from steelix.frontend.subset import find_unsupported

    issues = [i for found in find_unsupported(ast.parse(source), "m.py").values() for i in found]
    assert [i.feature for i in issues] == [feature]


# --- closure regions -------------------------------------------------------------


def try_stmt(source: str) -> ast.Try:
    tree, _ = parse(source)
    stmt = tree.body[0]
    assert isinstance(stmt, ast.Try)
    return stmt


@pytest.mark.parametrize(
    ("source", "parts"),
    [
        # A single expression is matched on directly: no closure.
        ("try:\n    x = f()\nexcept E:\n    pass\n", []),
        ("try:\n    return f()\nexcept E:\n    pass\n", []),
        ("try:\n    f()\nexcept E:\n    pass\n", []),
        # Anything else runs in a closure.
        ("try:\n    x.y = f()\nexcept E:\n    pass\n", ["body"]),
        ("try:\n    f()\n    g()\nexcept E:\n    pass\n", ["body"]),
        # `finally` alone needs the body; with handlers also what they run.
        ("try:\n    f()\nfinally:\n    pass\n", ["body"]),
        ("try:\n    f()\nexcept E:\n    pass\nfinally:\n    pass\n", ["handled"]),
        ("try:\n    f()\n    g()\nexcept E:\n    pass\nfinally:\n    g()\n", ["handled", "body"]),
    ],
)
def test_try_regions(source: str, parts: list[str]) -> None:
    from steelix.frontend.regions import try_regions

    assert [region.part for region in try_regions(try_stmt(source))] == parts


def test_expression_body() -> None:
    from steelix.frontend.regions import expression_body

    stmt = try_stmt("try:\n    x = f()\nexcept E:\n    pass\n")
    assert ast.unparse(expression_body(stmt)) == "f()"  # type: ignore[arg-type]
    assert expression_body(try_stmt("try:\n    x, y = f()\nexcept E:\n    pass\n")) is None
    assert expression_body(try_stmt("try:\n    return\nexcept E:\n    pass\n")) is None
    assert expression_body(try_stmt("try:\n    f()\nfinally:\n    pass\n")) is None


def test_region_nodes() -> None:
    from steelix.frontend.regions import Region, region_node_ids, regions_in

    tree, _ = parse(
        """
        with a() as x, b() as y:
            f(x, y)
        try:
            g()
            h()
        except E:
            k()
        else:
            m()
        """
    )
    with_stmt, try_ = tree.body
    assert isinstance(with_stmt, ast.With) and isinstance(try_, ast.Try)
    assert list(regions_in(tree.body)) == [
        Region(with_stmt, 0),
        Region(with_stmt, 1),
        Region(try_, "body"),
    ]
    # The first item's closure gets `x`, then evaluates the later items.
    first = [ast.unparse(node) for node in Region(with_stmt, 0).nodes()]
    assert first == ["x", "b()", "y", "f(x, y)"]
    second = [ast.unparse(node) for node in Region(with_stmt, 1).nodes()]
    assert second == ["y", "f(x, y)"]
    handled = Region(try_, "handled").nodes()
    assert handled[:2] == try_.body and handled[2] is try_.handlers[0]
    assert handled[3:] == try_.orelse
    ids = region_node_ids(Region(try_, "body"))
    assert id(try_.body[0]) in ids and id(try_.handlers[0]) not in ids
    assert id(try_.orelse[0]) not in ids


# --- locals in handlers, `with` and regions ------------------------------------------


def test_handler_names_are_scoped_to_their_handler() -> None:
    info, _ = locals_of(
        """
        def f(g):
            try:
                g()
            except ValueError as e:
                print(e)
            except KeyError as e:
                print(e)
            try:
                g()
            except ValueError as kept:
                pass
            return kept
        """
    )
    # `kept` is read after its handler (where the implicit `del` unbinds it).
    assert info.handler_scoped == {"e"}
    assert "kept" in info.optional


def test_with_targets_are_closure_parameters() -> None:
    info, fn = locals_of(
        """
        def f(cm):
            with cm as w, cm as v:
                print(w, v)
            with cm as later:
                pass
            return later
        """
    )
    assert info.with_params == {"w", "v"}
    first = fn.body[0]
    assert info.scoped["w"].node is first and info.scoped["w"].part == 0
    assert info.scoped["v"].part == 1
    # Read after the `with`: it lives outside the closure.
    assert "later" not in info.with_params


def test_locals_of_a_region_are_declared_inside_it() -> None:
    info, fn = locals_of(
        """
        def f(g):
            try:
                y = g()
                if y:
                    s = 1
                else:
                    s = 2
                print(y, s)
            finally:
                pass
            try:
                out = g()
                out = out + 1
            finally:
                pass
            return out
        """
    )
    region = info.scoped["y"]
    assert region.node is fn.body[0] and region.part == "body"
    assert info.scoped["s"] == region
    # `s` is first assigned in a branch: it is declared at the closure's start.
    assert info.region_top[region] == ["s"]
    assert info.declared_at["y"] is region.node.body[0]
    # `out` is used after the `try`: declared outside, before the closure.
    assert "out" not in info.scoped
    assert "out" in info.declared_at_top


def test_try_that_can_not_raise() -> None:
    info, fn = locals_of(
        """
        def f(g):
            x = g()
            try:
                a = 1
            except Exception:
                print("never")
            try:
                c = (a, x)
            except Exception:
                pass
            try:
                pass
            except Exception:
                pass
            try:
                a = 2
                b = a
            except Exception:
                b = 0
            try:
                d = g()
            except Exception:
                d = None
            return a, b, c, d
        """
    )
    first, tuple_, pass_, several, call = fn.body[1:6]
    assert info.infallible_tries == {first, tuple_, pass_}
    assert isinstance(first, ast.Try) and isinstance(call, ast.Try)
    assert first.handlers[0].body[0] in info.unreachable
    # Declared where the body assigns them, as if there were no `try`.
    assert info.declared_at["a"] is first.body[0]
    assert info.declared_at["c"] is tuple_.body[0]  # type: ignore[attr-defined]
    # Only a single statement counts: a longer body still gets a closure.
    assert several not in info.infallible_tries
    # Every handler assigns `d` too: `let d = match .. { .. };`.
    assert info.declared_at["d"] is call


def test_walrus_declares_where_it_always_runs() -> None:
    info, fn = locals_of(
        """
        def f(g):
            print(x := g(), x)
            if (y := g()) and (z := g()):
                print(y, z)
            return x, y
        """
    )
    walrus = fn.body[0].value.args[0]  # type: ignore[attr-defined]
    assert info.declared_at["x"] is walrus
    assert isinstance(info.declared_at["y"], ast.NamedExpr)
    # Only assigned when `y` is true.
    assert "z" not in info.declared_at


# --- bool locals -------------------------------------------------------------


def bools_of(source: str, name: str = "f") -> frozenset[str]:
    tree, src = parse(source)
    fn = function(tree, name)
    scopes = analyze_scopes(tree, src, "m.py")
    scope = scopes.of(fn)
    bindings = analyze_module_bindings(tree)
    return bool_locals(fn, scope, analyze_locals(scope, scopes), bindings)


def test_locals_assigned_only_bools_are_bool_locals() -> None:
    found = bools_of(
        """
        def f(x, xs):
            a = True
            b = not x
            c = x is None
            d = x is not None
            e = x in xs
            g = x not in xs
            h = isinstance(x, int)
            i = bool(x)
            j = a and (b or c)
            k = d if x else e
            l = k
            if x:
                a = False
            return a, b, c, d, e, g, h, i, j, k, l
        """
    )
    assert found == set("abcdeghijkl")


def test_values_python_does_not_guarantee_to_be_bool() -> None:
    found = bools_of(
        """
        def f(x, y, xs, isinstance):
            lt = x < y
            eq = x == y
            chained = x in xs in xs
            either = x or True
            shadowed = isinstance(x, int)
            mixed = False
            if x:
                mixed = 1
            maybe = None
            maybe = True
            return lt, eq, chained, either, shadowed, mixed, maybe
        """
    )
    assert found == frozenset()


def test_shadowed_bool_builtin_is_not_a_bool() -> None:
    tree, src = parse(
        """
        def bool(x):
            return x

        def f(x):
            b = bool(x)
            return b
        """
    )
    fn = function(tree, "f")
    scopes = analyze_scopes(tree, src, "m.py")
    scope = scopes.of(fn)
    info = analyze_locals(scope, scopes)
    assert bool_locals(fn, scope, info, analyze_module_bindings(tree)) == frozenset()


def test_bool_locals_must_be_plain_locals() -> None:
    found = bools_of(
        """
        def f(flag, xs):
            loop = False
            for loop in xs:
                pass
            deleted = True
            del deleted
            captured = True
            def g():
                return captured
            walrus = False
            if (walrus := flag is None):
                pass
            unbound_on_some_path = None
            if flag:
                maybe = True
            print(maybe)
            augmented = True
            augmented &= flag is None
            tupled = True
            tupled, other = True, False
            flag = True
            try:
                attempt = flag in xs
            except TypeError:
                attempt = False
            plain = flag is None
            return loop, captured, walrus, augmented, tupled, other, attempt, plain
        """
    )
    assert found == {"plain"}


# --- container locals --------------------------------------------------------


def containers_of(source: str, name: str = "f") -> dict[str, str]:
    tree, src = parse(source)
    fn = function(tree, name)
    scopes = analyze_scopes(tree, src, "m.py")
    scope = scopes.of(fn)
    return container_locals(fn, scope, analyze_locals(scope, scopes))


def test_locals_assigned_only_displays_are_container_locals() -> None:
    found = containers_of(
        """
        def f(x, xs):
            a = []
            b = [x, *xs]
            c = [i for i in xs]
            d = {}
            e = {x: 1, **xs}
            g = {k: v for k, v in xs}
            h = {x}
            i = {y for y in xs}
            j = a
            k = j
            if x:
                a = [1]
                k = b
            return a, b, c, d, e, g, h, i, j, k
        """
    )
    lists, dicts, sets = "abcjk", "deg", "hi"
    assert found == (
        dict.fromkeys(lists, "PyList")
        | dict.fromkeys(dicts, "PyDict")
        | dict.fromkeys(sets, "PySet")
    )


def test_values_python_does_not_guarantee_to_be_a_container() -> None:
    found = containers_of(
        """
        def f(x, xs):
            called = list()
            called2 = dict(xs)
            empty_set = set()
            tupled = ()
            either = [] or xs
            chosen = [] if x else [1]
            copied = xs
            kinds = []
            kinds = {}
            other = []
            if x:
                other = None
            alias = other
            cycle = cycle2 = []
            return called, called2, empty_set, tupled, either, chosen, copied, kinds, alias
        """
    )
    assert found == {}


def test_container_locals_must_be_plain_locals() -> None:
    found = containers_of(
        """
        def f(flag, xs):
            loop = []
            for loop in xs:
                pass
            deleted = []
            del deleted
            captured = []
            def g():
                return captured
            walrus = []
            if (walrus := xs):
                pass
            if flag:
                maybe = []
            print(maybe)
            augmented = []
            augmented += xs
            tupled = []
            tupled, other = [], {}
            flag = []
            try:
                attempt = []
            except TypeError:
                attempt = []
            with xs as managed:
                managed = []
            plain = []
            return loop, captured, walrus, augmented, tupled, other, attempt, managed, plain
        """
    )
    assert found == {"plain": "PyList"}


def test_subscripted_containers_stay_objects() -> None:
    found = containers_of(
        """
        def f(i, v):
            read = [1]
            stored = [1]
            removed = [1]
            updated = [1]
            looked_up = {}
            filled = {}
            emptied = {i: v}
            counted = {}
            inner = [[1]]
            plain = [1]
            stored[i] = v
            del removed[i]
            updated[i] += v
            filled[i] = v
            del emptied[i]
            counted[i] += 1
            plain.append([x[0] for x in inner])
            return read[i], looked_up[i], [plain for plain[0] in inner]
        """
    )
    assert found == {"filled": "PyDict", "emptied": "PyDict", "inner": "PyList"}
