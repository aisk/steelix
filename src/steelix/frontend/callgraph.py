"""Call graph of direct Rust calls between functions.

It tells which module level functions are part of a cycle of direct Rust
calls and need a `RecursionGuard` (calls through Python objects are already
guarded by CPython; nested functions are only ever called through Python).
"""

from __future__ import annotations

import ast
from dataclasses import dataclass

from steelix.frontend.bindings import ModuleBindings
from steelix.frontend.resolve import Direct, resolve_call
from steelix.frontend.scope import FunctionNode, ModuleScopes, scope_body, walk_scope


@dataclass
class CallGraph:
    edges: dict[FunctionNode, list[ast.FunctionDef]]
    recursive: set[FunctionNode]


def build_call_graph(
    module: ast.Module, bindings: ModuleBindings, scopes: ModuleScopes
) -> CallGraph:
    edges: dict[FunctionNode, list[ast.FunctionDef]] = {}
    for fn, scope in scopes.functions.items():
        callees: list[ast.FunctionDef] = []
        for node in _walk_body(fn):
            if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                name = node.func.id
                if not scope.is_global(name):
                    continue
                target = resolve_call(bindings, name, node)
                if isinstance(target, Direct) and target.fn not in callees:
                    callees.append(target.fn)
        edges[fn] = callees
    return CallGraph(edges, _in_cycles(edges))


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
