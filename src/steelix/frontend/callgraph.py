"""Call graph of direct Rust calls between functions.

Two facts come out of it:

* which module level functions are part of a cycle of direct Rust calls and
  need a `RecursionGuard` (calls through Python objects are already guarded by
  CPython; nested functions are only ever called through Python), and
* which functions, nested ones included, need the module globals passed in:
  because they look a name up dynamically, create a function object, read a
  default for a direct call, or directly call a function that needs them.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import ModuleBindings
from steelix.frontend.resolve import Direct, Lookup, resolve_call, resolve_global
from steelix.frontend.scope import FunctionNode, FunctionScope, ModuleScopes, scope_body, walk_scope


@dataclass
class CallGraph:
    edges: dict[FunctionNode, list[ast.FunctionDef]]
    recursive: set[FunctionNode]
    needs_globals: set[FunctionNode]


def build_call_graph(
    module: ast.Module, bindings: ModuleBindings, scopes: ModuleScopes
) -> CallGraph:
    edges: dict[FunctionNode, list[ast.FunctionDef]] = {}
    needs_globals: set[FunctionNode] = set()

    for fn, scope in scopes.functions.items():
        callees: list[ast.FunctionDef] = []
        callee_names: set[int] = set()
        for node in _walk_body(fn):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                name = node.func.id
                if not scope.is_global(name):
                    continue
                target = resolve_call(bindings, name, node)
                if isinstance(target, Direct):
                    callee_names.add(id(node.func))
                    if target.fn not in callees:
                        callees.append(target.fn)
                    if any(slot.kind == "default" for slot in target.slots):
                        needs_globals.add(fn)
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
            return scope.is_global(name) and (
                resolve_global(bindings, name) is Lookup.GLOBAL or name == "globals"
            )
        case ast.Name(name):
            # Stores and deletes of names declared `global`.
            return scope.is_global(name)
        case ast.Import() | ast.ImportFrom():
            # `__import__` receives the importing module's globals.
            return True
        case ast.FunctionDef() | ast.Lambda():
            # A function object holds the module globals.
            return True
    return False


def _walk_body(fn: FunctionNode):
    for node in scope_body(fn):
        yield from walk_scope(node)


def _in_cycles(edges: dict[FunctionNode, list[ast.FunctionDef]]) -> set[FunctionNode]:
    """Nodes on a cycle: members of a non-trivial SCC or with a self loop (Tarjan)."""
    index: dict[FunctionNode, int] = {}
    low: dict[FunctionNode, int] = {}
    stack: list[FunctionNode] = []
    on_stack: set[FunctionNode] = set()
    result: set[FunctionNode] = set()

    def connect(v: FunctionNode) -> None:
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
