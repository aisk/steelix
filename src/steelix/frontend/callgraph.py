"""Call graph between module level functions that call each other directly.

Two facts come out of it:

* which functions are part of a cycle of direct Rust calls and need a
  `RecursionGuard` (calls through Python objects are already guarded by
  CPython), and
* which functions need the module globals passed in, either because they look
  a name up dynamically or because a function they call directly does.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import ModuleBindings
from steelix.frontend.resolve import Direct, Lookup, resolve_global
from steelix.frontend.scope import FunctionScope, ModuleScopes


@dataclass
class CallGraph:
    edges: dict[ast.FunctionDef, list[ast.FunctionDef]]
    recursive: set[ast.FunctionDef]
    needs_globals: set[ast.FunctionDef]


def build_call_graph(
    module: ast.Module, bindings: ModuleBindings, scopes: ModuleScopes
) -> CallGraph:
    functions = [stmt for stmt in module.body if isinstance(stmt, ast.FunctionDef)]
    edges: dict[ast.FunctionDef, list[ast.FunctionDef]] = {}
    needs_globals: set[ast.FunctionDef] = set()

    for fn in functions:
        scope = scopes.of(fn)
        callees: list[ast.FunctionDef] = []
        callee_names: set[int] = set()
        for node in _walk_body(fn):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                name = node.func.id
                if scope.is_local(name):
                    continue
                target = resolve_global(bindings, name, call_arity=len(node.args))
                if isinstance(target, Direct):
                    callee_names.add(id(node.func))
                    if target.fn not in callees:
                        callees.append(target.fn)
        for node in _walk_body(fn):
            if _uses_globals(node, scope, bindings) and id(node) not in callee_names:
                needs_globals.add(fn)
        edges[fn] = callees

    # Needing globals propagates from callees to callers.
    changed = True
    while changed:
        changed = False
        for fn, callees in edges.items():
            if fn not in needs_globals and any(c in needs_globals for c in callees):
                needs_globals.add(fn)
                changed = True

    return CallGraph(edges, _in_cycles(edges), needs_globals)


def _uses_globals(node: ast.AST, scope: FunctionScope, bindings: ModuleBindings) -> bool:
    """Whether lowering `node` refers to the module globals."""
    match node:
        case ast.Name(name, ast.Load()):
            return not scope.is_local(name) and (
                resolve_global(bindings, name) is Lookup.GLOBAL or name == "globals"
            )
        case ast.Name(name):
            # Stores and deletes of names declared `global`.
            return not scope.is_local(name)
        case ast.Import() | ast.ImportFrom():
            # `__import__` receives the importing module's globals.
            return True
    return False


def _walk_body(fn: ast.FunctionDef):
    for stmt in fn.body:
        yield from ast.walk(stmt)


def _in_cycles(edges: dict[ast.FunctionDef, list[ast.FunctionDef]]) -> set[ast.FunctionDef]:
    """Nodes on a cycle: members of a non-trivial SCC or with a self loop (Tarjan)."""
    index: dict[ast.FunctionDef, int] = {}
    low: dict[ast.FunctionDef, int] = {}
    stack: list[ast.FunctionDef] = []
    on_stack: set[ast.FunctionDef] = set()
    result: set[ast.FunctionDef] = set()

    def connect(v: ast.FunctionDef) -> None:
        index[v] = low[v] = len(index)
        stack.append(v)
        on_stack.add(v)
        for w in edges.get(v, []):
            if w not in index:
                connect(w)
                low[v] = min(low[v], low[w])
            elif w in on_stack:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            component = []
            while True:
                w = stack.pop()
                on_stack.discard(w)
                component.append(w)
                if w is v:
                    break
            if len(component) > 1 or v in edges.get(v, []):
                result.update(component)

    for v in edges:
        if v not in index:
            connect(v)
    return result
