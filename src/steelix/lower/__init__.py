"""Lowering: translate analysed Python AST into the Rust AST.

Modules:
    context.py   per-module / per-function lowering state (scopes, temp names, labels)
    module.py    module body -> `exec_module(m)` and one Rust fn per `def`
    stmt.py      statements, assignment targets, loops, imports
    expr.py      expressions, evaluation order temporaries, builtin special cases
    names.py     name hygiene
"""
