"""Lowering: translate analysed Python AST into the Rust AST.

Planned modules:
    context.py   per-module / per-function lowering state (scopes, temp names, labels)
    module.py    module body -> `#[pymodule]` init function and item list
    function.py  `def` / `lambda` -> Rust functions and closures
    stmt.py      statements
    expr.py      expressions
    builtins.py  special handling for builtins such as print, len, range, isinstance
"""
