"""Rewrite constructs that have a simpler equivalent before the analyses run."""

from __future__ import annotations

import ast


class _LocalAnnotations(ast.NodeTransformer):
    """Annotated assignments in function bodies.

    The annotation of a local variable is never evaluated (PEP 526, PEP 649),
    so `x: int = v` is just `x = v`, and `x: int` only makes `x` local, which
    `symtable` already accounts for.
    """

    def __init__(self) -> None:
        self.depth = 0

    def visit_FunctionDef(self, node: ast.FunctionDef) -> ast.AST:
        self.depth += 1
        self.generic_visit(node)
        self.depth -= 1
        return node

    def visit_AnnAssign(self, node: ast.AnnAssign) -> ast.AST:
        if self.depth == 0:
            return node
        if node.value is not None:
            return ast.copy_location(ast.Assign([node.target], node.value), node)
        if isinstance(node.target, ast.Name):
            return ast.copy_location(ast.Pass(), node)
        # `obj.attr: int` still evaluates `obj`; left to the subset check.
        return node


def desugar(module: ast.Module) -> None:
    """Rewrite `module` in place."""
    _LocalAnnotations().visit(module)
